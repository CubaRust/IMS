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

use cuba_bootstrap::AppState;
use cuba_shared::{audit::AuditContext, auth::jwt, error::AppError};

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

    let claims = jwt::decode_token(token, state.config().jwt_secret.as_bytes())?;
    let user_id = claims.user_id().ok_or(AppError::Unauthenticated)?;

    // 查 jti 黑名单(logout / refresh / force_logout 会写)
    if !claims.jti.is_empty() {
        let revoked: Option<i64> =
            sqlx::query_scalar("select 1 from sys.sys_jwt_revocation where jti = $1 limit 1")
                .bind(&claims.jti)
                .fetch_optional(state.db())
                .await
                .map_err(|e| {
                    tracing::warn!(error=%e, "query jwt revocation failed");
                    AppError::Unauthenticated
                })?;
        if revoked.is_some() {
            return Err(AppError::Unauthenticated);
        }
    }

    let trace_id = req
        .extensions()
        .get::<TraceId>()
        .map_or_else(|| Uuid::new_v4().to_string(), |t| t.0.clone());

    let ctx = AuditContext {
        user_id,
        login_name: claims.login_name,
        trace_id,
        tenant_id: claims.tenant_id,
        ip: None,
        user_agent: req
            .headers()
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(ToString::to_string),
        permissions: claims.permissions,
        roles: claims.roles,
        jti: if claims.jti.is_empty() {
            None
        } else {
            Some(claims.jti)
        },
        jwt_exp: Some(claims.exp),
    };

    req.extensions_mut().insert(ctx);
    Ok(next.run(req).await)
}

/// 给调试 404 用的小工具:不属于中间件,暂放这
pub async fn health() -> (StatusCode, &'static str) {
    (StatusCode::OK, "ok")
}

/// k8s liveness — 进程是否活着。不查 DB,永远返回 200。
pub async fn live() -> (StatusCode, &'static str) {
    (StatusCode::OK, "live")
}

/// k8s readiness — 是否可接流量。ping 一次 DB。
///
/// DB 不可达 → 503,k8s 会把 Pod 从 Service endpoints 摘掉。
pub async fn ready(State(state): State<AppState>) -> (StatusCode, &'static str) {
    match sqlx::query_scalar::<_, i64>("select 1")
        .fetch_one(state.db())
        .await
    {
        Ok(_) => (StatusCode::OK, "ready"),
        Err(e) => {
            tracing::warn!(error = %e, "readiness probe failed");
            (StatusCode::SERVICE_UNAVAILABLE, "not-ready")
        }
    }
}

/// `/metrics` — Prometheus 指标导出
pub async fn metrics() -> (StatusCode, [(header::HeaderName, HeaderValue); 1], String) {
    let body = cuba_metrics::gather_text();
    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; version=0.0.4"),
        )],
        body,
    )
}

/// HTTP metrics 中间件 — 记录 cuba_http_requests_total / duration
pub async fn http_metrics(req: Request<Body>, next: Next) -> Response {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let start = std::time::Instant::now();
    let resp = next.run(req).await;
    let elapsed = start.elapsed().as_secs_f64();
    cuba_metrics::record_http(&method, &path, resp.status().as_u16(), elapsed);
    resp
}

// ---------------------------------------------------------------------------
// 审计日志中间件
// ---------------------------------------------------------------------------

/// 审计日志中间件
///
/// 在请求完成后,**异步**(spawn)写一行到 `sys.sys_audit_log`。
/// 写日志失败仅 warn,不影响请求结果。
///
/// 挂载方式:在根路由的 `protected` 分支上挂,只记录受保护请求:
/// ```ignore
/// .layer(axum_mw::from_fn_with_state(state.clone(), audit_log))
/// ```
pub async fn audit_log(State(state): State<AppState>, req: Request<Body>, next: Next) -> Response {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let trace_id = req
        .headers()
        .get(TRACE_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let user_agent = req
        .headers()
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    // 注:ip 需要上游 TrustedProxy 头或 ConnectInfo,本期先空
    let ip: Option<String> = None;

    // 取出 AuditContext(如果已在 auth_guard 里放了)
    let ctx = req.extensions().get::<AuditContext>().cloned();
    let user_id = ctx.as_ref().map(|c| c.user_id);
    let login_name = ctx.as_ref().map(|c| c.login_name.clone());

    let start = std::time::Instant::now();
    let resp = next.run(req).await;
    let elapsed_ms = start.elapsed().as_millis() as i32;
    let http_status = resp.status().as_u16() as i32;

    let pool = state.db().clone();
    tokio::spawn(async move {
        if let Err(e) = sqlx::query(
            r#"
            insert into sys.sys_audit_log
                (trace_id, user_id, login_name,
                 http_method, http_path, http_status,
                 ip, user_agent, duration_ms)
            values ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            "#,
        )
        .bind(&trace_id)
        .bind(user_id)
        .bind(&login_name)
        .bind(&method)
        .bind(&path)
        .bind(http_status)
        .bind(&ip)
        .bind(&user_agent)
        .bind(elapsed_ms)
        .execute(&pool)
        .await
        {
            tracing::warn!(error = %e, "audit log insert failed");
        }
    });

    resp
}
