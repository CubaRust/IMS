//! 异常先发 HTTP
//!
//! ```text
//! GET  /api/v1/preissues                 -- 列表
//! GET  /api/v1/preissues/:id             -- 详情
//! POST /api/v1/preissues                 -- 创建并产生 PREISSUE_PENDING 库存
//! POST /api/v1/preissues/:id/void        -- 作废(仅 PENDING)
//! ```

use axum::{
    extract::{Extension, Path, Query, State},
    routing::{get, post},
    Json, Router,
};

use cuba_bootstrap::AppState;
use cuba_preissue::{
    CreatePreissueCommand, PreissueHeadView, PreissueService, QueryPreissues,
    SubmitPreissueResult,
};
use cuba_shared::{audit::AuditContext, error::AppError};

use crate::response::AppJson;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/preissues", get(list).post(create))
        .route("/preissues/:id", get(detail))
        .route("/preissues/:id/void", post(void))
}

async fn list(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryPreissues>,
) -> Result<AppJson<Vec<PreissueHeadView>>, AppError> {
    ctx.require_permission("preissue.view")?;
    Ok(AppJson(PreissueService::new(state.db().clone()).list(&ctx, &q).await?))
}

async fn detail(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<PreissueHeadView>, AppError> {
    ctx.require_permission("preissue.view")?;
    Ok(AppJson(PreissueService::new(state.db().clone()).get(&ctx, id).await?))
}

async fn create(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Json(cmd): Json<CreatePreissueCommand>,
) -> Result<AppJson<SubmitPreissueResult>, AppError> {
    ctx.require_permission("preissue.create")?;
    Ok(AppJson(
        PreissueService::new(state.db().clone())
            .create_and_issue(&ctx, cmd)
            .await?,
    ))
}

async fn void(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<()>, AppError> {
    ctx.require_permission("preissue.close")?;
    PreissueService::new(state.db().clone()).void(&ctx, id).await?;
    Ok(AppJson(()))
}
