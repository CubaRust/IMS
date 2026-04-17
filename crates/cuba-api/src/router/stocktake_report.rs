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
    AgingBucketRow, DormantRow, ExceptionSummaryRow, QueryAging, QueryDormant,
    QueryExceptionSummary, QueryTxnFlow, ReportingService, TxnFlowRow,
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
}

// -- stocktake --

async fn st_list(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryStocktakes>,
) -> Result<AppJson<Vec<StocktakeHeadView>>, AppError> {
    ctx.require_permission("stocktake.view")?;
    Ok(AppJson(StocktakeService::new(state.db().clone()).list(&q).await?))
}
async fn st_get(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<StocktakeHeadView>, AppError> {
    ctx.require_permission("stocktake.view")?;
    Ok(AppJson(StocktakeService::new(state.db().clone()).get(id).await?))
}
async fn st_create(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Json(cmd): Json<CreateStocktakeCommand>,
) -> Result<AppJson<StocktakeHeadView>, AppError> {
    ctx.require_permission("stocktake.create")?;
    Ok(AppJson(
        StocktakeService::new(state.db().clone()).create(&ctx, cmd).await?,
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
        .record_counts(&ctx, id, cmd).await?;
    Ok(AppJson(()))
}
async fn st_submit(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<SubmitStocktakeResult>, AppError> {
    ctx.require_permission("stocktake.submit")?;
    Ok(AppJson(
        StocktakeService::new(state.db().clone()).submit(&ctx, id).await?,
    ))
}
async fn st_void(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<()>, AppError> {
    ctx.require_permission("stocktake.void")?;
    StocktakeService::new(state.db().clone()).void(&ctx, id).await?;
    Ok(AppJson(()))
}

// -- reports --

async fn rpt_aging(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryAging>,
) -> Result<AppJson<Vec<AgingBucketRow>>, AppError> {
    ctx.require_permission("report.view")?;
    Ok(AppJson(ReportingService::new(state.db().clone()).aging(&q).await?))
}
async fn rpt_dormant(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryDormant>,
) -> Result<AppJson<Vec<DormantRow>>, AppError> {
    ctx.require_permission("report.view")?;
    Ok(AppJson(ReportingService::new(state.db().clone()).dormant(&q).await?))
}
async fn rpt_exception(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryExceptionSummary>,
) -> Result<AppJson<Vec<ExceptionSummaryRow>>, AppError> {
    ctx.require_permission("report.view")?;
    Ok(AppJson(ReportingService::new(state.db().clone()).exception_summary(&q).await?))
}
async fn rpt_txn_flow(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryTxnFlow>,
) -> Result<AppJson<Vec<TxnFlowRow>>, AppError> {
    ctx.require_permission("report.view")?;
    Ok(AppJson(ReportingService::new(state.db().clone()).txn_flow(&q).await?))
}
