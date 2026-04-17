//! 通用中间件
//!
//! - `trace_id` : 为每个请求生成或透传 `X-Trace-Id`,并注入 tracing span
//! - `auth_guard` : 校验 `Authorization: Bearer <jwt>`,把 `AuditContext` 放进 request extensions
//!
//! 业务 handler 通过 `Extension<AuditContext>` 取出当前用户;
//! 未登录接口(login / health)不挂 `auth_guard`。

use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderValue, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use cuba_shared::{
    audit::AuditContext,
    auth::jwt,
    error::AppError,
};
use cuba_bootstrap::AppState;

const TRACE_HEADER: &str = "X-Trace-Id";

/// Trace-ID 中间件
///
/// - 读取请求的 `X-Trace-Id`,没有就生成 `uuid-v4`
/// - 响应头带回 `X-Trace-Id`
/// - 把 trace_id 存进 request extensions,供后续层使用
pub async fn trace_id(mut req: Request<Body>, next: Next) -> Response {
    let trace_id = req
        .headers()
        .get(TRACE_HEADER)
        .and_then(|v| v.to_str().ok())
        .map_or_else(|| Uuid::new_v4().to_string(), ToString::to_string);

    req.extensions_mut().insert(TraceId(trace_id.clone()));

    let mut resp = next.run(req).await;
    if let Ok(v) = HeaderValue::from_str(&trace_id) {
        resp.headers_mut().insert(TRACE_HEADER, v);
    }
    resp
}

/// 请求作用域内的 trace-id
#[derive(Debug, Clone)]
pub struct TraceId(pub String);

/// JWT 鉴权中间件
///
/// 成功:把 `AuditContext` 塞进 extensions
/// 失败:401
pub async fn auth_guard(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthenticated)?;

    let claims =
        jwt::decode_token(token, state.config().jwt_secret.as_bytes())?;
    let user_id = claims.user_id().ok_or(AppError::Unauthenticated)?;

    let trace_id = req
        .extensions()
        .get::<TraceId>()
        .map_or_else(|| Uuid::new_v4().to_string(), |t| t.0.clone());

    let ctx = AuditContext {
        user_id,
        login_name: claims.login_name,
        trace_id,
        ip: None,
        user_agent: req
            .headers()
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(ToString::to_string),
        permissions: claims.permissions,
        roles: claims.roles,
    };

    req.extensions_mut().insert(ctx);
    Ok(next.run(req).await)
}

/// 给调试 404 用的小工具:不属于中间件,暂放这
pub async fn health() -> (StatusCode, &'static str) {
    (StatusCode::OK, "ok")
}
