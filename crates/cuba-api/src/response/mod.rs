//! HTTP 响应:本地新类型包装 + 404 兜底
//!
//! `IntoResponse` for `AppError` / `ApiSuccess<T>` 已移至 `cuba-shared`(孤儿规则)。
//! 本模块保留 `AppJson<T>` 便捷包装和 `AppErrorResponse`(含 metrics 上报)。

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;

use cuba_shared::{error::AppError, result::ApiSuccess};

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

/// 新类型包装:给业务 crate 的 handler 直接返回 `AppJson<T>`,
/// 不用自己包 `ApiSuccess`
#[derive(Debug)]
pub struct AppJson<T>(pub T);

impl<T: Serialize> IntoResponse for AppJson<T> {
    fn into_response(self) -> Response {
        ApiSuccess::ok(self.0).into_response()
    }
}

/// 对 `AppError` 的 `IntoResponse` 包装(含 metrics 上报)
pub struct AppErrorResponse(pub AppError);

impl From<AppError> for AppErrorResponse {
    fn from(e: AppError) -> Self {
        Self(e)
    }
}

impl IntoResponse for AppErrorResponse {
    fn into_response(self) -> Response {
        let err = self.0;
        // 上报到 Prometheus(HTTP 200 的业务错误也算,500 也算)
        cuba_metrics::record_business_error(err.code().as_u32());
        err.into_response()
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
