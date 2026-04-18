//! 库存仓储:trait + PG 实现
//!
//! ## 关键实现点
//! 1. **提交事务在单个 DB 事务里**:`txn_h` → `txn_d[]` → `balance upsert[]`
//! 2. **余额 upsert** 用 `INSERT ... ON CONFLICT (material_id, wh_id, loc_id, batch_no, stock_status) DO UPDATE SET ...`
//! 3. **不做真负库存** 由 DDL CHECK 兜底(`book_qty >= 0 or stock_status = 'PREISSUE_PENDING'`),
//!    DB 抛 23514(check_violation)时转 `INV_INSUFFICIENT`
//! 4. **单据号生成** 走 `sys.fn_next_doc_no('INVENTORY_TXN')`

use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{postgres::PgRow, PgPool, Postgres, Row, Transaction};

use cuba_shared::{
    audit::AuditContext,
    error::AppError,
    pagination::{PageQuery, PageResponse},
    types::StockStatus,
};

use crate::application::{
    commands::CommitTxnResult,
    dto::{BalanceView, TxnHeadView, TxnLineView},
    queries::{QueryBalance, QueryTxns},
};
use crate::domain::{
    errors::InventoryError,
    model::{StockDelta, TxnHead, TxnLine},
};

/// 库存仓储接口
///
/// 所有写操作接受 `AuditContext`,用于记录 operator_id 和 trace_id。
#[async_trait]
pub trait InventoryRepository: Send + Sync {
    /// 提交一笔事务:写头 + 行 + 更新余额
    async fn commit_txn(
        &self,
        ctx: &AuditContext,
        head: TxnHead,
        lines: Vec<TxnLine>,
        deltas: Vec<StockDelta>,
    ) -> Result<CommitTxnResult, AppError>;

    /// 查询余额(分页)
    async fn query_balance(
        &self,
        tenant_id: i64,
        query: &QueryBalance,
        page: PageQuery,
    ) -> Result<PageResponse<BalanceView>, AppError>;

    /// 查询事务头(分页)
    async fn query_txns(
        &self,
        tenant_id: i64,
        query: &QueryTxns,
        page: PageQuery,
    ) -> Result<PageResponse<TxnHeadView>, AppError>;

    /// 查询事务行
    async fn query_txn_lines(&self, tenant_id: i64, txn_id: i64) -> Result<Vec<TxnLineView>, AppError>;
}

/// PG 实现
pub struct PgInventoryRepository {
    pool: PgPool,
}

impl PgInventoryRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InventoryRepository for PgInventoryRepository {
    async fn commit_txn(
        &self,
        ctx: &AuditContext,
        head: TxnHead,
        lines: Vec<TxnLine>,
        deltas: Vec<StockDelta>,
    ) -> Result<CommitTxnResult, AppError> {
        let mut tx = self.pool.begin().await?;

        // 1) 取单据号
        let txn_no: String = sqlx::query_scalar("select sys.fn_next_doc_no('INVENTORY_TXN')")
            .fetch_one(&mut *tx)
            .await?;

        // 2) 插事务头
        let txn_id = insert_txn_head(&mut tx, &txn_no, &head, ctx).await?;

        // 3) 插事务行
        for line in &lines {
            insert_txn_line(&mut tx, ctx.tenant_id, txn_id, line).await?;
        }

        // 4) 按 delta 更新余额
        for d in &deltas {
            upsert_balance(&mut tx, ctx.tenant_id, d).await?;
        }

        // 5) 写领域事件(同事务,保证和业务原子)
        let lines_summary = lines
            .iter()
            .map(|l| cuba_events::types::InventoryLineSummary {
                material_id: l.material_id,
                batch_no: l.batch_no.clone(),
                qty: l.qty,
                io_flag: l.io_flag.as_str().to_string(),
            })
            .collect();
        let event = cuba_events::DomainEvent::InventoryTxnCommitted {
            txn_id,
            txn_no: txn_no.clone(),
            txn_type: head.txn_type.as_str().to_string(),
            scene_code: head.scene_code.clone(),
            doc_type: head.doc_type.clone(),
            doc_no: head.doc_no.clone(),
            line_count: lines.len(),
            lines_summary,
        };
        let ev_ctx = cuba_events::WriteEventCtx::from(ctx);
        cuba_events::write_event_tx(&mut tx, &ev_ctx, &event).await?;

        tx.commit().await?;

        Ok(CommitTxnResult {
            id: txn_id,
            txn_no,
            line_count: lines.len() as u32,
        })
    }

