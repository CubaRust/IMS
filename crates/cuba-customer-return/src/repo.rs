//! customer-return repo
//!
//! DDL 的 `wms_customer_return_h` 没有 wh/loc 字段,放 extra_json(需 0015 migration 加上 extra_json)。
//! 注:0005 DDL 里这张表有 remark 但没 extra_json,所以需要补。

use async_trait::async_trait;
use sqlx::{postgres::PgRow, PgPool, Postgres, Row};

use cuba_shared::{audit::AuditContext, error::AppError, types::DocStatus};

use crate::service::{
    CreateCustomerReturnCommand, CustomerReturnHeadView, CustomerReturnLineView, JudgeLineCommand,
    QueryCustomerReturns,
};

pub struct ReturnLocations {
    pub return_wh: i64,
    pub return_loc: i64,
    pub defect_wh: Option<i64>,
    pub defect_loc: Option<i64>,
    pub scrap_wh: Option<i64>,
    pub scrap_loc: Option<i64>,
}

#[async_trait]
pub trait CustomerReturnRepository: Send + Sync {
    async fn create(
        &self,
        ctx: &AuditContext,
        cmd: &CreateCustomerReturnCommand,
    ) -> Result<CustomerReturnHeadView, AppError>;
    async fn get(&self, id: i64) -> Result<CustomerReturnHeadView, AppError>;
    async fn list(&self, q: &QueryCustomerReturns)
        -> Result<Vec<CustomerReturnHeadView>, AppError>;
    async fn update_status(&self, id: i64, status: DocStatus) -> Result<(), AppError>;
    async fn update_judges(
        &self,
        id: i64,
        judges: &[JudgeLineCommand],
        ctx: &AuditContext,
    ) -> Result<(), AppError>;
    async fn get_locations(&self, id: i64) -> Result<ReturnLocations, AppError>;
}

pub struct PgCustomerReturnRepository {
    pool: PgPool,
}

