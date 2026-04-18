//! preissue repository

use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{postgres::PgRow, PgPool, Postgres, Row};

use cuba_shared::{audit::AuditContext, error::AppError};

use crate::application::{
    CreatePreissueCommand, PreissueHeadView, PreissueLineView, QueryPreissues,
};
use crate::domain::PreissueError;

#[async_trait]
pub trait PreissueRepository: Send + Sync {
    async fn create(
        &self,
        ctx: &AuditContext,
        cmd: &CreatePreissueCommand,
    ) -> Result<PreissueHeadView, AppError>;

    async fn get(&self, tenant_id: i64, id: i64) -> Result<PreissueHeadView, AppError>;
    async fn list(
        &self,
        tenant_id: i64,
        q: &QueryPreissues,
    ) -> Result<Vec<PreissueHeadView>, AppError>;

    /// 冲销一行
    ///
    /// 返回:`(preissue_no, new_line_status, material_id, expected_batch_no)`
    async fn apply_fill(
        &self,
        tenant_id: i64,
        line_id: i64,
        filled_now: Decimal,
    ) -> Result<(String, String, i64, String), AppError>;

    async fn update_head_status(
        &self,
        tenant_id: i64,
        id: i64,
        status: &str,
    ) -> Result<(), AppError>;
}

pub struct PgPreissueRepository {
    pool: PgPool,
}

impl PgPreissueRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PreissueRepository for PgPreissueRepository {
    async fn create(
        &self,
        ctx: &AuditContext,
        cmd: &CreatePreissueCommand,
    ) -> Result<PreissueHeadView, AppError> {
        let mut tx = self.pool.begin().await?;

        let preissue_no: String = sqlx::query_scalar("select sys.fn_next_doc_no('PREISSUE')")
            .fetch_one(&mut *tx)
            .await?;

        let exception_type = cmd
            .exception_type
            .clone()
            .unwrap_or_else(|| "PREISSUE".into());

        let id: i64 = sqlx::query_scalar(
            r#"
            insert into wms.wms_preissue_h
                (tenant_id, preissue_no, exception_type, supplier_id,
                 work_order_no, process_name, workshop_name,
                 issue_date, operator_id, reason,
                 exception_status, expected_close_date, remark)
            values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'PENDING',$11,$12)
            returning id
            "#,
        )
        .bind(ctx.tenant_id)
        .bind(&preissue_no)
        .bind(&exception_type)
        .bind(cmd.supplier_id)
        .bind(&cmd.work_order_no)
        .bind(&cmd.process_name)
        .bind(&cmd.workshop_name)
        .bind(cmd.issue_date)
        .bind(ctx.user_id)
        .bind(&cmd.reason)
        .bind(cmd.expected_close_date)
        .bind(&cmd.remark)
        .fetch_one(&mut *tx)
        .await?;

        for l in &cmd.lines {
            sqlx::query(
                r#"
                insert into wms.wms_preissue_d
                    (tenant_id, preissue_id, line_no, material_id, qty, filled_qty, unfilled_qty,
                     expected_batch_no, target_desc, line_status, note)
                values ($1,$2,$3,$4,$5,0,$5,$6,$7,'PENDING',$8)
                "#,
            )
            .bind(ctx.tenant_id)
            .bind(id)
            .bind(l.line_no)
            .bind(l.material_id)
            .bind(l.qty)
            .bind(&l.expected_batch_no)
            .bind(&l.target_desc)
            .bind(&l.note)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.get(ctx.tenant_id, id).await
    }

