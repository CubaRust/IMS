//! HTTP 响应:为 `AppError` / `ApiSuccess` 实现 `IntoResponse`
//!
//! ## 成功
//! ```json
//! { "code": 0, "message": "ok", "data": ..., "trace_id": "..." }
//! ```
//!
//! ## 失败
//! ```json
//! { "code": 20101, "message": "库存不足", "trace_id": "..." }
//! ```
//! HTTP 状态按 `AppError::http_status()` 决定;业务错误统一 200。

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;

use cuba_shared::{
    error::{AppError, ErrorBody},
    result::ApiSuccess,
};

/// 统一响应信封:成功 / 失败都走这个
#[derive(Debug, Serialize)]
pub struct Envelope<T> {
    pub code: u32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

impl<T: Serialize> IntoResponse for ApiSuccess<T> {
    fn into_response(self) -> Response {
        let body = Envelope {
            code: self.code,
            message: self.message.to_string(),
            data: Some(self.data),
            trace_id: self.trace_id,
        };
        (StatusCode::OK, Json(body)).into_response()
    }
}

/// 新类型包装:给业务 crate 的 handler 直接返回 `AppJson<T>`,
/// 不用自己包 `ApiSuccess`
#[derive(Debug)]
pub struct AppJson<T>(pub T);

impl<T: Serialize> IntoResponse for AppJson<T> {
    fn into_response(self) -> Response {
        ApiSuccess::ok(self.0).into_response()
    }
}

/// 对 `AppError` 的 `IntoResponse` 实现
pub struct AppErrorResponse(pub AppError);

impl From<AppError> for AppErrorResponse {
    fn from(e: AppError) -> Self {
        Self(e)
    }
}

impl IntoResponse for AppErrorResponse {
    fn into_response(self) -> Response {
        let err = self.0;
        let status = StatusCode::from_u16(err.http_status()).unwrap_or(StatusCode::OK);

        // 对 500 级日志化
        if err.http_status() >= 500 {
            tracing::error!(error = %err, "server error");
        } else {
            tracing::debug!(error = %err, "business error");
        }

        // 上报到 Prometheus(HTTP 200 的业务错误也算,500 也算)
        cuba_metrics::record_business_error(err.code().as_u32());

        let body: ErrorBody = (&err).into();
        (status, Json(body)).into_response()
    }
}

/// 让业务 handler 可以直接 `?` 抛 `AppError`
///
/// 约定:业务 handler 的返回类型用 `Result<AppJson<T>, AppError>`,
/// 由 axum 通过此 impl 把错误转成 JSON 响应。
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        AppErrorResponse(self).into_response()
    }
}

/// 404 兜底(axum fallback)
pub async fn not_found_fallback() -> Response {
    let body = json!({
        "code": 10404,
        "message": "路径不存在",
    });
    (StatusCode::NOT_FOUND, Json(body)).into_response()
}
