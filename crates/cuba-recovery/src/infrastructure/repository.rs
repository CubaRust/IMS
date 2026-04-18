//! recovery repo
//!
//! 操作 4 张表:`wms_recovery_h / wms_recovery_in / wms_recovery_out / wms_recovery_scrap`
//!
//! DDL 里 `wms_recovery_h` 本身有 `extra_json`(0006),所以仓位存这里即可,无需新 migration。

use async_trait::async_trait;
use sqlx::{postgres::PgRow, PgPool, Postgres, Row};

use cuba_shared::{audit::AuditContext, error::AppError, types::DocStatus};

use crate::application::{
    CreateRecoveryCommand, QueryRecoveries, RecoveryHeadView, RecoveryInView,
    RecoveryOutView, RecoveryScrapView,
};

#[async_trait]
pub trait RecoveryRepository: Send + Sync {
    async fn create(
        &self,
        ctx: &AuditContext,
        cmd: &CreateRecoveryCommand,
    ) -> Result<RecoveryHeadView, AppError>;
    async fn get(&self, tenant_id: i64, id: i64) -> Result<RecoveryHeadView, AppError>;
    async fn list(
        &self,
        tenant_id: i64,
        q: &QueryRecoveries,
    ) -> Result<Vec<RecoveryHeadView>, AppError>;
    async fn update_status(
        &self,
        tenant_id: i64,
        id: i64,
        status: DocStatus,
    ) -> Result<(), AppError>;

    /// (src_wh, src_loc, scrap_wh, scrap_loc) 从 extra_json 读
    async fn get_locations(
        &self,
        tenant_id: i64,
        id: i64,
    ) -> Result<(i64, i64, i64, i64), AppError>;
}

pub struct PgRecoveryRepository {
    pool: PgPool,
}

impl PgRecoveryRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RecoveryRepository for PgRecoveryRepository {
    async fn create(
        &self,
        ctx: &AuditContext,
        cmd: &CreateRecoveryCommand,
    ) -> Result<RecoveryHeadView, AppError> {
        let mut tx = self.pool.begin().await?;

        let recovery_no: String = sqlx::query_scalar("select sys.fn_next_doc_no('RECOVERY')")
            .fetch_one(&mut *tx).await?;

        let extra = serde_json::json!({
            "source_wh_id":  cmd.source_wh_id,
            "source_loc_id": cmd.source_loc_id,
            "scrap_wh_id":   cmd.scrap_wh_id,
            "scrap_loc_id":  cmd.scrap_loc_id,
        });

        let id: i64 = sqlx::query_scalar(
            r#"
            insert into wms.wms_recovery_h
                (tenant_id, recovery_no, source_defect_id, tpl_id, recovery_date,
                 operator_id, doc_status, extra_json, remark)
            values ($1,$2,$3,$4,$5,$6,'DRAFT',$7,$8)
            returning id
            "#,
        )
        .bind(ctx.tenant_id)
        .bind(&recovery_no)
        .bind(cmd.source_defect_id)
        .bind(cmd.tpl_id)
        .bind(cmd.recovery_date)
        .bind(ctx.user_id)
        .bind(&extra)
        .bind(&cmd.remark)
        .fetch_one(&mut *tx).await?;

        for l in &cmd.inputs {
            sqlx::query(
                r#"
                insert into wms.wms_recovery_in
                    (tenant_id, recovery_id, line_no, material_id, batch_no, qty, unit, note)
                values ($1,$2,$3,$4,$5,$6,$7,$8)
                "#,
            )
            .bind(ctx.tenant_id)
            .bind(id)
            .bind(l.line_no)
            .bind(l.material_id)
            .bind(&l.batch_no)
            .bind(l.qty)
            .bind(&l.unit)
            .bind(&l.note)
            .execute(&mut *tx).await?;
        }

        for l in &cmd.outputs {
            sqlx::query(
                r#"
                insert into wms.wms_recovery_out
                    (tenant_id, recovery_id, line_no, material_id, qty, unit,
                     target_wh_id, target_loc_id, target_status, note)
                values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
                "#,
            )
            .bind(ctx.tenant_id)
            .bind(id)
            .bind(l.line_no)
            .bind(l.material_id)
            .bind(l.qty)
            .bind(&l.unit)
            .bind(l.target_wh_id)
            .bind(l.target_loc_id)
            .bind(&l.target_status)
            .bind(&l.note)
            .execute(&mut *tx).await?;
        }

        for l in &cmd.scraps {
            sqlx::query(
                r#"
                insert into wms.wms_recovery_scrap
                    (tenant_id, recovery_id, line_no, material_id, qty, unit, scrap_reason, note)
                values ($1,$2,$3,$4,$5,$6,$7,$8)
                "#,
            )
            .bind(ctx.tenant_id)
            .bind(id)
            .bind(l.line_no)
            .bind(l.material_id)
            .bind(l.qty)
            .bind(&l.unit)
            .bind(&l.scrap_reason)
            .bind(&l.note)
            .execute(&mut *tx).await?;
        }

        tx.commit().await?;
        self.get(ctx.tenant_id, id).await
    }