    async fn get(&self, tenant_id: i64, id: i64) -> Result<PreissueHeadView, AppError> {
        let head = sqlx::query(
            r#"
            select h.id, h.preissue_no, h.exception_type,
                   h.supplier_id, s.supplier_name,
                   h.work_order_no, h.process_name, h.workshop_name,
                   h.issue_date, h.operator_id, h.reason,
                   h.exception_status, h.timeout_flag, h.expected_close_date,
                   h.remark, h.created_at, h.updated_at
              from wms.wms_preissue_h h
              left join mdm.mdm_supplier s on s.id = h.supplier_id
             where h.id = $1 and h.tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found(format!("异常先发 id={id} 不存在")))?;

        let lines = sqlx::query(
            r#"
            select d.id, d.line_no, d.material_id, m.material_code,
                   d.qty, d.filled_qty, d.unfilled_qty,
                   d.expected_batch_no, d.target_desc, d.line_status,
                   d.closed_by_inbound_line_id, d.note
              from wms.wms_preissue_d d
              left join mdm.mdm_material m on m.id = d.material_id
             where d.preissue_id = $1 and d.tenant_id = $2
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

    async fn list(
        &self,
        tenant_id: i64,
        q: &QueryPreissues,
    ) -> Result<Vec<PreissueHeadView>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select h.id, h.preissue_no, h.exception_type,
                   h.supplier_id, s.supplier_name,
                   h.work_order_no, h.process_name, h.workshop_name,
                   h.issue_date, h.operator_id, h.reason,
                   h.exception_status, h.timeout_flag, h.expected_close_date,
                   h.remark, h.created_at, h.updated_at
              from wms.wms_preissue_h h
              left join mdm.mdm_supplier s on s.id = h.supplier_id
             where h.tenant_id =
            "#,
        );
        qb.push_bind(tenant_id);
        if let Some(no) = &q.preissue_no {
            qb.push(" and h.preissue_no = ").push_bind(no.clone());
        }
        if let Some(sid) = q.supplier_id {
            qb.push(" and h.supplier_id = ").push_bind(sid);
        }
        if let Some(wo) = &q.work_order_no {
            qb.push(" and h.work_order_no = ").push_bind(wo.clone());
        }
        if let Some(s) = &q.exception_status {
            qb.push(" and h.exception_status = ").push_bind(s.clone());
        }
        if let Some(t) = q.timeout_flag {
            qb.push(" and h.timeout_flag = ").push_bind(t);
        }
        if let Some(from) = q.date_from {
            qb.push(" and h.issue_date >= ").push_bind(from);
        }
        if let Some(to) = q.date_to {
            qb.push(" and h.issue_date <= ").push_bind(to);
        }
        qb.push(" order by h.issue_date desc, h.id desc limit 500");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| row_to_head(r, vec![])).collect())
    }

    async fn apply_fill(
        &self,
        tenant_id: i64,
        line_id: i64,
        filled_now: Decimal,
    ) -> Result<(String, String, i64, String), AppError> {
        let mut tx = self.pool.begin().await?;

        // 锁行 + 拿当前数据
        let line = sqlx::query(
            r#"
            select d.id, d.preissue_id, d.qty, d.filled_qty, d.unfilled_qty,
                   d.material_id, d.expected_batch_no, d.line_status,
                   h.preissue_no
              from wms.wms_preissue_d d
              join wms.wms_preissue_h h on h.id = d.preissue_id
             where d.id = $1 and d.tenant_id = $2
             for update of d
            "#,
        )
        .bind(line_id)
        .bind(tenant_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::not_found(format!("preissue_line id={line_id} 不存在")))?;

        let unfilled: Decimal = line.get("unfilled_qty");
        let status: String = line.get("line_status");

        if status == "CLOSED" {
            return Err(PreissueError::already_closed(
                line.get::<String, _>("preissue_no").as_str(),
            ));
        }
        if filled_now > unfilled {
            return Err(PreissueError::overfill(line_id));
        }

        let new_filled: Decimal = line.get::<Decimal, _>("filled_qty") + filled_now;
        let new_unfilled: Decimal = unfilled - filled_now;
        let new_line_status = if new_unfilled == Decimal::ZERO {
            "CLOSED"
        } else {
            "PARTIAL"
        };

        sqlx::query(
            r#"
            update wms.wms_preissue_d
               set filled_qty = $1, unfilled_qty = $2, line_status = $3
             where id = $4 and tenant_id = $5
            "#,
        )
        .bind(new_filled)
        .bind(new_unfilled)
        .bind(new_line_status)
        .bind(line_id)
        .bind(tenant_id)
        .execute(&mut *tx)
        .await?;

        // 重新聚合 head 状态
        let preissue_id: i64 = line.get("preissue_id");
        let agg: (i64, i64) = sqlx::query_as(
            r#"
            select
                sum(case when line_status = 'CLOSED'  then 1 else 0 end)::bigint,
                sum(case when line_status = 'PENDING' then 1 else 0 end)::bigint
              from wms.wms_preissue_d
             where preissue_id = $1 and tenant_id = $2
            "#,
        )
        .bind(preissue_id)
        .bind(tenant_id)
        .fetch_one(&mut *tx)
        .await?;

        let (closed_cnt, pending_cnt) = agg;
        let total: i64 = sqlx::query_scalar(
            "select count(*) from wms.wms_preissue_d where preissue_id = $1 and tenant_id = $2",
        )
        .bind(preissue_id)
        .bind(tenant_id)
        .fetch_one(&mut *tx)
        .await?;

        let new_head_status = if closed_cnt == total {
            "CLOSED"
        } else if pending_cnt == total {
            "PENDING"
        } else {
            "PARTIAL"
        };

        sqlx::query(
            "update wms.wms_preissue_h set exception_status = $1 where id = $2 and tenant_id = $3",
        )
        .bind(new_head_status)
        .bind(preissue_id)
        .bind(tenant_id)
        .execute(&mut *tx)
        .await?;

        let preissue_no: String = line.get("preissue_no");
        let material_id: i64 = line.get("material_id");
        let batch_no: String = line
            .try_get::<Option<String>, _>("expected_batch_no")?
            .unwrap_or_default();

        tx.commit().await?;
        Ok((
            preissue_no,
            new_line_status.to_string(),
            material_id,
            batch_no,
        ))
    }

    async fn update_head_status(
        &self,
        tenant_id: i64,
        id: i64,
        status: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "update wms.wms_preissue_h set exception_status = $1 where id = $2 and tenant_id = $3",
        )
        .bind(status)
        .bind(id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

fn row_to_head(row: PgRow, lines: Vec<PreissueLineView>) -> PreissueHeadView {
    PreissueHeadView {
        id: row.get("id"),
        preissue_no: row.get("preissue_no"),
        exception_type: row.get("exception_type"),
        supplier_id: row.get("supplier_id"),
        supplier_name: row.get("supplier_name"),
        work_order_no: row.get("work_order_no"),
        process_name: row.get("process_name"),
        workshop_name: row.get("workshop_name"),
        issue_date: row.get("issue_date"),
        operator_id: row.get("operator_id"),
        reason: row.get("reason"),
        exception_status: row.get("exception_status"),
        timeout_flag: row.get("timeout_flag"),
        expected_close_date: row.get("expected_close_date"),
        remark: row.get("remark"),
        lines,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_line(row: PgRow) -> PreissueLineView {
    PreissueLineView {
        id: row.get("id"),
        line_no: row.get("line_no"),
        material_id: row.get("material_id"),
        material_code: row.get("material_code"),
        qty: row.get("qty"),
        filled_qty: row.get("filled_qty"),
        unfilled_qty: row.get("unfilled_qty"),
        expected_batch_no: row.get("expected_batch_no"),
        target_desc: row.get("target_desc"),
        line_status: row.get("line_status"),
        closed_by_inbound_line_id: row.get("closed_by_inbound_line_id"),
        note: row.get("note"),
    }
}
