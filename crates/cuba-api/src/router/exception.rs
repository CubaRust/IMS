//! 不良 / 报废 / 拆解回收 HTTP
//!
//! ```text
//! POST/GET     /api/v1/defects
//! GET/POST     /api/v1/defects/:id[/submit|/void]
//!
//! POST/GET     /api/v1/scraps
//! GET/POST     /api/v1/scraps/:id[/submit|/void]
//!
//! POST/GET     /api/v1/recoveries
//! GET/POST     /api/v1/recoveries/:id[/submit|/void]
//! ```

use axum::{
    extract::{Extension, Path, Query, State},
    routing::{get, post},
    Json, Router,
};

use cuba_bootstrap::AppState;
use cuba_defect::{
    CreateDefectCommand, DefectHeadView, DefectService, QueryDefects, SubmitDefectResult,
};
use cuba_recovery::{
    CreateRecoveryCommand, QueryRecoveries, RecoveryHeadView, RecoveryService,
    SubmitRecoveryResult,
};
use cuba_scrap::{
    CreateScrapCommand, QueryScraps, ScrapHeadView, ScrapService, SubmitScrapResult,
};
use cuba_shared::{audit::AuditContext, error::AppError};

use crate::response::AppJson;

pub fn routes() -> Router<AppState> {
    Router::new()
        // defects
        .route("/defects", get(list_defects).post(create_defect))
        .route("/defects/:id", get(get_defect))
        .route("/defects/:id/submit", post(submit_defect))
        .route("/defects/:id/void", post(void_defect))
        // scraps
        .route("/scraps", get(list_scraps).post(create_scrap))
        .route("/scraps/:id", get(get_scrap))
        .route("/scraps/:id/submit", post(submit_scrap))
        .route("/scraps/:id/void", post(void_scrap))
        // recoveries
        .route("/recoveries", get(list_recoveries).post(create_recovery))
        .route("/recoveries/:id", get(get_recovery))
        .route("/recoveries/:id/submit", post(submit_recovery))
        .route("/recoveries/:id/void", post(void_recovery))
}

// -- defects --

async fn list_defects(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryDefects>,
) -> Result<AppJson<Vec<DefectHeadView>>, AppError> {
    ctx.require_permission("defect.view")?;
    Ok(AppJson(DefectService::new(state.db_read().clone()).list(&ctx, &q).await?))
}

async fn get_defect(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<DefectHeadView>, AppError> {
    ctx.require_permission("defect.view")?;
    Ok(AppJson(DefectService::new(state.db_read().clone()).get(&ctx, id).await?))
}

async fn create_defect(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Json(cmd): Json<CreateDefectCommand>,
) -> Result<AppJson<DefectHeadView>, AppError> {
    ctx.require_permission("defect.create")?;
    Ok(AppJson(
        DefectService::new(state.db().clone()).create(&ctx, cmd).await?,
    ))
}

async fn submit_defect(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<SubmitDefectResult>, AppError> {
    ctx.require_permission("defect.submit")?;
    Ok(AppJson(
        DefectService::new(state.db().clone()).submit(&ctx, id).await?,
    ))
}

async fn void_defect(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<()>, AppError> {
    ctx.require_permission("defect.void")?;
    DefectService::new(state.db().clone()).void(&ctx, id).await?;
    Ok(AppJson(()))
}

// -- scraps --

async fn list_scraps(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryScraps>,
) -> Result<AppJson<Vec<ScrapHeadView>>, AppError> {
    ctx.require_permission("scrap.view")?;
    Ok(AppJson(ScrapService::new(state.db().clone()).list(&q).await?))
}

async fn get_scrap(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<ScrapHeadView>, AppError> {
    ctx.require_permission("scrap.view")?;
    Ok(AppJson(ScrapService::new(state.db().clone()).get(id).await?))
}

async fn create_scrap(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Json(cmd): Json<CreateScrapCommand>,
) -> Result<AppJson<ScrapHeadView>, AppError> {
    ctx.require_permission("scrap.create")?;
    Ok(AppJson(
        ScrapService::new(state.db().clone()).create(&ctx, cmd).await?,
    ))
}

async fn submit_scrap(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<SubmitScrapResult>, AppError> {
    ctx.require_permission("scrap.submit")?;
    Ok(AppJson(
        ScrapService::new(state.db().clone()).submit(&ctx, id).await?,
    ))
}

async fn void_scrap(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<()>, AppError> {
    ctx.require_permission("scrap.void")?;
    ScrapService::new(state.db().clone()).void(&ctx, id).await?;
    Ok(AppJson(()))
}

// -- recoveries --

async fn list_recoveries(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryRecoveries>,
) -> Result<AppJson<Vec<RecoveryHeadView>>, AppError> {
    ctx.require_permission("recovery.view")?;
    Ok(AppJson(RecoveryService::new(state.db_read().clone()).list(&ctx, &q).await?))
}

async fn get_recovery(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<RecoveryHeadView>, AppError> {
    ctx.require_permission("recovery.view")?;
    Ok(AppJson(RecoveryService::new(state.db_read().clone()).get(&ctx, id).await?))
}

async fn create_recovery(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Json(cmd): Json<CreateRecoveryCommand>,
) -> Result<AppJson<RecoveryHeadView>, AppError> {
    ctx.require_permission("recovery.create")?;
    Ok(AppJson(
        RecoveryService::new(state.db().clone()).create(&ctx, cmd).await?,
    ))
}

async fn submit_recovery(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<SubmitRecoveryResult>, AppError> {
    ctx.require_permission("recovery.submit")?;
    Ok(AppJson(
        RecoveryService::new(state.db().clone()).submit(&ctx, id).await?,
    ))
}

async fn void_recovery(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<()>, AppError> {
    ctx.require_permission("recovery.void")?;
    RecoveryService::new(state.db().clone()).void(&ctx, id).await?;
    Ok(AppJson(()))
}
