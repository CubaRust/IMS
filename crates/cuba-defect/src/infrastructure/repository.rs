//! defect repo
//!
//! DDL 里 `wms_defect_h` 没有 source_wh_id/source_loc_id/target_wh_id/target_loc_id 字段,
//! 我把它们存在 `extra_json` 里,避免扩展 migration。

pub mod placeholder {}

use async_trait::async_trait;
use sqlx::{postgres::PgRow, PgPool, Postgres, Row};

use cuba_shared::{audit::AuditContext, error::AppError, types::DocStatus};

use crate::application::{
    CreateDefectCommand, DefectHeadView, DefectLineView, QueryDefects,
};

#[async_trait]
pub trait DefectRepository: Send + Sync {
    async fn create(
        &self,
        ctx: &AuditContext,
        cmd: &CreateDefectCommand,
    ) -> Result<DefectHeadView, AppError>;
    async fn get(&self, id: i64) -> Result<DefectHeadView, AppError>;
    async fn list(&self, q: &QueryDefects) -> Result<Vec<DefectHeadView>, AppError>;
    async fn update_status(&self, id: i64, status: DocStatus) -> Result<(), AppError>;

    /// 从 extra_json 取仓位
    async fn get_locations(
        &self,
        id: i64,
    ) -> Result<(i64, i64, Option<i64>, Option<i64>), AppError>;
}

pub struct PgDefectRepository {
    pool: PgPool,
}