    async fn query_balance(
        &self,
        tenant_id: i64,
        query: &QueryBalance,
        page: PageQuery,
    ) -> Result<PageResponse<BalanceView>, AppError> {
        let p = page.normalize();

        let total = count_balance(&self.pool, tenant_id, query).await.unwrap_or(0);

        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select b.id, b.material_id, m.material_code, m.material_name,
                   b.wh_id, w.wh_code, w.wh_name,
                   b.loc_id, l.loc_code, l.loc_name,
                   b.batch_no, b.stock_status,
                   b.book_qty, b.available_qty, b.occupied_qty,
                   b.bad_qty, b.scrap_qty, b.pending_qty, b.updated_at
              from wms.wms_inventory_balance b
              join mdm.mdm_material  m on m.id = b.material_id
              join mdm.mdm_warehouse w on w.id = b.wh_id
              join mdm.mdm_location  l on l.id = b.loc_id
             where b.tenant_id =
            "#,
        );
        qb.push_bind(tenant_id);
        push_balance_filters(&mut qb, query);

        qb.push(" order by b.updated_at desc ");
        qb.push(" limit ").push_bind(p.limit());
        qb.push(" offset ").push_bind(p.offset());

        let rows = qb.build().fetch_all(&self.pool).await?;
        let items = rows.into_iter().map(row_to_balance_view).collect();
        Ok(PageResponse::new(p, total, items))
    }

    async fn query_txns(
        &self,
        tenant_id: i64,
        query: &QueryTxns,
        page: PageQuery,
    ) -> Result<PageResponse<TxnHeadView>, AppError> {
        let p = page.normalize();

        let total = count_txns(&self.pool, tenant_id, query).await.unwrap_or(0);

        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select id, txn_no, txn_type, scene_code, scene_name,
                   doc_type, doc_no, source_wh_id, source_loc_id,
                   target_wh_id, target_loc_id, source_status, target_status,
                   is_exception, exception_type, operator_id, related_doc_no,
                   remark, operate_time
              from wms.wms_inventory_txn_h
             where tenant_id =
            "#,
        );
        qb.push_bind(tenant_id);
        push_txn_filters(&mut qb, query);

        qb.push(" order by operate_time desc ");
        qb.push(" limit ").push_bind(p.limit());
        qb.push(" offset ").push_bind(p.offset());

        let rows = qb.build().fetch_all(&self.pool).await?;
        let items = rows.into_iter().map(row_to_txn_head_view).collect();
        Ok(PageResponse::new(p, total, items))
    }

    async fn query_txn_lines(&self, tenant_id: i64, txn_id: i64) -> Result<Vec<TxnLineView>, AppError> {
        let rows = sqlx::query(
            r#"
            select d.id, d.txn_id, d.line_no, d.material_id, m.material_code,
                   d.batch_no, d.qty, d.unit, d.io_flag, d.stock_status,
                   d.status_change_flag, d.location_change_flag, d.item_change_flag,
                   d.recoverable_flag, d.scrap_flag, d.note
              from wms.wms_inventory_txn_d d
              join mdm.mdm_material m on m.id = d.material_id
             where d.txn_id = $1 and d.tenant_id = $2
             order by d.line_no
            "#,
        )
        .bind(txn_id)
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_txn_line_view).collect())
    }
}

// ---------------------------------------------------------------------------
// 私有辅助:insert / upsert
// ---------------------------------------------------------------------------

async fn insert_txn_head(
    tx: &mut Transaction<'_, Postgres>,
    txn_no: &str,
    head: &TxnHead,
    ctx: &AuditContext,
) -> Result<i64, AppError> {
    let (src_wh, src_loc, src_status) = head
        .source
        .as_ref()
        .map_or((None, None, None), |s| (Some(s.wh_id), Some(s.loc_id), s.status));
    let (tgt_wh, tgt_loc, tgt_status) = head
        .target
        .as_ref()
        .map_or((None, None, None), |s| (Some(s.wh_id), Some(s.loc_id), s.status));

    let id: i64 = sqlx::query_scalar(
        r#"
        insert into wms.wms_inventory_txn_h
            (tenant_id, txn_no, txn_type, scene_code, scene_name, doc_type, doc_no,
             source_object_type, source_object_id, target_object_type, target_object_id,
             source_wh_id, source_loc_id, target_wh_id, target_loc_id,
             source_status, target_status,
             is_exception, exception_type, operator_id, related_doc_no,
             snapshot_json, remark)
        values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23)
        returning id
        "#,
    )
    .bind(ctx.tenant_id)
    .bind(txn_no)
    .bind(head.txn_type.as_str())
    .bind(&head.scene_code)
    .bind(&head.scene_name)
    .bind(&head.doc_type)
    .bind(&head.doc_no)
    .bind(&head.source_object_type)
    .bind(head.source_object_id)
    .bind(&head.target_object_type)
    .bind(head.target_object_id)
    .bind(src_wh)
    .bind(src_loc)
    .bind(tgt_wh)
    .bind(tgt_loc)
    .bind(src_status.map(|s| s.as_str()))
    .bind(tgt_status.map(|s| s.as_str()))
    .bind(head.is_exception)
    .bind(&head.exception_type)
    .bind(head.operator_id.or(Some(ctx.user_id)))
    .bind(&head.related_doc_no)
    .bind(&head.snapshot_json)
    .bind(&head.remark)
    .fetch_one(&mut **tx)
    .await?;

    Ok(id)
}