impl PgCustomerReturnRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CustomerReturnRepository for PgCustomerReturnRepository {
    async fn create(
        &self,
        ctx: &AuditContext,
        cmd: &CreateCustomerReturnCommand,
    ) -> Result<CustomerReturnHeadView, AppError> {
        let mut tx = self.pool.begin().await?;

        let return_no: String = sqlx::query_scalar("select sys.fn_next_doc_no('CUSTOMER_RETURN')")
            .fetch_one(&mut *tx)
            .await?;

        let extra = serde_json::json!({
            "return_wh_id": cmd.return_wh_id,
            "return_loc_id": cmd.return_loc_id,
            "defect_wh_id": cmd.defect_wh_id,
            "defect_loc_id": cmd.defect_loc_id,
            "scrap_wh_id": cmd.scrap_wh_id,
            "scrap_loc_id": cmd.scrap_loc_id,
        });

        let id: i64 = sqlx::query_scalar(
            r#"
            insert into wms.wms_customer_return_h
                (return_no, customer_id, return_date, original_doc_no,
                 operator_id, doc_status, extra_json, remark)
            values ($1,$2,$3,$4,$5,'DRAFT',$6,$7)
            returning id
            "#,
        )
        .bind(&return_no)
        .bind(cmd.customer_id)
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
                insert into wms.wms_customer_return_d
                    (return_id, line_no, material_id, batch_no, qty, unit,
                     return_reason, judge_method, judge_note, note)
                values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
                "#,
            )
            .bind(id)
            .bind(l.line_no)
            .bind(l.material_id)
            .bind(&l.batch_no)
            .bind(l.qty)
            .bind(&l.unit)
            .bind(&l.return_reason)
            .bind(&l.judge_method)
            .bind(&l.judge_note)
            .bind(&l.note)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.get(id).await
    }

    async fn get(&self, id: i64) -> Result<CustomerReturnHeadView, AppError> {
        let head = sqlx::query(
            r#"
            select h.id, h.return_no, h.customer_id, c.customer_name,
                   h.return_date, h.original_doc_no, h.operator_id,
                   h.doc_status, h.remark, h.created_at, h.updated_at
              from wms.wms_customer_return_h h
              left join mdm.mdm_customer c on c.id = h.customer_id
             where h.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found(format!("客户退货单 id={id} 不存在")))?;

        let lines = sqlx::query(
            r#"
            select d.id, d.line_no, d.material_id, m.material_code,
                   d.batch_no, d.qty, d.unit, d.return_reason,
                   d.judge_method, d.judge_note, d.note
              from wms.wms_customer_return_d d
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
        q: &QueryCustomerReturns,
    ) -> Result<Vec<CustomerReturnHeadView>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select h.id, h.return_no, h.customer_id, c.customer_name,
                   h.return_date, h.original_doc_no, h.operator_id,
                   h.doc_status, h.remark, h.created_at, h.updated_at
              from wms.wms_customer_return_h h
              left join mdm.mdm_customer c on c.id = h.customer_id
             where 1 = 1
            "#,
        );
        if let Some(no) = &q.return_no {
            qb.push(" and h.return_no = ").push_bind(no.clone());
        }
        if let Some(c) = q.customer_id {
            qb.push(" and h.customer_id = ").push_bind(c);
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
        sqlx::query("update wms.wms_customer_return_h set doc_status = $1 where id = $2")
            .bind(status.as_str())
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_judges(
        &self,
        id: i64,
        judges: &[JudgeLineCommand],
        ctx: &AuditContext,
    ) -> Result<(), AppError> {
        use crate::service::judge_method_to_result;
        use rust_decimal::Decimal;

        let mut tx = self.pool.begin().await?;
        for j in judges {
            sqlx::query(
                r#"update wms.wms_customer_return_d
                      set judge_method = $1, judge_note = $2
                    where id = $3 and return_id = $4"#,
            )
            .bind(&j.judge_method)
            .bind(&j.judge_note)
            .bind(j.line_id)
            .bind(id)
            .execute(&mut *tx)
            .await?;

            // Fetch qty from the line to use as judge_qty
            let judge_qty: Decimal = sqlx::query_scalar(
                "select qty from wms.wms_customer_return_d where id = $1 and return_id = $2",
            )
            .bind(j.line_id)
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;

            let judge_result = judge_method_to_result(&j.judge_method);
            let judge_reason = j.judge_note.clone().unwrap_or_default();

            sqlx::query(
                r#"insert into wms.wms_customer_return_judge
                    (customer_return_id, customer_return_line_id, judge_result,
                     judge_qty, judge_reason, judge_user_id, judge_time)
                   values ($1, $2, $3, $4, $5, $6, now())"#,
            )
            .bind(id)
            .bind(j.line_id)
            .bind(judge_result)
            .bind(judge_qty)
            .bind(&judge_reason)
            .bind(ctx.user_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn get_locations(&self, id: i64) -> Result<ReturnLocations, AppError> {
        let extra: serde_json::Value =
            sqlx::query_scalar("select extra_json from wms.wms_customer_return_h where id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(|| AppError::not_found(format!("客户退货单 id={id} 不存在")))?;

        let return_wh = extra
            .get("return_wh_id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| AppError::validation("return_wh_id 缺失"))?;
        let return_loc = extra
            .get("return_loc_id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| AppError::validation("return_loc_id 缺失"))?;
        Ok(ReturnLocations {
            return_wh,
            return_loc,
            defect_wh: extra.get("defect_wh_id").and_then(|v| v.as_i64()),
            defect_loc: extra.get("defect_loc_id").and_then(|v| v.as_i64()),
            scrap_wh: extra.get("scrap_wh_id").and_then(|v| v.as_i64()),
            scrap_loc: extra.get("scrap_loc_id").and_then(|v| v.as_i64()),
        })
    }
}

fn row_to_head(row: PgRow, lines: Vec<CustomerReturnLineView>) -> CustomerReturnHeadView {
    CustomerReturnHeadView {
        id: row.get("id"),
        return_no: row.get("return_no"),
        customer_id: row.get("customer_id"),
        customer_name: row.get("customer_name"),
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

fn row_to_line(row: PgRow) -> CustomerReturnLineView {
    CustomerReturnLineView {
        id: row.get("id"),
        line_no: row.get("line_no"),
        material_id: row.get("material_id"),
        material_code: row.get("material_code"),
        batch_no: row.get("batch_no"),
        qty: row.get("qty"),
        unit: row.get("unit"),
        return_reason: row.get("return_reason"),
        judge_method: row.get("judge_method"),
        judge_note: row.get("judge_note"),
        note: row.get("note"),
    }
}
