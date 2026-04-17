//! scrap repo
//!
//! `wms_scrap_h` DDL 没有 wh/loc 字段,我把 4 个仓位 id 塞 extra_json。
//! (scrap_h DDL 本期没有 extra_json 字段?我 check:0007 里 scrap_h 没 extra_json。)
//! 因此我们用一个**补丁 migration** 给 scrap_h 加 extra_json,在 0014 补。

use async_trait::async_trait;
use sqlx::{postgres::PgRow, PgPool, Postgres, Row};

use cuba_shared::{audit::AuditContext, error::AppError, types::DocStatus};

use crate::application::{
    CreateScrapCommand, QueryScraps, ScrapHeadView, ScrapLineView,
};

#[async_trait]
pub trait ScrapRepository: Send + Sync {
    async fn create(
        &self,
        ctx: &AuditContext,
        cmd: &CreateScrapCommand,
    ) -> Result<ScrapHeadView, AppError>;
    async fn get(&self, id: i64) -> Result<ScrapHeadView, AppError>;
    async fn list(&self, q: &QueryScraps) -> Result<Vec<ScrapHeadView>, AppError>;
    async fn update_status(&self, id: i64, status: DocStatus) -> Result<(), AppError>;

    /// (src_wh, src_loc, scrap_wh, scrap_loc) 从 extra_json 读
    async fn get_locations(&self, id: i64) -> Result<(i64, i64, i64, i64), AppError>;
}

pub struct PgScrapRepository {
    pool: PgPool,
}

impl PgScrapRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ScrapRepository for PgScrapRepository {
    async fn create(
        &self,
        ctx: &AuditContext,
        cmd: &CreateScrapCommand,
    ) -> Result<ScrapHeadView, AppError> {
        let mut tx = self.pool.begin().await?;
        let scrap_no: String = sqlx::query_scalar("select sys.fn_next_doc_no('SCRAP')")
            .fetch_one(&mut *tx).await?;

        let id: i64 = sqlx::query_scalar(
            r#"
            insert into wms.wms_scrap_h
                (scrap_no, scrap_source, source_doc_type, source_doc_no,
                 scrap_date, operator_id, doc_status, remark)
            values ($1,$2,$3,$4,$5,$6,'DRAFT',$7)
            returning id
            "#,
        )
        .bind(&scrap_no)
        .bind(&cmd.scrap_source)
        .bind(&cmd.source_doc_type)
        .bind(&cmd.source_doc_no)
        .bind(cmd.scrap_date)
        .bind(ctx.user_id)
        .bind(&cmd.remark)
        .fetch_one(&mut *tx).await?;

        // 把 4 个仓位放在 extra_json(需 0014 migration 给 scrap_h 加 extra_json)
        let extra = serde_json::json!({
            "source_wh_id": cmd.source_wh_id,
            "source_loc_id": cmd.source_loc_id,
            "scrap_wh_id": cmd.scrap_wh_id,
            "scrap_loc_id": cmd.scrap_loc_id,
        });
        sqlx::query(
            r#"update wms.wms_scrap_h
                  set extra_json = $1
                where id = $2"#,
        )
        .bind(&extra)
        .bind(id)
        .execute(&mut *tx).await?;

        for l in &cmd.lines {
            sqlx::query(
                r#"
                insert into wms.wms_scrap_d
                    (scrap_id, line_no, material_id, batch_no, qty, unit,
                     stock_status, scrap_reason, note)
                values ($1,$2,$3,$4,$5,$6,$7,$8,$9)
                "#,
            )
            .bind(id)
            .bind(l.line_no)
            .bind(l.material_id)
            .bind(&l.batch_no)
            .bind(l.qty)
            .bind(&l.unit)
            .bind(&l.stock_status)
            .bind(&l.scrap_reason)
            .bind(&l.note)
            .execute(&mut *tx).await?;
        }

        tx.commit().await?;
        self.get(id).await
    }

