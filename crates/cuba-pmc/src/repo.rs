//! pmc repo
//!
//! 操作 `wms_outsource_h / wms_outsource_send_d / wms_outsource_back_d`。
//! send_status / back_status 放在 extra_json 里(0015 已加 extra_json)。

use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{postgres::PgRow, PgPool, Postgres, Row};

use cuba_shared::{audit::AuditContext, error::AppError, types::DocStatus};

use crate::service::{
    CreateOutsourceCommand, OutsourceHeadView, OutsourceLineView, QueryOutsources, SubmitBackLine,
};

#[async_trait]
pub trait PmcRepository: Send + Sync {
    async fn create(
        &self,
        ctx: &AuditContext,
        cmd: &CreateOutsourceCommand,
    ) -> Result<OutsourceHeadView, AppError>;
    async fn get(&self, id: i64) -> Result<OutsourceHeadView, AppError>;
    async fn list(&self, q: &QueryOutsources) -> Result<Vec<OutsourceHeadView>, AppError>;
    async fn update_status(&self, id: i64, status: DocStatus) -> Result<(), AppError>;
    async fn mark_sent(&self, id: i64) -> Result<(), AppError>;
    async fn append_back(&self, id: i64, lines: &[SubmitBackLine]) -> Result<(), AppError>;
    async fn get_locations(&self, id: i64) -> Result<(i64, i64, i64, i64), AppError>;
}

pub struct PgPmcRepository {
    pool: PgPool,
}

