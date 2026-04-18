//! stocktake repo

use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{postgres::PgRow, PgPool, Postgres, Row};

use cuba_shared::{audit::AuditContext, error::AppError, types::DocStatus};

use crate::service::{
    CreateStocktakeCommand, QueryStocktakes, RecordCountLine, StocktakeHeadView, StocktakeLineView,
};

#[async_trait]
pub trait StocktakeRepository: Send + Sync {
    async fn create(
        &self,
        ctx: &AuditContext,
        cmd: &CreateStocktakeCommand,
    ) -> Result<StocktakeHeadView, AppError>;
    async fn get(&self, id: i64) -> Result<StocktakeHeadView, AppError>;
    async fn list(&self, q: &QueryStocktakes) -> Result<Vec<StocktakeHeadView>, AppError>;
    async fn update_status(&self, id: i64, status: DocStatus) -> Result<(), AppError>;
    async fn update_counts(&self, id: i64, lines: &[RecordCountLine]) -> Result<(), AppError>;
}

pub struct PgStocktakeRepository {
    pool: PgPool,
}

impl PgStocktakeRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StocktakeRepository for PgStocktakeRepository {
    async fn create(
        &self,
        ctx: &AuditContext,
        cmd: &CreateStocktakeCommand,
    ) -> Result<StocktakeHeadView, AppError> {
        let mut tx = self.pool.begin().await?;

        let stocktake_no: String = sqlx::query_scalar("select sys.fn_next_doc_no('STOCKTAKE')")
            .fetch_one(&mut *tx)
            .await?;

        let id: i64 = sqlx::query_scalar(
            r#"
            insert into wms.wms_stocktake_h
                (stocktake_no, wh_id, loc_id, stocktake_date,
                 operator_id, doc_status, remark)
            values ($1,$2,$3,$4,$5,'DRAFT',$6)
            returning id
            "#,
        )
        .bind(&stocktake_no)
        .bind(cmd.wh_id)
        .bind(cmd.loc_id)
        .bind(cmd.stocktake_date)
        .bind(ctx.user_id)
        .bind(&cmd.remark)
        .fetch_one(&mut *tx)
        .await?;

        if cmd.snapshot_from_balance {
            // 从 inv.balance 快照该仓(位)所有非零余额为盘点行
            let mut qb = sqlx::QueryBuilder::<Postgres>::new(
                r#"
                insert into wms.wms_stocktake_d
                    (stocktake_id, line_no, material_id, batch_no, stock_status,
                     book_qty, unit, counted)
                select
                    
                "#,
            );
            qb.push_bind(id);
            qb.push(
                ", row_number() over (order by b.material_id, b.batch_no, b.stock_status)::int,",
            );
            qb.push(" b.material_id, b.batch_no, b.stock_status, b.book_qty, m.unit, false");
            qb.push(" from inv.balance b ");
            qb.push(" join mdm.mdm_material m on m.id = b.material_id");
            qb.push(" where b.wh_id = ").push_bind(cmd.wh_id);
            if let Some(loc) = cmd.loc_id {
                qb.push(" and b.loc_id = ").push_bind(loc);
            }
            qb.push(" and b.book_qty <> 0");
            qb.build().execute(&mut *tx).await?;
        } else {
            for l in &cmd.lines {
                sqlx::query(
                    r#"
                    insert into wms.wms_stocktake_d
                        (stocktake_id, line_no, material_id, batch_no, stock_status,
                         book_qty, unit, counted, note)
                    values ($1,$2,$3,$4,$5,$6,$7,false,$8)
                    "#,
                )
                .bind(id)
                .bind(l.line_no)
                .bind(l.material_id)
                .bind(&l.batch_no)
                .bind(&l.stock_status)
                .bind(l.book_qty)
                .bind(&l.unit)
                .bind(&l.note)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        self.get(id).await
    }

    async fn get(&self, id: i64) -> Result<StocktakeHeadView, AppError> {
        let head = sqlx::query(
            r#"
            select h.id, h.stocktake_no, h.wh_id, w.wh_code,
                   h.loc_id, l.loc_code, h.stocktake_date, h.operator_id,
                   h.doc_status, h.remark, h.created_at, h.updated_at
              from wms.wms_stocktake_h h
              left join mdm.mdm_warehouse w on w.id = h.wh_id
              left join mdm.mdm_location  l on l.id = h.loc_id
             where h.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found(format!("盘点单 id={id} 不存在")))?;

        let lines = sqlx::query(
            r#"
            select d.id, d.line_no, d.material_id, m.material_code,
                   d.batch_no, d.stock_status, d.book_qty, d.actual_qty,
                   case when d.actual_qty is not null
                        then d.actual_qty - d.book_qty
                        else null
                   end as diff_qty,
                   d.unit, d.counted, d.note
              from wms.wms_stocktake_d d
              left join mdm.mdm_material m on m.id = d.material_id
             where d.stocktake_id = $1
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

    async fn list(&self, q: &QueryStocktakes) -> Result<Vec<StocktakeHeadView>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select h.id, h.stocktake_no, h.wh_id, w.wh_code,
                   h.loc_id, l.loc_code, h.stocktake_date, h.operator_id,
                   h.doc_status, h.remark, h.created_at, h.updated_at
              from wms.wms_stocktake_h h
              left join mdm.mdm_warehouse w on w.id = h.wh_id
              left join mdm.mdm_location  l on l.id = h.loc_id
             where 1 = 1
            "#,
        );
        if let Some(no) = &q.stocktake_no {
            qb.push(" and h.stocktake_no = ").push_bind(no.clone());
        }
        if let Some(w) = q.wh_id {
            qb.push(" and h.wh_id = ").push_bind(w);
        }
        if let Some(s) = &q.doc_status {
            qb.push(" and h.doc_status = ").push_bind(s.clone());
        }
        if let Some(f) = q.date_from {
            qb.push(" and h.stocktake_date >= ").push_bind(f);
        }
        if let Some(t) = q.date_to {
            qb.push(" and h.stocktake_date <= ").push_bind(t);
        }
        qb.push(" order by h.stocktake_date desc, h.id desc limit 500");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| row_to_head(r, vec![])).collect())
    }