    async fn get(&self, id: i64) -> Result<ScrapHeadView, AppError> {
        let head = sqlx::query(
            r#"
            select h.id, h.scrap_no, h.scrap_source, h.source_doc_type, h.source_doc_no,
                   h.scrap_date, h.operator_id, h.doc_status, h.remark,
                   h.created_at, h.updated_at
              from wms.wms_scrap_h h
             where h.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found(format!("报废单 id={id} 不存在")))?;

        let lines = sqlx::query(
            r#"
            select d.id, d.line_no, d.material_id, m.material_code,
                   d.batch_no, d.qty, d.unit, d.stock_status,
                   d.scrap_reason, d.note
              from wms.wms_scrap_d d
              left join mdm.mdm_material m on m.id = d.material_id
             where d.scrap_id = $1
             order by d.line_no
            "#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(row_to_line)
        .collect();

        Ok(row_to_head(head, lines))
    }

    async fn list(&self, q: &QueryScraps) -> Result<Vec<ScrapHeadView>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select h.id, h.scrap_no, h.scrap_source, h.source_doc_type, h.source_doc_no,
                   h.scrap_date, h.operator_id, h.doc_status, h.remark,
                   h.created_at, h.updated_at
              from wms.wms_scrap_h h
             where 1 = 1
            "#,
        );
        if let Some(no) = &q.scrap_no { qb.push(" and h.scrap_no = ").push_bind(no.clone()); }
        if let Some(s) = &q.scrap_source { qb.push(" and h.scrap_source = ").push_bind(s.clone()); }
        if let Some(s) = &q.doc_status { qb.push(" and h.doc_status = ").push_bind(s.clone()); }
        if let Some(from) = q.date_from { qb.push(" and h.scrap_date >= ").push_bind(from); }
        if let Some(to) = q.date_to { qb.push(" and h.scrap_date <= ").push_bind(to); }
        qb.push(" order by h.scrap_date desc, h.id desc limit 500");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| row_to_head(r, vec![])).collect())
    }

    async fn update_status(&self, id: i64, status: DocStatus) -> Result<(), AppError> {
        sqlx::query("update wms.wms_scrap_h set doc_status = $1 where id = $2")
            .bind(status.as_str())
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_locations(&self, id: i64) -> Result<(i64, i64, i64, i64), AppError> {
        let extra: serde_json::Value = sqlx::query_scalar(
            "select extra_json from wms.wms_scrap_h where id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found(format!("报废单 id={id} 不存在")))?;

        let src_wh = extra.get("source_wh_id").and_then(|v| v.as_i64())
            .ok_or_else(|| AppError::validation("source_wh_id 缺失"))?;
        let src_loc = extra.get("source_loc_id").and_then(|v| v.as_i64())
            .ok_or_else(|| AppError::validation("source_loc_id 缺失"))?;
        let sc_wh = extra.get("scrap_wh_id").and_then(|v| v.as_i64())
            .ok_or_else(|| AppError::validation("scrap_wh_id 缺失"))?;
        let sc_loc = extra.get("scrap_loc_id").and_then(|v| v.as_i64())
            .ok_or_else(|| AppError::validation("scrap_loc_id 缺失"))?;
        Ok((src_wh, src_loc, sc_wh, sc_loc))
    }
}

fn row_to_head(row: PgRow, lines: Vec<ScrapLineView>) -> ScrapHeadView {
    ScrapHeadView {
        id: row.get("id"),
        scrap_no: row.get("scrap_no"),
        scrap_source: row.get("scrap_source"),
        source_doc_type: row.get("source_doc_type"),
        source_doc_no: row.get("source_doc_no"),
        scrap_date: row.get("scrap_date"),
        operator_id: row.get("operator_id"),
        doc_status: row.get("doc_status"),
        remark: row.get("remark"),
        lines,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_line(row: PgRow) -> ScrapLineView {
    ScrapLineView {
        id: row.get("id"),
        line_no: row.get("line_no"),
        material_id: row.get("material_id"),
        material_code: row.get("material_code"),
        batch_no: row.get("batch_no"),
        qty: row.get("qty"),
        unit: row.get("unit"),
        stock_status: row.get("stock_status"),
        scrap_reason: row.get("scrap_reason"),
        note: row.get("note"),
    }
}
