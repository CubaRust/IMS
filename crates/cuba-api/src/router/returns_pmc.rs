//! 客户退货 / 退供 / 委外 HTTP 路由

use axum::{
    extract::{Extension, Path, Query, State},
    routing::{get, post},
    Json, Router,
};

use cuba_bootstrap::AppState;
use cuba_customer_return::{
    CreateCustomerReturnCommand, CustomerReturnHeadView, CustomerReturnService, JudgeLineCommand,
    QueryCustomerReturns, SubmitCustomerReturnResult,
};
use cuba_pmc::{
    CreateOutsourceCommand, OutsourceHeadView, PmcService, QueryOutsources, SubmitBackCommand,
    SubmitResult as PmcSubmitResult,
};
use cuba_shared::{audit::AuditContext, error::AppError};
use cuba_supplier_return::{
    CreateSupplierReturnCommand, QuerySupplierReturns, SubmitSupplierReturnResult,
    SupplierReturnHeadView, SupplierReturnService,
};

use crate::response::AppJson;

pub fn routes() -> Router<AppState> {
    Router::new()
        // customer returns
        .route("/customer-returns", get(cr_list).post(cr_create))
        .route("/customer-returns/:id", get(cr_get))
        .route("/customer-returns/:id/judge", post(cr_judge))
        .route("/customer-returns/:id/submit", post(cr_submit))
        .route("/customer-returns/:id/void", post(cr_void))
        // supplier returns
        .route("/supplier-returns", get(sr_list).post(sr_create))
        .route("/supplier-returns/:id", get(sr_get))
        .route("/supplier-returns/:id/submit", post(sr_submit))
        .route("/supplier-returns/:id/void", post(sr_void))
        // outsource (PMC)
        .route("/outsources", get(os_list).post(os_create))
        .route("/outsources/:id", get(os_get))
        .route("/outsources/:id/send", post(os_send))
        .route("/outsources/:id/back", post(os_back))
        .route("/outsources/:id/void", post(os_void))
}

// -- customer returns --

async fn cr_list(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryCustomerReturns>,
) -> Result<AppJson<Vec<CustomerReturnHeadView>>, AppError> {
    ctx.require_permission("customer_return.view")?;
    Ok(AppJson(CustomerReturnService::new(state.db_read().clone()).list(&q).await?))
}
async fn cr_get(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<CustomerReturnHeadView>, AppError> {
    ctx.require_permission("customer_return.view")?;
    Ok(AppJson(CustomerReturnService::new(state.db_read().clone()).get(id).await?))
}
async fn cr_create(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Json(cmd): Json<CreateCustomerReturnCommand>,
) -> Result<AppJson<CustomerReturnHeadView>, AppError> {
    ctx.require_permission("customer_return.create")?;
    Ok(AppJson(
        CustomerReturnService::new(state.db().clone()).create(&ctx, cmd).await?,
    ))
}
async fn cr_judge(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
    Json(lines): Json<Vec<JudgeLineCommand>>,
) -> Result<AppJson<()>, AppError> {
    ctx.require_permission("customer_return.judge")?;
    CustomerReturnService::new(state.db().clone())
        .judge(&ctx, id, lines)
        .await?;
    Ok(AppJson(()))
}
async fn cr_submit(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<SubmitCustomerReturnResult>, AppError> {
    ctx.require_permission("customer_return.submit")?;
    Ok(AppJson(
        CustomerReturnService::new(state.db().clone()).submit(&ctx, id).await?,
    ))
}
async fn cr_void(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<()>, AppError> {
    ctx.require_permission("customer_return.void")?;
    CustomerReturnService::new(state.db().clone()).void(&ctx, id).await?;
    Ok(AppJson(()))
}

// -- supplier returns --

async fn sr_list(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QuerySupplierReturns>,
) -> Result<AppJson<Vec<SupplierReturnHeadView>>, AppError> {
    ctx.require_permission("supplier_return.view")?;
    Ok(AppJson(SupplierReturnService::new(state.db_read().clone()).list(&q).await?))
}
async fn sr_get(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<SupplierReturnHeadView>, AppError> {
    ctx.require_permission("supplier_return.view")?;
    Ok(AppJson(SupplierReturnService::new(state.db_read().clone()).get(id).await?))
}
async fn sr_create(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Json(cmd): Json<CreateSupplierReturnCommand>,
) -> Result<AppJson<SupplierReturnHeadView>, AppError> {
    ctx.require_permission("supplier_return.create")?;
    Ok(AppJson(
        SupplierReturnService::new(state.db().clone()).create(&ctx, cmd).await?,
    ))
}
async fn sr_submit(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<SubmitSupplierReturnResult>, AppError> {
    ctx.require_permission("supplier_return.submit")?;
    Ok(AppJson(
        SupplierReturnService::new(state.db().clone()).submit(&ctx, id).await?,
    ))
}
async fn sr_void(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<()>, AppError> {
    ctx.require_permission("supplier_return.void")?;
    SupplierReturnService::new(state.db().clone()).void(&ctx, id).await?;
    Ok(AppJson(()))
}

// -- outsource --

async fn os_list(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryOutsources>,
) -> Result<AppJson<Vec<OutsourceHeadView>>, AppError> {
    ctx.require_permission("pmc.outsource.view")?;
    Ok(AppJson(PmcService::new(state.db_read().clone()).list(&q).await?))
}
async fn os_get(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<OutsourceHeadView>, AppError> {
    ctx.require_permission("pmc.outsource.view")?;
    Ok(AppJson(PmcService::new(state.db_read().clone()).get(id).await?))
}
async fn os_create(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Json(cmd): Json<CreateOutsourceCommand>,
) -> Result<AppJson<OutsourceHeadView>, AppError> {
    ctx.require_permission("pmc.outsource.create")?;
    Ok(AppJson(
        PmcService::new(state.db().clone()).create(&ctx, cmd).await?,
    ))
}
async fn os_send(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<PmcSubmitResult>, AppError> {
    ctx.require_permission("pmc.outsource.send")?;
    Ok(AppJson(
        PmcService::new(state.db().clone()).submit_send(&ctx, id).await?,
    ))
}
async fn os_back(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
    Json(cmd): Json<SubmitBackCommand>,
) -> Result<AppJson<PmcSubmitResult>, AppError> {
    ctx.require_permission("pmc.outsource.back")?;
    Ok(AppJson(
        PmcService::new(state.db().clone()).submit_back(&ctx, id, cmd).await?,
    ))
}
async fn os_void(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<()>, AppError> {
    ctx.require_permission("pmc.outsource.void")?;
    PmcService::new(state.db().clone()).void(&ctx, id).await?;
    Ok(AppJson(()))
}