    async fn update_status(&self, id: i64, status: DocStatus) -> Result<(), AppError> {
        sqlx::query("update wms.wms_stocktake_h set doc_status = $1 where id = $2")
            .bind(status.as_str())
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_counts(&self, id: i64, lines: &[RecordCountLine]) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;
        for l in lines {
            sqlx::query(
                r#"
                update wms.wms_stocktake_d
                   set actual_qty = $1, counted = true,
                       note = coalesce($2, note)
                 where id = $3 and stocktake_id = $4
                "#,
            )
            .bind(l.actual_qty)
            .bind(&l.note)
            .bind(l.line_id)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

fn row_to_head(row: PgRow, lines: Vec<StocktakeLineView>) -> StocktakeHeadView {
    StocktakeHeadView {
        id: row.get("id"),
        stocktake_no: row.get("stocktake_no"),
        wh_id: row.get("wh_id"),
        wh_code: row.get("wh_code"),
        loc_id: row.get("loc_id"),
        loc_code: row.get("loc_code"),
        stocktake_date: row.get("stocktake_date"),
        operator_id: row.get("operator_id"),
        doc_status: row.get("doc_status"),
        remark: row.get("remark"),
        lines,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_line(row: PgRow) -> StocktakeLineView {
    let diff: Option<Decimal> = row.try_get("diff_qty").ok().flatten();
    StocktakeLineView {
        id: row.get("id"),
        line_no: row.get("line_no"),
        material_id: row.get("material_id"),
        material_code: row.get("material_code"),
        batch_no: row.get("batch_no"),
        stock_status: row.get("stock_status"),
        book_qty: row.get("book_qty"),
        actual_qty: row.get("actual_qty"),
        diff_qty: diff,
        unit: row.get("unit"),
        counted: row.get("counted"),
        note: row.get("note"),
    }
}
