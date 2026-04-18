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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryByMaterialRow {
    pub material_id: i64,
    pub material_code: String,
    pub material_name: Option<String>,
    pub material_category: Option<String>,
    pub process_type: Option<String>,
    pub brand: Option<String>,
    pub unit: Option<String>,
    pub safety_stock: Option<Decimal>,
    pub min_stock: Option<Decimal>,
    pub book_qty_total: Decimal,
    pub available_qty_total: Decimal,
    pub occupied_qty_total: Decimal,
    pub bad_qty_total: Decimal,
    pub scrap_qty_total: Decimal,
    pub pending_qty_total: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryByLocationRow {
    pub id: i64,
    pub material_code: String,
    pub material_name: Option<String>,
    pub process_type: Option<String>,
    pub brand: Option<String>,
    pub wh_code: String,
    pub wh_name: Option<String>,
    pub loc_code: String,
    pub loc_name: Option<String>,
    pub batch_no: Option<String>,
    pub stock_status: Option<String>,
    pub book_qty: Decimal,
    pub available_qty: Decimal,
    pub occupied_qty: Decimal,
    pub bad_qty: Decimal,
    pub scrap_qty: Decimal,
    pub pending_qty: Decimal,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LowStockWarningRow {
    pub material_id: i64,
    pub material_code: String,
    pub material_name: Option<String>,
    pub material_category: Option<String>,
    pub unit: Option<String>,
    pub safety_stock: Decimal,
    pub min_stock: Decimal,
    pub available_qty_total: Decimal,
    pub warning_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyTodoRow {
    pub anomaly_type: String,
    pub doc_id: i64,
    pub doc_no: String,
    pub event_date: Option<Date>,
    pub supplier_id: Option<i64>,
    pub work_order_no: Option<String>,
    pub reason: Option<String>,
    pub status: Option<String>,
    pub timeout_flag: Option<bool>,
    pub created_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodayIoRow {
    pub txn_id: i64,
    pub txn_no: String,
    pub txn_type: String,
    pub scene_code: String,
    pub doc_type: Option<String>,
    pub doc_no: Option<String>,
    pub material_id: i64,
    pub material_code: String,
    pub material_name: Option<String>,
    pub batch_no: Option<String>,
    pub qty: Decimal,
    pub unit: String,
    pub io_flag: String,
    pub source_wh_id: Option<i64>,
    pub target_wh_id: Option<i64>,
    pub operator_id: Option<i64>,
    pub operate_time: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefectStats30dRow {
    pub material_code: String,
    pub material_name: Option<String>,
    pub defect_source: Option<String>,
    pub product_stage: Option<String>,
    pub process_method: Option<String>,
    pub line_count: i64,
    pub total_qty: Decimal,
    pub last_found_date: Option<Date>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutsourceInTransitRow {
    pub outsource_id: i64,
    pub outsource_no: String,
    pub supplier_code: Option<String>,
    pub supplier_name: Option<String>,
    pub work_order_no: Option<String>,
    pub process_name: Option<String>,
    pub send_date: Option<Date>,
    pub expect_back_date: Option<Date>,
    pub doc_status: String,
    pub total_sent_qty: Decimal,
    pub total_received_qty: Decimal,
    pub in_transit_qty: Decimal,
}

/// 首页看板聚合数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub today_inbound_count: i64,
    pub today_outbound_count: i64,
    pub today_inbound_qty: Decimal,
    pub today_outbound_qty: Decimal,
    pub low_stock_warning_count: i64,
    pub anomaly_todo_count: i64,
    pub outsource_in_transit_count: i64,
    pub defect_pending_count: i64,
    pub total_material_count: i64,
    pub total_sku_with_stock: i64,
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

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryInventoryByMaterial {
    pub material_category: Option<String>,
    pub process_type: Option<String>,
    pub material_code: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryInventoryByLocation {
    pub wh_code: Option<String>,
    pub material_code: Option<String>,
    pub stock_status: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryLowStockWarning {
    pub warning_level: Option<String>,
    pub material_category: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryAnomalyTodo {
    pub anomaly_type: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryTodayIo {
    pub txn_type: Option<String>,
    pub scene_code: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryDefectStats30d {
    pub material_code: Option<String>,
    pub defect_source: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryOutsourceInTransit {
    pub supplier_code: Option<String>,
    pub doc_status: Option<String>,
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
        if let Some(w) = q.wh_id {
            qb.push(" and wh_id = ").push_bind(w);
        }
        if let Some(d) = q.min_days {
            qb.push(" and dormant_days >= ").push_bind(d);
        }
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
        if let Some(f) = q.date_from {
            qb.push(" and stat_date >= ").push_bind(f);
        }
        if let Some(t) = q.date_to {
            qb.push(" and stat_date <= ").push_bind(t);
        }
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
        if let Some(m) = q.material_id {
            qb.push(" and material_id = ").push_bind(m);
        }
        if let Some(w) = q.wh_id {
            qb.push(" and wh_id = ").push_bind(w);
        }
        if let Some(d) = &q.doc_type {
            qb.push(" and doc_type = ").push_bind(d.clone());
        }
        if let Some(s) = &q.scene_code {
            qb.push(" and scene_code = ").push_bind(s.clone());
        }
        if let Some(f) = q.date_from {
            qb.push(" and txn_date >= ").push_bind(f);
        }
        if let Some(t) = q.date_to {
            qb.push(" and txn_date <= ").push_bind(t);
        }
        qb.push(" order by created_at desc limit ");
        qb.push_bind(q.limit.unwrap_or(500));
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_flow).collect())
    }

    pub async fn inventory_by_material(
        &self,
        q: &QueryInventoryByMaterial,
    ) -> Result<Vec<InventoryByMaterialRow>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select material_id, material_code, material_name, material_category,
                   process_type, brand, unit, safety_stock, min_stock,
                   book_qty_total, available_qty_total, occupied_qty_total,
                   bad_qty_total, scrap_qty_total, pending_qty_total
              from rpt.v_inventory_by_material
             where 1 = 1
            "#,
        );
        if let Some(c) = &q.material_category {
            qb.push(" and material_category = ").push_bind(c.clone());
        }
        if let Some(p) = &q.process_type {
            qb.push(" and process_type = ").push_bind(p.clone());
        }
        if let Some(m) = &q.material_code {
            qb.push(" and material_code ilike ")
                .push_bind(format!("%{m}%"));
        }
        qb.push(" order by material_code limit 1000");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_inv_material).collect())
    }

    pub async fn inventory_by_location(
        &self,
        q: &QueryInventoryByLocation,
    ) -> Result<Vec<InventoryByLocationRow>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select id, material_code, material_name, process_type, brand,
                   wh_code, wh_name, loc_code, loc_name, batch_no,
                   stock_status, book_qty, available_qty, occupied_qty,
                   bad_qty, scrap_qty, pending_qty, updated_at
              from rpt.v_inventory_by_location
             where 1 = 1
            "#,
        );
        if let Some(w) = &q.wh_code {
            qb.push(" and wh_code = ").push_bind(w.clone());
        }
        if let Some(m) = &q.material_code {
            qb.push(" and material_code ilike ")
                .push_bind(format!("%{m}%"));
        }
        if let Some(s) = &q.stock_status {
            qb.push(" and stock_status = ").push_bind(s.clone());
        }
        qb.push(" order by wh_code, loc_code, material_code limit 1000");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_inv_location).collect())
    }

    pub async fn low_stock_warning(
        &self,
        q: &QueryLowStockWarning,
    ) -> Result<Vec<LowStockWarningRow>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select material_id, material_code, material_name, material_category,
                   unit, safety_stock, min_stock, available_qty_total, warning_level
              from rpt.v_low_stock_warning
             where 1 = 1
            "#,
        );
        if let Some(l) = &q.warning_level {
            qb.push(" and warning_level = ").push_bind(l.clone());
        }
        if let Some(c) = &q.material_category {
            qb.push(" and material_category = ").push_bind(c.clone());
        }
        qb.push(" order by warning_level, material_code limit 1000");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_low_stock).collect())
    }

    pub async fn anomaly_todo(
        &self,
        q: &QueryAnomalyTodo,
    ) -> Result<Vec<AnomalyTodoRow>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select anomaly_type, doc_id, doc_no, event_date,
                   supplier_id, work_order_no, reason, status,
                   timeout_flag, created_at
              from rpt.v_anomaly_todo
             where 1 = 1
            "#,
        );
        if let Some(t) = &q.anomaly_type {
            qb.push(" and anomaly_type = ").push_bind(t.clone());
        }
        qb.push(" order by created_at desc limit 500");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_anomaly).collect())
    }

    pub async fn today_io(&self, q: &QueryTodayIo) -> Result<Vec<TodayIoRow>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select txn_id, txn_no, txn_type, scene_code, doc_type, doc_no,
                   material_id, material_code, material_name, batch_no,
                   qty, unit, io_flag, source_wh_id, target_wh_id,
                   operator_id, operate_time
              from rpt.v_today_io
             where 1 = 1
            "#,
        );
        if let Some(t) = &q.txn_type {
            qb.push(" and txn_type = ").push_bind(t.clone());
        }
        if let Some(s) = &q.scene_code {
            qb.push(" and scene_code = ").push_bind(s.clone());
        }
        qb.push(" order by operate_time desc limit 1000");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_today_io).collect())
    }

    pub async fn defect_stats_30d(
        &self,
        q: &QueryDefectStats30d,
    ) -> Result<Vec<DefectStats30dRow>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select material_code, material_name, defect_source, product_stage,
                   process_method, line_count, total_qty, last_found_date
              from rpt.v_defect_stats_30d
             where 1 = 1
            "#,
        );
        if let Some(m) = &q.material_code {
            qb.push(" and material_code ilike ")
                .push_bind(format!("%{m}%"));
        }
        if let Some(s) = &q.defect_source {
            qb.push(" and defect_source = ").push_bind(s.clone());
        }
        qb.push(" order by total_qty desc limit 500");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_defect_stats).collect())
    }

    pub async fn outsource_in_transit(
        &self,
        q: &QueryOutsourceInTransit,
    ) -> Result<Vec<OutsourceInTransitRow>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select outsource_id, outsource_no, supplier_code, supplier_name,
                   work_order_no, process_name, send_date, expect_back_date,
                   doc_status, total_sent_qty, total_received_qty, in_transit_qty
              from rpt.v_outsource_in_transit
             where 1 = 1
            "#,
        );
        if let Some(s) = &q.supplier_code {
            qb.push(" and supplier_code = ").push_bind(s.clone());
        }
        if let Some(d) = &q.doc_status {
            qb.push(" and doc_status = ").push_bind(d.clone());
        }
        qb.push(" order by send_date desc limit 500");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_outsource).collect())
    }

    /// 首页看板:一次查询聚合多个指标
    pub async fn dashboard(&self) -> Result<DashboardData, AppError> {
        let row = sqlx::query(
            r#"
            select
                coalesce((select count(*) from rpt.v_today_io where io_flag = 'IN'), 0) as today_in_count,
                coalesce((select count(*) from rpt.v_today_io where io_flag = 'OUT'), 0) as today_out_count,
                coalesce((select sum(qty) from rpt.v_today_io where io_flag = 'IN'), 0) as today_in_qty,
                coalesce((select sum(qty) from rpt.v_today_io where io_flag = 'OUT'), 0) as today_out_qty,
                coalesce((select count(*) from rpt.v_low_stock_warning), 0) as low_stock_count,
                coalesce((select count(*) from rpt.v_anomaly_todo), 0) as anomaly_count,
                coalesce((select count(*) from rpt.v_outsource_in_transit), 0) as outsource_count,
                coalesce((select count(*) from wms.wms_defect_h where doc_status in ('DRAFT','SUBMITTED')), 0) as defect_count,
                coalesce((select count(*) from mdm.mdm_material where is_active = true), 0) as material_count,
                coalesce((select count(distinct material_id) from wms.wms_inventory_balance where book_qty > 0), 0) as sku_with_stock
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(DashboardData {
            today_inbound_count: row.get("today_in_count"),
            today_outbound_count: row.get("today_out_count"),
            today_inbound_qty: row.get("today_in_qty"),
            today_outbound_qty: row.get("today_out_qty"),
            low_stock_warning_count: row.get("low_stock_count"),
            anomaly_todo_count: row.get("anomaly_count"),
            outsource_in_transit_count: row.get("outsource_count"),
            defect_pending_count: row.get("defect_count"),
            total_material_count: row.get("material_count"),
            total_sku_with_stock: row.get("sku_with_stock"),
        })
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

fn row_to_inv_material(row: PgRow) -> InventoryByMaterialRow {
    InventoryByMaterialRow {
        material_id: row.get("material_id"),
        material_code: row.get("material_code"),
        material_name: row.try_get("material_name").ok(),
        material_category: row.try_get("material_category").ok(),
        process_type: row.try_get("process_type").ok(),
        brand: row.try_get("brand").ok(),
        unit: row.try_get("unit").ok(),
        safety_stock: row.try_get("safety_stock").ok(),
        min_stock: row.try_get("min_stock").ok(),
        book_qty_total: row.get("book_qty_total"),
        available_qty_total: row.get("available_qty_total"),
        occupied_qty_total: row.get("occupied_qty_total"),
        bad_qty_total: row.get("bad_qty_total"),
        scrap_qty_total: row.get("scrap_qty_total"),
        pending_qty_total: row.get("pending_qty_total"),
    }
}

fn row_to_inv_location(row: PgRow) -> InventoryByLocationRow {
    InventoryByLocationRow {
        id: row.get("id"),
        material_code: row.get("material_code"),
        material_name: row.try_get("material_name").ok(),
        process_type: row.try_get("process_type").ok(),
        brand: row.try_get("brand").ok(),
        wh_code: row.get("wh_code"),
        wh_name: row.try_get("wh_name").ok(),
        loc_code: row.get("loc_code"),
        loc_name: row.try_get("loc_name").ok(),
        batch_no: row.try_get("batch_no").ok(),
        stock_status: row.try_get("stock_status").ok(),
        book_qty: row.get("book_qty"),
        available_qty: row.get("available_qty"),
        occupied_qty: row.get("occupied_qty"),
        bad_qty: row.get("bad_qty"),
        scrap_qty: row.get("scrap_qty"),
        pending_qty: row.get("pending_qty"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_low_stock(row: PgRow) -> LowStockWarningRow {
    LowStockWarningRow {
        material_id: row.get("material_id"),
        material_code: row.get("material_code"),
        material_name: row.try_get("material_name").ok(),
        material_category: row.try_get("material_category").ok(),
        unit: row.try_get("unit").ok(),
        safety_stock: row.get("safety_stock"),
        min_stock: row.get("min_stock"),
        available_qty_total: row.get("available_qty_total"),
        warning_level: row.get("warning_level"),
    }
}

fn row_to_anomaly(row: PgRow) -> AnomalyTodoRow {
    AnomalyTodoRow {
        anomaly_type: row.get("anomaly_type"),
        doc_id: row.get("doc_id"),
        doc_no: row.get("doc_no"),
        event_date: row.try_get("event_date").ok().flatten(),
        supplier_id: row.try_get("supplier_id").ok(),
        work_order_no: row.try_get("work_order_no").ok(),
        reason: row.try_get("reason").ok(),
        status: row.try_get("status").ok(),
        timeout_flag: row.try_get("timeout_flag").ok(),
        created_at: row.get("created_at"),
    }
}

fn row_to_today_io(row: PgRow) -> TodayIoRow {
    TodayIoRow {
        txn_id: row.get("txn_id"),
        txn_no: row.get("txn_no"),
        txn_type: row.get("txn_type"),
        scene_code: row.get("scene_code"),
        doc_type: row.try_get("doc_type").ok(),
        doc_no: row.try_get("doc_no").ok(),
        material_id: row.get("material_id"),
        material_code: row.get("material_code"),
        material_name: row.try_get("material_name").ok(),
        batch_no: row.try_get("batch_no").ok(),
        qty: row.get("qty"),
        unit: row.get("unit"),
        io_flag: row.get("io_flag"),
        source_wh_id: row.try_get("source_wh_id").ok(),
        target_wh_id: row.try_get("target_wh_id").ok(),
        operator_id: row.try_get("operator_id").ok(),
        operate_time: row.get("operate_time"),
    }
}

fn row_to_defect_stats(row: PgRow) -> DefectStats30dRow {
    DefectStats30dRow {
        material_code: row.get("material_code"),
        material_name: row.try_get("material_name").ok(),
        defect_source: row.try_get("defect_source").ok(),
        product_stage: row.try_get("product_stage").ok(),
        process_method: row.try_get("process_method").ok(),
        line_count: row.get("line_count"),
        total_qty: row.get("total_qty"),
        last_found_date: row.try_get("last_found_date").ok().flatten(),
    }
}

fn row_to_outsource(row: PgRow) -> OutsourceInTransitRow {
    OutsourceInTransitRow {
        outsource_id: row.get("outsource_id"),
        outsource_no: row.get("outsource_no"),
        supplier_code: row.try_get("supplier_code").ok(),
        supplier_name: row.try_get("supplier_name").ok(),
        work_order_no: row.try_get("work_order_no").ok(),
        process_name: row.try_get("process_name").ok(),
        send_date: row.try_get("send_date").ok().flatten(),
        expect_back_date: row.try_get("expect_back_date").ok().flatten(),
        doc_status: row.get("doc_status"),
        total_sent_qty: row.get("total_sent_qty"),
        total_received_qty: row.get("total_received_qty"),
        in_transit_qty: row.get("in_transit_qty"),
    }
}