async fn insert_txn_line(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    txn_id: i64,
    line: &TxnLine,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        insert into wms.wms_inventory_txn_d
            (tenant_id, txn_id, line_no, material_id, batch_no, qty, unit, io_flag,
             source_material_id, target_material_id, stock_status,
             status_change_flag, location_change_flag, item_change_flag,
             recoverable_flag, scrap_flag, note)
        values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17)
        "#,
    )
    .bind(tenant_id)
    .bind(txn_id)
    .bind(line.line_no)
    .bind(line.material_id)
    .bind(&line.batch_no)
    .bind(line.qty)
    .bind(&line.unit)
    .bind(line.io_flag.as_str())
    .bind(line.source_material_id)
    .bind(line.target_material_id)
    .bind(line.stock_status.map(|s| s.as_str()))
    .bind(line.status_change_flag)
    .bind(line.location_change_flag)
    .bind(line.item_change_flag)
    .bind(line.recoverable_flag)
    .bind(line.scrap_flag)
    .bind(&line.note)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn upsert_balance(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    d: &StockDelta,
) -> Result<(), AppError> {
    // 行锁顺序固定:(tenant_id, material_id, wh_id, loc_id, batch_no, stock_status) 升序
    // 注:ON CONFLICT 以老表的 (material_id, wh_id, loc_id, batch_no, stock_status)
    // 为唯一键 + tenant_id 当作普通列。多租户场景下,
    // 逻辑唯一性靠 (tenant_id, material_id, wh_id, loc_id, batch_no, stock_status),
    // 但 wh_id/loc_id/material_id 通过 tenant 天然隔离,因此老约束可复用。
    let res = sqlx::query(
        r#"
        insert into wms.wms_inventory_balance
            (tenant_id, material_id, wh_id, loc_id, batch_no, stock_status,
             book_qty, available_qty, occupied_qty, bad_qty, scrap_qty, pending_qty)
        values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
        on conflict (material_id, wh_id, loc_id, batch_no, stock_status)
        do update set
            book_qty      = wms_inventory_balance.book_qty      + excluded.book_qty,
            available_qty = wms_inventory_balance.available_qty + excluded.available_qty,
            occupied_qty  = wms_inventory_balance.occupied_qty  + excluded.occupied_qty,
            bad_qty       = wms_inventory_balance.bad_qty       + excluded.bad_qty,
            scrap_qty     = wms_inventory_balance.scrap_qty     + excluded.scrap_qty,
            pending_qty   = wms_inventory_balance.pending_qty   + excluded.pending_qty,
            updated_at    = now()
        "#,
    )
    .bind(tenant_id)
    .bind(d.locator.material_id)
    .bind(d.locator.wh_id)
    .bind(d.locator.loc_id)
    .bind(&d.locator.batch_no)
    .bind(d.locator.stock_status.as_str())
    .bind(d.book)
    .bind(d.available)
    .bind(d.occupied)
    .bind(d.bad)
    .bind(d.scrap)
    .bind(d.pending)
    .execute(&mut **tx)
    .await;

    match res {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(db)) if db.code().as_deref() == Some("23514") => {
            // CHECK 违反:几乎一定是 book_qty < 0 但状态不是 PREISSUE_PENDING
            Err(InventoryError::insufficient(format!(
                "库存不足:material_id={} wh={} loc={} batch={} status={}",
                d.locator.material_id,
                d.locator.wh_id,
                d.locator.loc_id,
                d.locator.batch_no,
                d.locator.stock_status
            )))
        }
        Err(e) => Err(e.into()),
    }
}

async fn count_balance(
    pool: &PgPool,
    tenant_id: i64,
    query: &QueryBalance,
) -> Result<i64, AppError> {
    let mut qb = sqlx::QueryBuilder::<Postgres>::new(
        "select count(*) from wms.wms_inventory_balance b where b.tenant_id = ",
    );
    qb.push_bind(tenant_id);
    push_balance_filters(&mut qb, query);
    let c: i64 = qb.build_query_scalar().fetch_one(pool).await?;
    Ok(c)
}

