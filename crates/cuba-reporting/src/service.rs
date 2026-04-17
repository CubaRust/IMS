//! reporting service(只读)

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, PgPool, Postgres, Row};
use time::{Date, PrimitiveDateTime};

use cuba_shared::error::AppError;

// -- Rows --------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgingBucketRow {
    pub wh_code: String,
    pub material_code: String,
    pub material_name: Option<String>,
    pub stock_status: String,
    pub qty_0_30: Decimal,
    pub qty_31_60: Decimal,
    pub qty_61_90: Decimal,
    pub qty_91_180: Decimal,
    pub qty_over_180: Decimal,
    pub total_qty: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DormantRow {
    pub wh_code: String,
    pub material_code: String,
    pub material_name: Option<String>,
    pub stock_status: String,
    pub qty: Decimal,
    pub unit: String,
    pub last_txn_date: Option<Date>,
    pub dormant_days: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExceptionSummaryRow {
    pub exception_type: String,
    pub doc_count: i64,
    pub line_count: i64,
    pub total_qty: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxnFlowRow {
    pub txn_date: Date,
    pub txn_no: String,
    pub txn_type: String,
    pub scene_code: String,
    pub material_code: String,
    pub qty: Decimal,
    pub io_flag: String,
    pub wh_code: Option<String>,
    pub loc_code: Option<String>,
    pub stock_status: Option<String>,
    pub doc_type: String,
    pub doc_no: String,
    pub created_at: PrimitiveDateTime,
}

// -- Queries -----------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryAging {
    pub wh_id: Option<i64>,
    pub material_category: Option<String>,
    pub stock_status: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryDormant {
    pub wh_id: Option<i64>,
    pub min_days: Option<i32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryExceptionSummary {
    pub date_from: Option<Date>,
    pub date_to: Option<Date>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryTxnFlow {
    pub material_id: Option<i64>,
    pub wh_id: Option<i64>,
    pub doc_type: Option<String>,
    pub scene_code: Option<String>,
    pub date_from: Option<Date>,
    pub date_to: Option<Date>,
    pub limit: Option<i64>,
}

// -- Service -----------------------------------------------------------------

#[derive(Clone)]
pub struct ReportingService {
    pool: PgPool,
}

impl ReportingService {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 库龄报表(5 个桶)
    ///
    /// 依赖 `rpt.v_stock_aging`(0009 视图);若视图结构不一致,此查询会失败。
    /// 容错:我们用 `coalesce` 兜底,失败时返回空。
    pub async fn aging(&self, q: &QueryAging) -> Result<Vec<AgingBucketRow>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select wh_code, material_code, material_name, stock_status,
                   coalesce(qty_0_30, 0) as qty_0_30,
                   coalesce(qty_31_60, 0) as qty_31_60,
                   coalesce(qty_61_90, 0) as qty_61_90,
                   coalesce(qty_91_180, 0) as qty_91_180,
                   coalesce(qty_over_180, 0) as qty_over_180,
                   coalesce(total_qty, 0) as total_qty
              from rpt.v_stock_aging
             where 1 = 1
            "#,
        );
        if let Some(w) = q.wh_id {
            qb.push(" and wh_id = ").push_bind(w);
        }
        if let Some(c) = &q.material_category {
            qb.push(" and material_category = ").push_bind(c.clone());
        }
        if let Some(s) = &q.stock_status {
            qb.push(" and stock_status = ").push_bind(s.clone());
        }
        qb.push(" order by total_qty desc limit 1000");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_aging).collect())
    }

    pub async fn dormant(&self, q: &QueryDormant) -> Result<Vec<DormantRow>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select wh_code, material_code, material_name, stock_status,
                   qty, unit, last_txn_date, dormant_days
              from rpt.v_stock_dormant
             where 1 = 1
            "#,
        );
        if let Some(w) = q.wh_id { qb.push(" and wh_id = ").push_bind(w); }
        if let Some(d) = q.min_days { qb.push(" and dormant_days >= ").push_bind(d); }
        qb.push(" order by dormant_days desc limit 1000");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_dormant).collect())
    }

    pub async fn exception_summary(
        &self,
        q: &QueryExceptionSummary,
    ) -> Result<Vec<ExceptionSummaryRow>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select exception_type, doc_count, line_count, total_qty
              from rpt.v_exception_summary
             where 1 = 1
            "#,
        );
        if let Some(f) = q.date_from { qb.push(" and stat_date >= ").push_bind(f); }
        if let Some(t) = q.date_to { qb.push(" and stat_date <= ").push_bind(t); }
        qb.push(" order by total_qty desc");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_exc).collect())
    }

    pub async fn txn_flow(&self, q: &QueryTxnFlow) -> Result<Vec<TxnFlowRow>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select txn_date, txn_no, txn_type, scene_code,
                   material_code, qty, io_flag,
                   wh_code, loc_code, stock_status,
                   doc_type, doc_no, created_at
              from rpt.v_txn_flow
             where 1 = 1
            "#,
        );
        if let Some(m) = q.material_id { qb.push(" and material_id = ").push_bind(m); }
        if let Some(w) = q.wh_id { qb.push(" and wh_id = ").push_bind(w); }
        if let Some(d) = &q.doc_type { qb.push(" and doc_type = ").push_bind(d.clone()); }
        if let Some(s) = &q.scene_code { qb.push(" and scene_code = ").push_bind(s.clone()); }
        if let Some(f) = q.date_from { qb.push(" and txn_date >= ").push_bind(f); }
        if let Some(t) = q.date_to { qb.push(" and txn_date <= ").push_bind(t); }
        qb.push(" order by created_at desc limit ");
        qb.push_bind(q.limit.unwrap_or(500));
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_flow).collect())
    }
}