impl PgDefectRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DefectRepository for PgDefectRepository {
    async fn create(
        &self,
        ctx: &AuditContext,
        cmd: &CreateDefectCommand,
    ) -> Result<DefectHeadView, AppError> {
        let mut tx = self.pool.begin().await?;

        let defect_no: String = sqlx::query_scalar("select sys.fn_next_doc_no('DEFECT')")
            .fetch_one(&mut *tx)
            .await?;

        let extra = serde_json::json!({
            "source_wh_id": cmd.source_wh_id,
            "source_loc_id": cmd.source_loc_id,
            "target_wh_id": cmd.target_wh_id,
            "target_loc_id": cmd.target_loc_id,
        });

        let id: i64 = sqlx::query_scalar(
            r#"
            insert into wms.wms_defect_h
                (defect_no, defect_source, work_order_no, process_name,
                 product_stage, found_date, finder_name, process_method,
                 operator_id, doc_status, extra_json, remark)
            values ($1,$2,$3,$4,$5,$6,$7,$8,$9,'DRAFT',$10,$11)
            returning id
            "#,
        )
        .bind(&defect_no)
        .bind(&cmd.defect_source)
        .bind(&cmd.work_order_no)
        .bind(&cmd.process_name)
        .bind(&cmd.product_stage)
        .bind(cmd.found_date)
        .bind(&cmd.finder_name)
        .bind(&cmd.process_method)
        .bind(ctx.user_id)
        .bind(&extra)
        .bind(&cmd.remark)
        .fetch_one(&mut *tx)
        .await?;

        for l in &cmd.lines {
            sqlx::query(
                r#"
                insert into wms.wms_defect_d
                    (defect_id, line_no, material_id, batch_no, qty, unit,
                     defect_reason, defect_desc, source_doc_type, source_doc_no, note)
                values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
                "#,
            )
            .bind(id)
            .bind(l.line_no)
            .bind(l.material_id)
            .bind(&l.batch_no)
            .bind(l.qty)
            .bind(&l.unit)
            .bind(&l.defect_reason)
            .bind(&l.defect_desc)
            .bind(&l.source_doc_type)
            .bind(&l.source_doc_no)
            .bind(&l.note)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.get(id).await
    }

    async fn get(&self, id: i64) -> Result<DefectHeadView, AppError> {
        let head = sqlx::query(SELECT_HEAD)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::not_found(format!("不良单 id={id} 不存在")))?;

        let lines = sqlx::query(
            r#"
            select d.id, d.line_no, d.material_id, m.material_code,
                   d.batch_no, d.qty, d.unit,
                   d.defect_reason, d.defect_desc,
                   d.source_doc_type, d.source_doc_no,
                   d.target_status, d.note
              from wms.wms_defect_d d
              left join mdm.mdm_material m on m.id = d.material_id
             where d.defect_id = $1
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

    async fn list(&self, q: &QueryDefects) -> Result<Vec<DefectHeadView>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select h.id, h.defect_no, h.defect_source, h.work_order_no, h.process_name,
                   h.product_stage, h.found_date, h.finder_name, h.process_method,
                   h.operator_id, h.doc_status, h.remark, h.created_at, h.updated_at
              from wms.wms_defect_h h
             where 1 = 1
            "#,
        );
        if let Some(no) = &q.defect_no {
            qb.push(" and h.defect_no = ").push_bind(no.clone());
        }
        if let Some(s) = &q.defect_source {
            qb.push(" and h.defect_source = ").push_bind(s.clone());
        }
        if let Some(st) = &q.product_stage {
            qb.push(" and h.product_stage = ").push_bind(st.clone());
        }
        if let Some(m) = &q.process_method {
            qb.push(" and h.process_method = ").push_bind(m.clone());
        }
        if let Some(wo) = &q.work_order_no {
            qb.push(" and h.work_order_no = ").push_bind(wo.clone());
        }
        if let Some(s) = &q.doc_status {
            qb.push(" and h.doc_status = ").push_bind(s.clone());
        }
        if let Some(from) = q.date_from {
            qb.push(" and h.found_date >= ").push_bind(from);
        }
        if let Some(to) = q.date_to {
            qb.push(" and h.found_date <= ").push_bind(to);
        }
        qb.push(" order by h.found_date desc, h.id desc limit 500");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| row_to_head(r, vec![])).collect())
    }

    async fn update_status(&self, id: i64, status: DocStatus) -> Result<(), AppError> {
        sqlx::query("update wms.wms_defect_h set doc_status = $1 where id = $2")
            .bind(status.as_str())
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_locations(
        &self,
        id: i64,
    ) -> Result<(i64, i64, Option<i64>, Option<i64>), AppError> {
        let extra: serde_json::Value = sqlx::query_scalar(
            "select extra_json from wms.wms_defect_h where id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found(format!("不良单 id={id} 不存在")))?;

        let src_wh = extra.get("source_wh_id").and_then(|v| v.as_i64())
            .ok_or_else(|| AppError::validation("source_wh_id 缺失"))?;
        let src_loc = extra.get("source_loc_id").and_then(|v| v.as_i64())
            .ok_or_else(|| AppError::validation("source_loc_id 缺失"))?;
        let tgt_wh = extra.get("target_wh_id").and_then(|v| v.as_i64());
        let tgt_loc = extra.get("target_loc_id").and_then(|v| v.as_i64());
        Ok((src_wh, src_loc, tgt_wh, tgt_loc))
    }
}

const SELECT_HEAD: &str = r#"
    select h.id, h.defect_no, h.defect_source, h.work_order_no, h.process_name,
           h.product_stage, h.found_date, h.finder_name, h.process_method,
           h.operator_id, h.doc_status, h.remark, h.created_at, h.updated_at
      from wms.wms_defect_h h
     where h.id = $1
"#;

fn row_to_head(row: PgRow, lines: Vec<DefectLineView>) -> DefectHeadView {
    DefectHeadView {
        id: row.get("id"),
        defect_no: row.get("defect_no"),
        defect_source: row.get("defect_source"),
        work_order_no: row.get("work_order_no"),
        process_name: row.get("process_name"),
        product_stage: row.get("product_stage"),
        found_date: row.get("found_date"),
        finder_name: row.get("finder_name"),
        process_method: row.get("process_method"),
        operator_id: row.get("operator_id"),
        doc_status: row.get("doc_status"),
        remark: row.get("remark"),
        lines,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_line(row: PgRow) -> DefectLineView {
    DefectLineView {
        id: row.get("id"),
        line_no: row.get("line_no"),
        material_id: row.get("material_id"),
        material_code: row.get("material_code"),
        batch_no: row.get("batch_no"),
        qty: row.get("qty"),
        unit: row.get("unit"),
        defect_reason: row.get("defect_reason"),
        defect_desc: row.get("defect_desc"),
        source_doc_type: row.get("source_doc_type"),
        source_doc_no: row.get("source_doc_no"),
        target_status: row.get("target_status"),
        note: row.get("note"),
    }
}