fn push_balance_filters<'a>(qb: &mut sqlx::QueryBuilder<'a, Postgres>, query: &'a QueryBalance) {
    if let Some(mid) = query.material_id {
        qb.push(" and b.material_id = ").push_bind(mid);
    }
    if let Some(wh) = query.wh_id {
        qb.push(" and b.wh_id = ").push_bind(wh);
    }
    if let Some(loc) = query.loc_id {
        qb.push(" and b.loc_id = ").push_bind(loc);
    }
    if let Some(batch) = &query.batch_no {
        qb.push(" and b.batch_no = ").push_bind(batch.clone());
    }
    if let Some(status) = query.stock_status {
        qb.push(" and b.stock_status = ").push_bind(status.as_str());
    }
    if query.only_positive {
        qb.push(" and b.book_qty > 0");
    }
}

async fn count_txns(
    pool: &PgPool,
    tenant_id: i64,
    query: &QueryTxns,
) -> Result<i64, AppError> {
    let mut qb = sqlx::QueryBuilder::<Postgres>::new(
        "select count(*) from wms.wms_inventory_txn_h where tenant_id = ",
    );
    qb.push_bind(tenant_id);
    push_txn_filters(&mut qb, query);
    let c: i64 = qb.build_query_scalar().fetch_one(pool).await?;
    Ok(c)
}

fn push_txn_filters<'a>(qb: &mut sqlx::QueryBuilder<'a, Postgres>, query: &'a QueryTxns) {
    if let Some(no) = &query.doc_no {
        qb.push(" and doc_no = ").push_bind(no.clone());
    }
    if let Some(s) = &query.scene_code {
        qb.push(" and scene_code = ").push_bind(s.clone());
    }
    if let Some(t) = &query.doc_type {
        qb.push(" and doc_type = ").push_bind(t.clone());
    }
    if let Some(from) = query.date_from {
        qb.push(" and operate_time >= ").push_bind(from);
    }
    if let Some(to) = query.date_to {
        qb.push(" and operate_time < ").push_bind(to);
    }
}

// ---------------------------------------------------------------------------
// row -> view 映射
// ---------------------------------------------------------------------------

fn row_to_balance_view(row: PgRow) -> BalanceView {
    BalanceView {
        id: row.get("id"),
        material_id: row.get("material_id"),
        material_code: row.get("material_code"),
        material_name: row.get("material_name"),
        wh_id: row.get("wh_id"),
        wh_code: row.get("wh_code"),
        wh_name: row.get("wh_name"),
        loc_id: row.get("loc_id"),
        loc_code: row.get("loc_code"),
        loc_name: row.get("loc_name"),
        batch_no: row.get("batch_no"),
        stock_status: StockStatus::try_from(row.get::<String, _>("stock_status").as_str())
            .unwrap_or(StockStatus::Qualified),
        book_qty: row.get::<Decimal, _>("book_qty"),
        available_qty: row.get::<Decimal, _>("available_qty"),
        occupied_qty: row.get::<Decimal, _>("occupied_qty"),
        bad_qty: row.get::<Decimal, _>("bad_qty"),
        scrap_qty: row.get::<Decimal, _>("scrap_qty"),
        pending_qty: row.get::<Decimal, _>("pending_qty"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_txn_head_view(row: PgRow) -> TxnHeadView {
    TxnHeadView {
        id: row.get("id"),
        txn_no: row.get("txn_no"),
        txn_type: row.get("txn_type"),
        scene_code: row.get("scene_code"),
        scene_name: row.get("scene_name"),
        doc_type: row.get("doc_type"),
        doc_no: row.get("doc_no"),
        source_wh_id: row.get("source_wh_id"),
        source_loc_id: row.get("source_loc_id"),
        target_wh_id: row.get("target_wh_id"),
        target_loc_id: row.get("target_loc_id"),
        source_status: row.get("source_status"),
        target_status: row.get("target_status"),
        is_exception: row.get("is_exception"),
        exception_type: row.get("exception_type"),
        operator_id: row.get("operator_id"),
        related_doc_no: row.get("related_doc_no"),
        remark: row.get("remark"),
        operate_time: row.get("operate_time"),
    }
}

fn row_to_txn_line_view(row: PgRow) -> TxnLineView {
    TxnLineView {
        id: row.get("id"),
        txn_id: row.get("txn_id"),
        line_no: row.get("line_no"),
        material_id: row.get("material_id"),
        material_code: row.get("material_code"),
        batch_no: row.get("batch_no"),
        qty: row.get("qty"),
        unit: row.get("unit"),
        io_flag: row.get("io_flag"),
        stock_status: row.get("stock_status"),
        status_change_flag: row.get("status_change_flag"),
        location_change_flag: row.get("location_change_flag"),
        item_change_flag: row.get("item_change_flag"),
        recoverable_flag: row.get("recoverable_flag"),
        scrap_flag: row.get("scrap_flag"),
        note: row.get("note"),
    }
}
