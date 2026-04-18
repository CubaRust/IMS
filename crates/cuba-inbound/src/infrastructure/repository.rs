//! inbound repo
//!
//! 创建单据走 `sys.fn_next_doc_no('INBOUND')` 生成单号,头+行在同一 DB 事务。

use async_trait::async_trait;
use sqlx::{postgres::PgRow, PgPool, Postgres, Row};

use cuba_shared::{audit::AuditContext, error::AppError, types::DocStatus};

use crate::application::{
    CreateInboundCommand, InboundHeadView, InboundLineView, QueryInbounds,
};
use crate::domain::default_target_status;

#[async_trait]
pub trait InboundRepository: Send + Sync {
    async fn create(
        &self,
        ctx: &AuditContext,
        cmd: &CreateInboundCommand,
    ) -> Result<InboundHeadView, AppError>;

    async fn get(&self, tenant_id: i64, id: i64) -> Result<InboundHeadView, AppError>;
    async fn list(&self, q: &QueryInbounds) -> Result<Vec<InboundHeadView>, AppError>;
    async fn update_status(
        &self,
        tenant_id: i64,
        id: i64,
        status: DocStatus,
    ) -> Result<(), AppError>;
}

pub struct PgInboundRepository {
    pool: PgPool,
}

impl PgInboundRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InboundRepository for PgInboundRepository {
    async fn create(
        &self,
        ctx: &AuditContext,
        cmd: &CreateInboundCommand,
    ) -> Result<InboundHeadView, AppError> {
        let mut tx = self.pool.begin().await?;

        let inbound_no: String =
            sqlx::query_scalar("select sys.fn_next_doc_no('INBOUND')")
                .fetch_one(&mut *tx)
                .await?;

        let tenant_id = cmd.tenant_id.unwrap_or(ctx.tenant_id);

        let id: i64 = sqlx::query_scalar(
            r#"
            insert into wms.wms_inbound_h
                (tenant_id, inbound_no, inbound_type, supplier_id,
                 source_object_type, source_object_id,
                 wh_id, loc_id, inbound_date, operator_id,
                 doc_status, remark)
            values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'DRAFT',$11)
            returning id
            "#,
        )
        .bind(tenant_id)
        .bind(&inbound_no)
        .bind(&cmd.inbound_type)
        .bind(cmd.supplier_id)
        .bind(&cmd.source_object_type)
        .bind(cmd.source_object_id)
        .bind(cmd.wh_id)
        .bind(cmd.loc_id)
        .bind(cmd.inbound_date)
        .bind(ctx.user_id)
        .bind(&cmd.remark)
        .fetch_one(&mut *tx)
        .await?;

        let default_status = default_target_status(&cmd.inbound_type);
        for l in &cmd.lines {
            let status = l
                .stock_status
                .clone()
                .unwrap_or_else(|| default_status.to_string());
            sqlx::query(
                r#"
                insert into wms.wms_inbound_d
                    (tenant_id, inbound_id, line_no, material_id, batch_no, qty, unit, stock_status,
                     work_order_no, process_name, outsource_no, related_preissue_line_id, note)
                values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
                "#,
            )
            .bind(tenant_id)
            .bind(id)
            .bind(l.line_no)
            .bind(l.material_id)
            .bind(&l.batch_no)
            .bind(l.qty)
            .bind(&l.unit)
            .bind(&status)
            .bind(&l.work_order_no)
            .bind(&l.process_name)
            .bind(&l.outsource_no)
            .bind(l.related_preissue_line_id)
            .bind(&l.note)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.get(tenant_id, id).await
    }

