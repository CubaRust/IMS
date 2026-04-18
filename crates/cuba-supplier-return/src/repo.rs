//! supplier-return repo

use async_trait::async_trait;
use sqlx::{postgres::PgRow, PgPool, Postgres, Row};

use cuba_shared::{audit::AuditContext, error::AppError, types::DocStatus};

use crate::service::{
    CreateSupplierReturnCommand, QuerySupplierReturns, SupplierReturnHeadView,
    SupplierReturnLineView,
};

#[async_trait]
pub trait SupplierReturnRepository: Send + Sync {
    async fn create(
        &self,
        ctx: &AuditContext,
        cmd: &CreateSupplierReturnCommand,
    ) -> Result<SupplierReturnHeadView, AppError>;
    async fn get(&self, id: i64) -> Result<SupplierReturnHeadView, AppError>;
    async fn list(&self, q: &QuerySupplierReturns)
        -> Result<Vec<SupplierReturnHeadView>, AppError>;
    async fn update_status(&self, id: i64, status: DocStatus) -> Result<(), AppError>;
    async fn get_source_location(&self, id: i64) -> Result<(i64, i64), AppError>;
}

pub struct PgSupplierReturnRepository {
    pool: PgPool,
}

impl PgSupplierReturnRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SupplierReturnRepository for PgSupplierReturnRepository {
    async fn create(
        &self,
        ctx: &AuditContext,
        cmd: &CreateSupplierReturnCommand,
    ) -> Result<SupplierReturnHeadView, AppError> {
        let mut tx = self.pool.begin().await?;

        let return_no: String = sqlx::query_scalar("select sys.fn_next_doc_no('SUPPLIER_RETURN')")
            .fetch_one(&mut *tx)
            .await?;

        let extra = serde_json::json!({
            "source_wh_id": cmd.source_wh_id,
            "source_loc_id": cmd.source_loc_id,
        });

        let id: i64 = sqlx::query_scalar(
            r#"
            insert into wms.wms_supplier_return_h
                (return_no, supplier_id, return_date, original_doc_no,
                 operator_id, doc_status, extra_json, remark)
            values ($1,$2,$3,$4,$5,'DRAFT',$6,$7)
            returning id
            "#,
        )
        .bind(&return_no)
        .bind(cmd.supplier_id)
        .bind(cmd.return_date)
        .bind(&cmd.original_doc_no)
        .bind(ctx.user_id)
        .bind(&extra)
        .bind(&cmd.remark)
        .fetch_one(&mut *tx)
        .await?;

        for l in &cmd.lines {
            sqlx::query(
                r#"
                insert into wms.wms_supplier_return_d
                    (return_id, line_no, material_id, batch_no, qty, unit,
                     source_status, return_reason, note)
                values ($1,$2,$3,$4,$5,$6,$7,$8,$9)
                "#,
            )
            .bind(id)
            .bind(l.line_no)
            .bind(l.material_id)
            .bind(&l.batch_no)
            .bind(l.qty)
            .bind(&l.unit)
            .bind(&l.source_status)
            .bind(&l.return_reason)
            .bind(&l.note)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.get(id).await
    }

    async fn get(&self, id: i64) -> Result<SupplierReturnHeadView, AppError> {
        let head = sqlx::query(
            r#"
            select h.id, h.return_no, h.supplier_id, s.supplier_name,
                   h.return_date, h.original_doc_no, h.operator_id,
                   h.doc_status, h.remark, h.created_at, h.updated_at
              from wms.wms_supplier_return_h h
              left join mdm.mdm_supplier s on s.id = h.supplier_id
             where h.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found(format!("退供单 id={id} 不存在")))?;

        let lines = sqlx::query(
            r#"
            select d.id, d.line_no, d.material_id, m.material_code,
                   d.batch_no, d.qty, d.unit, d.source_status,
                   d.return_reason, d.note
              from wms.wms_supplier_return_d d
              left join mdm.mdm_material m on m.id = d.material_id
             where d.return_id = $1
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

    async fn list(
        &self,
        q: &QuerySupplierReturns,
    ) -> Result<Vec<SupplierReturnHeadView>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select h.id, h.return_no, h.supplier_id, s.supplier_name,
                   h.return_date, h.original_doc_no, h.operator_id,
                   h.doc_status, h.remark, h.created_at, h.updated_at
              from wms.wms_supplier_return_h h
              left join mdm.mdm_supplier s on s.id = h.supplier_id
             where 1 = 1
            "#,
        );
        if let Some(no) = &q.return_no {
            qb.push(" and h.return_no = ").push_bind(no.clone());
        }
        if let Some(sid) = q.supplier_id {
            qb.push(" and h.supplier_id = ").push_bind(sid);
        }
        if let Some(s) = &q.doc_status {
            qb.push(" and h.doc_status = ").push_bind(s.clone());
        }
        if let Some(f) = q.date_from {
            qb.push(" and h.return_date >= ").push_bind(f);
        }
        if let Some(t) = q.date_to {
            qb.push(" and h.return_date <= ").push_bind(t);
        }
        qb.push(" order by h.return_date desc, h.id desc limit 500");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| row_to_head(r, vec![])).collect())
    }

    async fn update_status(&self, id: i64, status: DocStatus) -> Result<(), AppError> {
        sqlx::query("update wms.wms_supplier_return_h set doc_status = $1 where id = $2")
            .bind(status.as_str())
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_source_location(&self, id: i64) -> Result<(i64, i64), AppError> {
        let extra: serde_json::Value =
            sqlx::query_scalar("select extra_json from wms.wms_supplier_return_h where id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(|| AppError::not_found(format!("退供单 id={id} 不存在")))?;

        let w = extra
            .get("source_wh_id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| AppError::validation("source_wh_id 缺失"))?;
        let l = extra
            .get("source_loc_id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| AppError::validation("source_loc_id 缺失"))?;
        Ok((w, l))
    }
}

fn row_to_head(row: PgRow, lines: Vec<SupplierReturnLineView>) -> SupplierReturnHeadView {
    SupplierReturnHeadView {
        id: row.get("id"),
        return_no: row.get("return_no"),
        supplier_id: row.get("supplier_id"),
        supplier_name: row.get("supplier_name"),
        return_date: row.get("return_date"),
        original_doc_no: row.get("original_doc_no"),
        operator_id: row.get("operator_id"),
        doc_status: row.get("doc_status"),
        remark: row.get("remark"),
        lines,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_line(row: PgRow) -> SupplierReturnLineView {
    SupplierReturnLineView {
        id: row.get("id"),
        line_no: row.get("line_no"),
        material_id: row.get("material_id"),
        material_code: row.get("material_code"),
        batch_no: row.get("batch_no"),
        qty: row.get("qty"),
        unit: row.get("unit"),
        source_status: row.get("source_status"),
        return_reason: row.get("return_reason"),
        note: row.get("note"),
    }
}