fn row_to_aging(row: PgRow) -> AgingBucketRow {
    AgingBucketRow {
        wh_code: row.get("wh_code"),
        material_code: row.get("material_code"),
        material_name: row.try_get("material_name").ok(),
        stock_status: row.get("stock_status"),
        qty_0_30: row.get("qty_0_30"),
        qty_31_60: row.get("qty_31_60"),
        qty_61_90: row.get("qty_61_90"),
        qty_91_180: row.get("qty_91_180"),
        qty_over_180: row.get("qty_over_180"),
        total_qty: row.get("total_qty"),
    }
}

fn row_to_dormant(row: PgRow) -> DormantRow {
    DormantRow {
        wh_code: row.get("wh_code"),
        material_code: row.get("material_code"),
        material_name: row.try_get("material_name").ok(),
        stock_status: row.get("stock_status"),
        qty: row.get("qty"),
        unit: row.get("unit"),
        last_txn_date: row.try_get("last_txn_date").ok().flatten(),
        dormant_days: row.get("dormant_days"),
    }
}

fn row_to_exc(row: PgRow) -> ExceptionSummaryRow {
    ExceptionSummaryRow {
        exception_type: row.get("exception_type"),
        doc_count: row.get("doc_count"),
        line_count: row.get("line_count"),
        total_qty: row.get("total_qty"),
    }
}

fn row_to_flow(row: PgRow) -> TxnFlowRow {
    TxnFlowRow {
        txn_date: row.get("txn_date"),
        txn_no: row.get("txn_no"),
        txn_type: row.get("txn_type"),
        scene_code: row.get("scene_code"),
        material_code: row.get("material_code"),
        qty: row.get("qty"),
        io_flag: row.get("io_flag"),
        wh_code: row.try_get("wh_code").ok(),
        loc_code: row.try_get("loc_code").ok(),
        stock_status: row.try_get("stock_status").ok(),
        doc_type: row.get("doc_type"),
        doc_no: row.get("doc_no"),
        created_at: row.get("created_at"),
    }
}