    async fn get(&self, tenant_id: i64, id: i64) -> Result<InboundHeadView, AppError> {
        let head = sqlx::query(
            r#"
            select h.id, h.inbound_no, h.inbound_type, h.supplier_id,
                   s.supplier_name,
                   h.wh_id, w.wh_code,
                   h.loc_id, l.loc_code,
                   h.inbound_date, h.operator_id, h.doc_status, h.remark,
                   h.created_at, h.updated_at
              from wms.wms_inbound_h h
              left join mdm.mdm_supplier  s on s.id = h.supplier_id
              left join mdm.mdm_warehouse w on w.id = h.wh_id
              left join mdm.mdm_location  l on l.id = h.loc_id
             where h.id = $1 and h.tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found(format!("入库单 id={id} 不存在")))?;

        let lines = sqlx::query(
            r#"
            select d.id, d.line_no, d.material_id, m.material_code,
                   d.batch_no, d.qty, d.unit, d.stock_status,
                   d.work_order_no, d.process_name, d.outsource_no,
                   d.related_preissue_line_id, d.note
              from wms.wms_inbound_d d
              left join mdm.mdm_material m on m.id = d.material_id
             where d.inbound_id = $1 and d.tenant_id = $2
             order by d.line_no
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(row_to_line)
        .collect();

        Ok(row_to_head(head, lines))
    }

    async fn list(&self, q: &QueryInbounds) -> Result<Vec<InboundHeadView>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select h.id, h.inbound_no, h.inbound_type, h.supplier_id,
                   s.supplier_name,
                   h.wh_id, w.wh_code,
                   h.loc_id, l.loc_code,
                   h.inbound_date, h.operator_id, h.doc_status, h.remark,
                   h.created_at, h.updated_at
              from wms.wms_inbound_h h
              left join mdm.mdm_supplier  s on s.id = h.supplier_id
              left join mdm.mdm_warehouse w on w.id = h.wh_id
              left join mdm.mdm_location  l on l.id = h.loc_id
             where 1 = 1
            "#,
        );
        if let Some(t) = q.tenant_id {
            qb.push(" and h.tenant_id = ").push_bind(t);
        }
        if let Some(no) = &q.inbound_no {
            qb.push(" and h.inbound_no = ").push_bind(no.clone());
        }
        if let Some(t) = &q.inbound_type {
            qb.push(" and h.inbound_type = ").push_bind(t.clone());
        }
        if let Some(sid) = q.supplier_id {
            qb.push(" and h.supplier_id = ").push_bind(sid);
        }
        if let Some(s) = &q.doc_status {
            qb.push(" and h.doc_status = ").push_bind(s.clone());
        }
        if let Some(from) = q.date_from {
            qb.push(" and h.inbound_date >= ").push_bind(from);
        }
        if let Some(to) = q.date_to {
            qb.push(" and h.inbound_date <= ").push_bind(to);
        }
        qb.push(" order by h.inbound_date desc, h.id desc limit 500");
        let rows = qb.build().fetch_all(&self.pool).await?;

        // 列表不带明细,详情接口再加载
        Ok(rows.into_iter().map(|r| row_to_head(r, vec![])).collect())
    }

    async fn update_status(
        &self,
        tenant_id: i64,
        id: i64,
        status: DocStatus,
    ) -> Result<(), AppError> {
        sqlx::query(
            "update wms.wms_inbound_h set doc_status = $1 where id = $2 and tenant_id = $3",
        )
        .bind(status.as_str())
        .bind(id)
        .bind(tenant_id)
        .execute(&self.pool)
            .await?;
        Ok(())
    }
}

fn row_to_head(row: PgRow, lines: Vec<InboundLineView>) -> InboundHeadView {
    InboundHeadView {
        id: row.get("id"),
        inbound_no: row.get("inbound_no"),
        inbound_type: row.get("inbound_type"),
        supplier_id: row.get("supplier_id"),
        supplier_name: row.get("supplier_name"),
        wh_id: row.get("wh_id"),
        wh_code: row.get("wh_code"),
        loc_id: row.get("loc_id"),
        loc_code: row.get("loc_code"),
        inbound_date: row.get("inbound_date"),
        operator_id: row.get("operator_id"),
        doc_status: row.get("doc_status"),
        remark: row.get("remark"),
        lines,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_line(row: PgRow) -> InboundLineView {
    InboundLineView {
        id: row.get("id"),
        line_no: row.get("line_no"),
        material_id: row.get("material_id"),
        material_code: row.get("material_code"),
        batch_no: row.get("batch_no"),
        qty: row.get("qty"),
        unit: row.get("unit"),
        stock_status: row.get("stock_status"),
        work_order_no: row.get("work_order_no"),
        process_name: row.get("process_name"),
        outsource_no: row.get("outsource_no"),
        related_preissue_line_id: row.get("related_preissue_line_id"),
        note: row.get("note"),
    }
}
