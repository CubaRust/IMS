//! 盘点 + 报表 HTTP
//!
//! ```text
//! POST/GET  /api/v1/stocktakes[/:id]
//! POST      /api/v1/stocktakes/:id/counts
//! POST      /api/v1/stocktakes/:id/submit|/void
//!
//! GET /api/v1/reports/aging
//! GET /api/v1/reports/dormant
//! GET /api/v1/reports/exception-summary
//! GET /api/v1/reports/txn-flow
//! ```

use axum::{
    extract::{Extension, Path, Query, State},
    routing::{get, post},
    Json, Router,
};

use cuba_bootstrap::AppState;
use cuba_reporting::{
    AgingBucketRow, AnomalyTodoRow, DashboardData, DefectStats30dRow, DormantRow,
    ExceptionSummaryRow, InventoryByLocationRow, InventoryByMaterialRow, LowStockWarningRow,
    OutsourceInTransitRow, QueryAging, QueryAnomalyTodo, QueryDefectStats30d, QueryDormant,
    QueryExceptionSummary, QueryInventoryByLocation, QueryInventoryByMaterial,
    QueryLowStockWarning, QueryOutsourceInTransit, QueryTodayIo, QueryTxnFlow, ReportingService,
    TodayIoRow, TxnFlowRow,
};
use cuba_shared::{audit::AuditContext, error::AppError};
use cuba_stocktake::{
    CreateStocktakeCommand, QueryStocktakes, RecordCountCommand, StocktakeHeadView,
    StocktakeService, SubmitStocktakeResult,
};

use crate::response::AppJson;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/stocktakes", get(st_list).post(st_create))
        .route("/stocktakes/:id", get(st_get))
        .route("/stocktakes/:id/counts", post(st_count))
        .route("/stocktakes/:id/submit", post(st_submit))
        .route("/stocktakes/:id/void", post(st_void))
        // reports
        .route("/reports/aging", get(rpt_aging))
        .route("/reports/dormant", get(rpt_dormant))
        .route("/reports/exception-summary", get(rpt_exception))
        .route("/reports/txn-flow", get(rpt_txn_flow))
        .route("/reports/inventory-by-material", get(rpt_inv_material))
        .route("/reports/inventory-by-location", get(rpt_inv_location))
        .route("/reports/low-stock-warning", get(rpt_low_stock))
        .route("/reports/anomaly-todo", get(rpt_anomaly))
        .route("/reports/today-io", get(rpt_today_io))
        .route("/reports/defect-stats", get(rpt_defect_stats))
        .route("/reports/outsource-in-transit", get(rpt_outsource))
        .route("/dashboard", get(dashboard))
}

// -- stocktake --

