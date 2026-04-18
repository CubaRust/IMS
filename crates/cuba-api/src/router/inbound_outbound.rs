//! 入库单 + 出库单 HTTP 路由
//!
//! ```text
//! GET/POST     /api/v1/inbounds
//! GET          /api/v1/inbounds/:id
//! POST         /api/v1/inbounds/:id/submit
//! POST         /api/v1/inbounds/:id/void
//!
//! GET/POST     /api/v1/outbounds
//! GET          /api/v1/outbounds/:id
//! POST         /api/v1/outbounds/:id/submit
//! POST         /api/v1/outbounds/:id/void
//! ```

use axum::{
    extract::{Extension, Path, Query, State},
    routing::{get, post},
    Json, Router,
};

use cuba_bootstrap::AppState;
use cuba_inbound::{
    CreateInboundCommand, InboundHeadView, InboundService, QueryInbounds, SubmitInboundResult,
};
use cuba_outbound::{
    CreateOutboundCommand, OutboundHeadView, OutboundService, QueryOutbounds, SubmitOutboundResult,
};
use cuba_shared::{audit::AuditContext, error::AppError};

use crate::response::AppJson;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/inbounds", get(list_inbounds).post(create_inbound))
        .route("/inbounds/:id", get(get_inbound))
        .route("/inbounds/:id/submit", post(submit_inbound))
        .route("/inbounds/:id/void", post(void_inbound))
        .route("/outbounds", get(list_outbounds).post(create_outbound))
        .route("/outbounds/:id", get(get_outbound))
        .route("/outbounds/:id/submit", post(submit_outbound))
        .route("/outbounds/:id/void", post(void_outbound))
}

// -- inbound --

async fn list_inbounds(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryInbounds>,
) -> Result<AppJson<Vec<InboundHeadView>>, AppError> {
    ctx.require_permission("inbound.view")?;
    Ok(AppJson(
        InboundService::new(state.db_read().clone())
            .list(&ctx, &q)
            .await?,
    ))
}

async fn get_inbound(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<InboundHeadView>, AppError> {
    ctx.require_permission("inbound.view")?;
    Ok(AppJson(
        InboundService::new(state.db_read().clone())
            .get(&ctx, id)
            .await?,
    ))
}

async fn create_inbound(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Json(cmd): Json<CreateInboundCommand>,
) -> Result<AppJson<InboundHeadView>, AppError> {
    ctx.require_permission("inbound.create")?;
    Ok(AppJson(
        InboundService::new(state.db().clone())
            .create(&ctx, cmd)
            .await?,
    ))
}

async fn submit_inbound(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<SubmitInboundResult>, AppError> {
    ctx.require_permission("inbound.submit")?;
    Ok(AppJson(
        InboundService::new(state.db().clone())
            .submit(&ctx, id)
            .await?,
    ))
}

async fn void_inbound(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<()>, AppError> {
    ctx.require_permission("inbound.void")?;
    InboundService::new(state.db().clone())
        .void(&ctx, id)
        .await?;
    Ok(AppJson(()))
}

// -- outbound --

async fn list_outbounds(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryOutbounds>,
) -> Result<AppJson<Vec<OutboundHeadView>>, AppError> {
    ctx.require_permission("outbound.view")?;
    Ok(AppJson(
        OutboundService::new(state.db_read().clone())
            .list(&ctx, &q)
            .await?,
    ))
}

async fn get_outbound(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<OutboundHeadView>, AppError> {
    ctx.require_permission("outbound.view")?;
    Ok(AppJson(
        OutboundService::new(state.db_read().clone())
            .get(&ctx, id)
            .await?,
    ))
}

async fn create_outbound(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Json(cmd): Json<CreateOutboundCommand>,
) -> Result<AppJson<OutboundHeadView>, AppError> {
    ctx.require_permission("outbound.create")?;
    Ok(AppJson(
        OutboundService::new(state.db().clone())
            .create(&ctx, cmd)
            .await?,
    ))
}

async fn submit_outbound(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<SubmitOutboundResult>, AppError> {
    ctx.require_permission("outbound.submit")?;
    Ok(AppJson(
        OutboundService::new(state.db().clone())
            .submit(&ctx, id)
            .await?,
    ))
}

async fn void_outbound(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<()>, AppError> {
    ctx.require_permission("outbound.void")?;
    OutboundService::new(state.db().clone())
        .void(&ctx, id)
        .await?;
    Ok(AppJson(()))
}
