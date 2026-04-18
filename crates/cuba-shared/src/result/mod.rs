use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// 工作区通用 Result
pub type AppResult<T> = std::result::Result<T, AppError>;

/// 成功响应信封
///
/// 格式:
/// ```json
/// { "code": 0, "message": "ok", "data": { ... }, "trace_id": "..." }
/// ```
///
/// 业务 crate 的 application 层返回领域对象,不关心信封;
/// 信封由 `cuba-api` 的 handler 包装。
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiSuccess<T> {
    pub code: u32,
    pub message: &'static str,
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

impl<T> ApiSuccess<T> {
    pub const fn ok(data: T) -> Self {
        Self {
            code: 0,
            message: "ok",
            data,
            trace_id: None,
        }
    }

    #[must_use]
    pub fn with_trace(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }
}

// `IntoResponse` 放在类型定义 crate 以满足孤儿规则
impl<T: Serialize> axum::response::IntoResponse for ApiSuccess<T> {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::Json;
        use serde_json::json;

        let body = json!({
            "code": self.code,
            "message": self.message,
            "data": self.data,
            "trace_id": self.trace_id,
        });
        (StatusCode::OK, Json(body)).into_response()
    }
}