async fn st_list(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryStocktakes>,
) -> Result<AppJson<Vec<StocktakeHeadView>>, AppError> {
    ctx.require_permission("stocktake.view")?;
    Ok(AppJson(
        StocktakeService::new(state.db_read().clone())
            .list(&q)
            .await?,
    ))
}
async fn st_get(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<StocktakeHeadView>, AppError> {
    ctx.require_permission("stocktake.view")?;
    Ok(AppJson(
        StocktakeService::new(state.db_read().clone())
            .get(id)
            .await?,
    ))
}
async fn st_create(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Json(cmd): Json<CreateStocktakeCommand>,
) -> Result<AppJson<StocktakeHeadView>, AppError> {
    ctx.require_permission("stocktake.create")?;
    Ok(AppJson(
        StocktakeService::new(state.db().clone())
            .create(&ctx, cmd)
            .await?,
    ))
}
async fn st_count(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
    Json(cmd): Json<RecordCountCommand>,
) -> Result<AppJson<()>, AppError> {
    ctx.require_permission("stocktake.count")?;
    StocktakeService::new(state.db().clone())
        .record_counts(&ctx, id, cmd)
        .await?;
    Ok(AppJson(()))
}
async fn st_submit(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<SubmitStocktakeResult>, AppError> {
    ctx.require_permission("stocktake.submit")?;
    Ok(AppJson(
        StocktakeService::new(state.db().clone())
            .submit(&ctx, id)
            .await?,
    ))
}
async fn st_void(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<()>, AppError> {
    ctx.require_permission("stocktake.void")?;
    StocktakeService::new(state.db().clone())
        .void(&ctx, id)
        .await?;
    Ok(AppJson(()))
}

// -- reports --

async fn rpt_aging(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryAging>,
) -> Result<AppJson<Vec<AgingBucketRow>>, AppError> {
    ctx.require_permission("report.view")?;
    Ok(AppJson(
        ReportingService::new(state.db_read().clone())
            .aging(&q)
            .await?,
    ))
}
async fn rpt_dormant(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryDormant>,
) -> Result<AppJson<Vec<DormantRow>>, AppError> {
    ctx.require_permission("report.view")?;
    Ok(AppJson(
        ReportingService::new(state.db_read().clone())
            .dormant(&q)
            .await?,
    ))
}
async fn rpt_exception(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryExceptionSummary>,
) -> Result<AppJson<Vec<ExceptionSummaryRow>>, AppError> {
    ctx.require_permission("report.view")?;
    Ok(AppJson(
        ReportingService::new(state.db_read().clone())
            .exception_summary(&q)
            .await?,
    ))
}
async fn rpt_txn_flow(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryTxnFlow>,
) -> Result<AppJson<Vec<TxnFlowRow>>, AppError> {
    ctx.require_permission("report.view")?;
    Ok(AppJson(
        ReportingService::new(state.db_read().clone())
            .txn_flow(&q)
            .await?,
    ))
}

async fn rpt_inv_material(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryInventoryByMaterial>,
) -> Result<AppJson<Vec<InventoryByMaterialRow>>, AppError> {
    ctx.require_permission("report.view")?;
    Ok(AppJson(
        ReportingService::new(state.db_read().clone())
            .inventory_by_material(&q)
            .await?,
    ))
}

async fn rpt_inv_location(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryInventoryByLocation>,
) -> Result<AppJson<Vec<InventoryByLocationRow>>, AppError> {
    ctx.require_permission("report.view")?;
    Ok(AppJson(
        ReportingService::new(state.db_read().clone())
            .inventory_by_location(&q)
            .await?,
    ))
}

async fn rpt_low_stock(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryLowStockWarning>,
) -> Result<AppJson<Vec<LowStockWarningRow>>, AppError> {
    ctx.require_permission("report.view")?;
    Ok(AppJson(
        ReportingService::new(state.db_read().clone())
            .low_stock_warning(&q)
            .await?,
    ))
}

async fn rpt_anomaly(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryAnomalyTodo>,
) -> Result<AppJson<Vec<AnomalyTodoRow>>, AppError> {
    ctx.require_permission("report.view")?;
    Ok(AppJson(
        ReportingService::new(state.db_read().clone())
            .anomaly_todo(&q)
            .await?,
    ))
}

async fn rpt_today_io(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryTodayIo>,
) -> Result<AppJson<Vec<TodayIoRow>>, AppError> {
    ctx.require_permission("report.view")?;
    Ok(AppJson(
        ReportingService::new(state.db_read().clone())
            .today_io(&q)
            .await?,
    ))
}

async fn rpt_defect_stats(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryDefectStats30d>,
) -> Result<AppJson<Vec<DefectStats30dRow>>, AppError> {
    ctx.require_permission("report.view")?;
    Ok(AppJson(
        ReportingService::new(state.db_read().clone())
            .defect_stats_30d(&q)
            .await?,
    ))
}

async fn rpt_outsource(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryOutsourceInTransit>,
) -> Result<AppJson<Vec<OutsourceInTransitRow>>, AppError> {
    ctx.require_permission("report.view")?;
    Ok(AppJson(
        ReportingService::new(state.db_read().clone())
            .outsource_in_transit(&q)
            .await?,
    ))
}

async fn dashboard(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
) -> Result<AppJson<DashboardData>, AppError> {
    ctx.require_permission("report.view")?;
    Ok(AppJson(
        ReportingService::new(state.db_read().clone())
            .dashboard()
            .await?,
    ))
}
