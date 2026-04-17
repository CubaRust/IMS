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
    error::AppError,
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

/// Newtype wrapper for ApiSuccess to satisfy orphan rules
pub struct ApiSuccessResponse<T>(pub ApiSuccess<T>);

impl<T> From<ApiSuccess<T>> for ApiSuccessResponse<T> {
    fn from(success: ApiSuccess<T>) -> Self {
        Self(success)
    }
}

impl<T: Serialize> IntoResponse for ApiSuccessResponse<T> {
    fn into_response(self) -> Response {
        let body = Envelope {
            code: self.0.code,
            message: self.0.message.to_string(),
            data: Some(self.0.data),
            trace_id: self.0.trace_id,
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
        ApiSuccessResponse(ApiSuccess::ok(self.0)).into_response()
    }
}

/// 对 `AppError` 的 `IntoResponse` 实现已经在 cuba-shared 中完成
/// 这里保留 AppErrorResponse 作为兼容性包装（如果需要的话）
pub struct AppErrorResponse(pub AppError);

impl From<AppError> for AppErrorResponse {
    fn from(e: AppError) -> Self {
        Self(e)
    }
}

impl IntoResponse for AppErrorResponse {
    fn into_response(self) -> Response {
        // 直接委托给 AppError 的 IntoResponse 实现
        self.0.into_response()
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