impl PgPmcRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PmcRepository for PgPmcRepository {
    async fn create(
        &self,
        ctx: &AuditContext,
        cmd: &CreateOutsourceCommand,
    ) -> Result<OutsourceHeadView, AppError> {
        let mut tx = self.pool.begin().await?;

        let outsource_no: String = sqlx::query_scalar("select sys.fn_next_doc_no('OUTSOURCE')")
            .fetch_one(&mut *tx)
            .await?;

        let extra = serde_json::json!({
            "send_wh_id": cmd.send_wh_id,
            "send_loc_id": cmd.send_loc_id,
            "back_wh_id": cmd.back_wh_id,
            "back_loc_id": cmd.back_loc_id,
            "send_status": "DRAFT",
            "back_status": "DRAFT",
        });

        let id: i64 = sqlx::query_scalar(
            r#"
            insert into wms.wms_outsource_h
                (outsource_no, supplier_id, issue_date, operator_id,
                 doc_status, extra_json, remark)
            values ($1,$2,$3,$4,'DRAFT',$5,$6)
            returning id
            "#,
        )
        .bind(&outsource_no)
        .bind(cmd.supplier_id)
        .bind(cmd.issue_date)
        .bind(ctx.user_id)
        .bind(&extra)
        .bind(&cmd.remark)
        .fetch_one(&mut *tx)
        .await?;

        for l in &cmd.send_lines {
            sqlx::query(
                r#"
                insert into wms.wms_outsource_send_d
                    (outsource_id, line_no, material_id, batch_no, qty, unit, note)
                values ($1,$2,$3,$4,$5,$6,$7)
                "#,
            )
            .bind(id)
            .bind(l.line_no)
            .bind(l.material_id)
            .bind(&l.batch_no)
            .bind(l.qty)
            .bind(&l.unit)
            .bind(&l.note)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.get(id).await
    }

    async fn get(&self, id: i64) -> Result<OutsourceHeadView, AppError> {
        let head = sqlx::query(
            r#"
            select h.id, h.outsource_no, h.supplier_id, s.supplier_name,
                   h.issue_date, h.operator_id, h.doc_status, h.extra_json,
                   h.remark, h.created_at, h.updated_at
              from wms.wms_outsource_h h
              left join mdm.mdm_supplier s on s.id = h.supplier_id
             where h.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found(format!("委外单 id={id} 不存在")))?;

        let send_lines = sqlx::query(
            r#"
            select d.id, d.line_no, d.material_id, m.material_code,
                   d.batch_no, d.qty,
                   coalesce(d.qty, 0) as actual_qty,
                   d.unit, d.note
              from wms.wms_outsource_send_d d
              left join mdm.mdm_material m on m.id = d.material_id
             where d.outsource_id = $1
             order by d.line_no
            "#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(row_to_line)
        .collect();

        let back_lines = sqlx::query(
            r#"
            select d.id, d.line_no, d.material_id, m.material_code,
                   d.batch_no,
                   d.qty as qty,
                   d.actual_qty,
                   d.unit, d.note
              from wms.wms_outsource_back_d d
              left join mdm.mdm_material m on m.id = d.material_id
             where d.outsource_id = $1
             order by d.line_no
            "#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(row_to_line)
        .collect();

        Ok(row_to_head(head, send_lines, back_lines))
    }

    async fn list(&self, q: &QueryOutsources) -> Result<Vec<OutsourceHeadView>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select h.id, h.outsource_no, h.supplier_id, s.supplier_name,
                   h.issue_date, h.operator_id, h.doc_status, h.extra_json,
                   h.remark, h.created_at, h.updated_at
              from wms.wms_outsource_h h
              left join mdm.mdm_supplier s on s.id = h.supplier_id
             where 1 = 1
            "#,
        );
        if let Some(no) = &q.outsource_no {
            qb.push(" and h.outsource_no = ").push_bind(no.clone());
        }
        if let Some(sid) = q.supplier_id {
            qb.push(" and h.supplier_id = ").push_bind(sid);
        }
        if let Some(s) = &q.doc_status {
            qb.push(" and h.doc_status = ").push_bind(s.clone());
        }
        if let Some(f) = q.date_from {
            qb.push(" and h.issue_date >= ").push_bind(f);
        }
        if let Some(t) = q.date_to {
            qb.push(" and h.issue_date <= ").push_bind(t);
        }
        qb.push(" order by h.issue_date desc, h.id desc limit 500");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|r| row_to_head(r, vec![], vec![]))
            .collect())
    }

    async fn update_status(&self, id: i64, status: DocStatus) -> Result<(), AppError> {
        sqlx::query("update wms.wms_outsource_h set doc_status = $1 where id = $2")
            .bind(status.as_str())
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn mark_sent(&self, id: i64) -> Result<(), AppError> {
        sqlx::query(
            r#"
            update wms.wms_outsource_h
               set extra_json = jsonb_set(coalesce(extra_json, '{}'::jsonb), '{send_status}', '"SENT"'::jsonb),
                   doc_status = 'SUBMITTED'
             where id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool).await?;
        Ok(())
    }

    async fn append_back(&self, id: i64, lines: &[SubmitBackLine]) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;
        // 简单实现:每次回料 insert 新行(不合并)
        let next_no: i32 = sqlx::query_scalar(
            "select coalesce(max(line_no), 0) + 1 from wms.wms_outsource_back_d where outsource_id = $1",
        )
        .bind(id)
        .fetch_one(&mut *tx).await?;

        for (i, l) in lines.iter().enumerate() {
            sqlx::query(
                r#"
                insert into wms.wms_outsource_back_d
                    (outsource_id, line_no, material_id, batch_no, qty, actual_qty, unit, note)
                values ($1,$2,$3,$4,$5,$5,$6,$7)
                "#,
            )
            .bind(id)
            .bind(next_no + i as i32)
            .bind(l.material_id)
            .bind(&l.batch_no)
            .bind(l.qty)
            .bind(&l.unit)
            .bind(&l.note)
            .execute(&mut *tx)
            .await?;
        }

        // 聚合判定:若 back 总量 >= send 总量,则整单完成
        let sent_total: Decimal = sqlx::query_scalar(
            "select coalesce(sum(qty), 0) from wms.wms_outsource_send_d where outsource_id = $1",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let back_total: Decimal = sqlx::query_scalar(
            "select coalesce(sum(actual_qty), 0) from wms.wms_outsource_back_d where outsource_id = $1",
        )
        .bind(id)
        .fetch_one(&mut *tx).await?;

        let new_back_status = if back_total >= sent_total {
            "COMPLETED"
        } else {
            "PARTIAL"
        };
        let new_doc_status = if back_total >= sent_total {
            "COMPLETED"
        } else {
            "SUBMITTED"
        };

        sqlx::query(
            r#"
            update wms.wms_outsource_h
               set extra_json = jsonb_set(coalesce(extra_json, '{}'::jsonb),
                                          '{back_status}',
                                          to_jsonb($1::text)),
                   doc_status = $2
             where id = $3
            "#,
        )
        .bind(new_back_status)
        .bind(new_doc_status)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn get_locations(&self, id: i64) -> Result<(i64, i64, i64, i64), AppError> {
        let extra: serde_json::Value =
            sqlx::query_scalar("select extra_json from wms.wms_outsource_h where id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(|| AppError::not_found(format!("委外单 id={id} 不存在")))?;

        let sw = extra
            .get("send_wh_id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| AppError::validation("send_wh_id 缺失"))?;
        let sl = extra
            .get("send_loc_id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| AppError::validation("send_loc_id 缺失"))?;
        let bw = extra
            .get("back_wh_id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| AppError::validation("back_wh_id 缺失"))?;
        let bl = extra
            .get("back_loc_id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| AppError::validation("back_loc_id 缺失"))?;
        Ok((sw, sl, bw, bl))
    }
}

fn row_to_head(
    row: PgRow,
    send_lines: Vec<OutsourceLineView>,
    back_lines: Vec<OutsourceLineView>,
) -> OutsourceHeadView {
    let extra: serde_json::Value = row
        .try_get("extra_json")
        .unwrap_or_else(|_| serde_json::json!({}));
    let send_status = extra
        .get("send_status")
        .and_then(|v| v.as_str())
        .unwrap_or("DRAFT")
        .to_string();
    let back_status = extra
        .get("back_status")
        .and_then(|v| v.as_str())
        .unwrap_or("DRAFT")
        .to_string();

    OutsourceHeadView {
        id: row.get("id"),
        outsource_no: row.get("outsource_no"),
        supplier_id: row.get("supplier_id"),
        supplier_name: row.get("supplier_name"),
        issue_date: row.get("issue_date"),
        operator_id: row.get("operator_id"),
        doc_status: row.get("doc_status"),
        send_status,
        back_status,
        remark: row.get("remark"),
        send_lines,
        back_lines,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_line(row: PgRow) -> OutsourceLineView {
    OutsourceLineView {
        id: row.get("id"),
        line_no: row.get("line_no"),
        material_id: row.get("material_id"),
        material_code: row.get("material_code"),
        batch_no: row.get("batch_no"),
        qty: row.get("qty"),
        actual_qty: row.try_get("actual_qty").unwrap_or_default(),
        unit: row.get("unit"),
        note: row.get("note"),
    }
}