    async fn get(&self, tenant_id: i64, id: i64) -> Result<RecoveryHeadView, AppError> {
        let head = sqlx::query(
            r#"
            select h.id, h.recovery_no, h.source_defect_id,
                   df.defect_no as source_defect_no,
                   h.tpl_id, h.recovery_date, h.operator_id,
                   h.doc_status, h.remark, h.created_at, h.updated_at
              from wms.wms_recovery_h h
              left join wms.wms_defect_h df on df.id = h.source_defect_id
             where h.id = $1 and h.tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found(format!("拆解回收单 id={id} 不存在")))?;

        let inputs = sqlx::query(
            r#"
            select i.id, i.line_no, i.material_id, m.material_code,
                   i.batch_no, i.qty, i.unit, i.note
              from wms.wms_recovery_in i
              left join mdm.mdm_material m on m.id = i.material_id
             where i.recovery_id = $1 and i.tenant_id = $2
             order by i.line_no
            "#,
        )
        .bind(id).bind(tenant_id).fetch_all(&self.pool).await?
        .into_iter().map(row_to_in).collect();

        let outputs = sqlx::query(
            r#"
            select o.id, o.line_no, o.material_id, m.material_code,
                   o.qty, o.unit, o.target_wh_id, o.target_loc_id,
                   o.target_status, o.note
              from wms.wms_recovery_out o
              left join mdm.mdm_material m on m.id = o.material_id
             where o.recovery_id = $1 and o.tenant_id = $2
             order by o.line_no
            "#,
        )
        .bind(id).bind(tenant_id).fetch_all(&self.pool).await?
        .into_iter().map(row_to_out).collect();

        let scraps = sqlx::query(
            r#"
            select s.id, s.line_no, s.material_id, m.material_code,
                   s.qty, s.unit, s.scrap_reason, s.note
              from wms.wms_recovery_scrap s
              left join mdm.mdm_material m on m.id = s.material_id
             where s.recovery_id = $1 and s.tenant_id = $2
             order by s.line_no
            "#,
        )
        .bind(id).bind(tenant_id).fetch_all(&self.pool).await?
        .into_iter().map(row_to_scrap).collect();

        Ok(row_to_head(head, inputs, outputs, scraps))
    }

