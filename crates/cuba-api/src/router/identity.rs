//! 认证 + 用户/角色/权限 HTTP 路由
//!
//! 路径:
//! ```text
//! POST /api/v1/auth/login          -- 登录(无守卫)
//! GET  /api/v1/auth/me             -- 当前用户(需守卫)
//! PUT  /api/v1/auth/password       -- 改自己密码(需守卫)
//!
//! GET  /api/v1/users               -- 用户列表(admin)
//! GET  /api/v1/roles               -- 角色列表(admin)
//! GET  /api/v1/permissions         -- 权限点列表(admin)
//! ```

use axum::{
    extract::{Extension, Query, State},
    routing::{get, post, put},
    Json, Router,
};

use cuba_bootstrap::AppState;
use cuba_identity::{
    ChangePasswordCommand, IdentityService, LoginCommand, LoginResult, UserView,
};
use cuba_identity::application::{PermissionView, QueryUsers, RoleView};
use cuba_shared::{audit::AuditContext, error::AppError};

use crate::response::AppJson;

/// 公开路由(不走 auth_guard)
pub fn public_routes() -> Router<AppState> {
    Router::new().route("/auth/login", post(login))
}

/// 受保护路由(挂 auth_guard)
pub fn protected_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/me", get(me))
        .route("/auth/password", put(change_password))
        .route("/auth/logout", post(logout))
        .route("/auth/refresh", post(refresh))
        .route("/users", get(list_users))
        .route("/roles", get(list_roles))
        .route("/permissions", get(list_permissions))
}

// ---------------------------------------------------------------------------
// handlers
// ---------------------------------------------------------------------------

fn build_service(state: &AppState) -> IdentityService {
    IdentityService::new(
        state.db().clone(),
        &state.config().jwt_secret,
        state.config().jwt_ttl_seconds,
    )
}

/// 只读 service:me / list_users / list_roles / list_permissions 用
fn build_read_service(state: &AppState) -> IdentityService {
    IdentityService::new(
        state.db_read().clone(),
        &state.config().jwt_secret,
        state.config().jwt_ttl_seconds,
    )
}

async fn login(
    State(state): State<AppState>,
    Json(cmd): Json<LoginCommand>,
) -> Result<AppJson<LoginResult>, AppError> {
    let svc = build_service(&state);
    let result = svc.login(cmd).await?;
    Ok(AppJson(result))
}

async fn me(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
) -> Result<AppJson<UserView>, AppError> {
    let svc = build_read_service(&state);
    Ok(AppJson(svc.me(&ctx).await?))
}

async fn change_password(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Json(cmd): Json<ChangePasswordCommand>,
) -> Result<AppJson<()>, AppError> {
    let svc = build_service(&state);
    svc.change_password(&ctx, cmd).await?;
    Ok(AppJson(()))
}

async fn list_users(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryUsers>,
) -> Result<AppJson<Vec<UserView>>, AppError> {
    ctx.require_permission("sys.user.manage")?;
    let svc = build_read_service(&state);
    Ok(AppJson(svc.list_users(&ctx, &q).await?))
}

async fn list_roles(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
) -> Result<AppJson<Vec<RoleView>>, AppError> {
    ctx.require_permission("sys.role.manage")?;
    let svc = build_read_service(&state);
    Ok(AppJson(svc.list_roles().await?))
}

async fn list_permissions(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
) -> Result<AppJson<Vec<PermissionView>>, AppError> {
    ctx.require_permission("sys.role.manage")?;
    let svc = build_read_service(&state);
    Ok(AppJson(svc.list_permissions().await?))
}

async fn logout(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
) -> Result<AppJson<()>, AppError> {
    let jti = ctx
        .jti
        .as_deref()
        .ok_or_else(|| AppError::validation("JWT 缺少 jti,不能登出"))?;
    let exp = ctx
        .jwt_exp
        .ok_or_else(|| AppError::validation("JWT 缺少 exp"))?;
    let svc = build_service(&state);
    svc.logout(&ctx, jti, exp).await?;
    Ok(AppJson(()))
}

async fn refresh(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
) -> Result<AppJson<cuba_identity::LoginResult>, AppError> {
    let jti = ctx
        .jti
        .as_deref()
        .ok_or_else(|| AppError::validation("JWT 缺少 jti,不能刷新"))?;
    let exp = ctx
        .jwt_exp
        .ok_or_else(|| AppError::validation("JWT 缺少 exp"))?;
    let svc = build_service(&state);
    Ok(AppJson(svc.refresh(&ctx, jti, exp).await?))
}