    async fn list(
        &self,
        tenant_id: i64,
        q: &QueryRecoveries,
    ) -> Result<Vec<RecoveryHeadView>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select h.id, h.recovery_no, h.source_defect_id,
                   df.defect_no as source_defect_no,
                   h.tpl_id, h.recovery_date, h.operator_id,
                   h.doc_status, h.remark, h.created_at, h.updated_at
              from wms.wms_recovery_h h
              left join wms.wms_defect_h df on df.id = h.source_defect_id
             where h.tenant_id =
            "#,
        );
        qb.push_bind(tenant_id);
        if let Some(no) = &q.recovery_no { qb.push(" and h.recovery_no = ").push_bind(no.clone()); }
        if let Some(d)  = q.source_defect_id { qb.push(" and h.source_defect_id = ").push_bind(d); }
        if let Some(s)  = &q.doc_status { qb.push(" and h.doc_status = ").push_bind(s.clone()); }
        if let Some(f)  = q.date_from { qb.push(" and h.recovery_date >= ").push_bind(f); }
        if let Some(t)  = q.date_to { qb.push(" and h.recovery_date <= ").push_bind(t); }
        qb.push(" order by h.recovery_date desc, h.id desc limit 500");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| row_to_head(r, vec![], vec![], vec![])).collect())
    }

    async fn update_status(
        &self,
        tenant_id: i64,
        id: i64,
        status: DocStatus,
    ) -> Result<(), AppError> {
        sqlx::query(
            "update wms.wms_recovery_h set doc_status = $1 where id = $2 and tenant_id = $3",
        )
        .bind(status.as_str())
        .bind(id)
        .bind(tenant_id)
        .execute(&self.pool).await?;
        Ok(())
    }

    async fn get_locations(
        &self,
        tenant_id: i64,
        id: i64,
    ) -> Result<(i64, i64, i64, i64), AppError> {
        let extra: serde_json::Value = sqlx::query_scalar(
            "select extra_json from wms.wms_recovery_h where id = $1 and tenant_id = $2",
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool).await?
        .ok_or_else(|| AppError::not_found(format!("拆解回收单 id={id} 不存在")))?;

        let sw = extra.get("source_wh_id").and_then(|v| v.as_i64())
            .ok_or_else(|| AppError::validation("source_wh_id 缺失"))?;
        let sl = extra.get("source_loc_id").and_then(|v| v.as_i64())
            .ok_or_else(|| AppError::validation("source_loc_id 缺失"))?;
        let xw = extra.get("scrap_wh_id").and_then(|v| v.as_i64())
            .ok_or_else(|| AppError::validation("scrap_wh_id 缺失"))?;
        let xl = extra.get("scrap_loc_id").and_then(|v| v.as_i64())
            .ok_or_else(|| AppError::validation("scrap_loc_id 缺失"))?;
        Ok((sw, sl, xw, xl))
    }
}

fn row_to_head(
    row: PgRow,
    inputs: Vec<RecoveryInView>,
    outputs: Vec<RecoveryOutView>,
    scraps: Vec<RecoveryScrapView>,
) -> RecoveryHeadView {
    RecoveryHeadView {
        id: row.get("id"),
        recovery_no: row.get("recovery_no"),
        source_defect_id: row.get("source_defect_id"),
        source_defect_no: row.get("source_defect_no"),
        tpl_id: row.get("tpl_id"),
        recovery_date: row.get("recovery_date"),
        operator_id: row.get("operator_id"),
        doc_status: row.get("doc_status"),
        remark: row.get("remark"),
        inputs,
        outputs,
        scraps,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_in(row: PgRow) -> RecoveryInView {
    RecoveryInView {
        id: row.get("id"),
        line_no: row.get("line_no"),
        material_id: row.get("material_id"),
        material_code: row.get("material_code"),
        batch_no: row.get("batch_no"),
        qty: row.get("qty"),
        unit: row.get("unit"),
        note: row.get("note"),
    }
}

fn row_to_out(row: PgRow) -> RecoveryOutView {
    RecoveryOutView {
        id: row.get("id"),
        line_no: row.get("line_no"),
        material_id: row.get("material_id"),
        material_code: row.get("material_code"),
        qty: row.get("qty"),
        unit: row.get("unit"),
        target_wh_id: row.get("target_wh_id"),
        target_loc_id: row.get("target_loc_id"),
        target_status: row.get("target_status"),
        note: row.get("note"),
    }
}

fn row_to_scrap(row: PgRow) -> RecoveryScrapView {
    RecoveryScrapView {
        id: row.get("id"),
        line_no: row.get("line_no"),
        material_id: row.get("material_id"),
        material_code: row.get("material_code"),
        qty: row.get("qty"),
        unit: row.get("unit"),
        scrap_reason: row.get("scrap_reason"),
        note: row.get("note"),
    }
}
