//! 错误类型与错误码
//!
//! ## 错误码分段(参见 docs/architecture/error-code-convention.md)
//!
//! | 段 | 范围 | 模块 |
//! |---|---|---|
//! | 10xxx | 10000-10999 | shared / 通用 |
//! | 11xxx | 11000-11999 | identity |
//! | 20xxx | 20000-20999 | inventory |
//! | 21xxx | 21000-21999 | warehouse |
//! | 22xxx | 22000-22999 | catalog |
//! | 30xxx | 30000-30999 | inbound |
//! | 31xxx | 31000-31999 | outbound |
//! | 32xxx | 32000-32999 | stocktake |
//! | 33xxx | 33000-33999 | preissue |
//! | 40xxx | 40000-40999 | defect |
//! | 41xxx | 41000-41999 | recovery |
//! | 42xxx | 42000-42999 | scrap |
//! | 43xxx | 43000-43999 | customer-return |
//! | 44xxx | 44000-44999 | supplier-return |
//! | 50xxx | 50000-50999 | pmc / outsource |
//! | 60xxx | 60000-60999 | reporting |
//!
//! ## 响应约定
//!
//! 业务错误 HTTP 状态统一 200,`code` 字段区分;仅基础设施错误(401/403/500)
//! 才使用对应的 HTTP status。
//!
//! ```json
//! { "code": 20101, "message": "库存不足", "data": null, "trace_id": "..." }
//! ```

use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use thiserror::Error;

mod codes;
pub use codes::ErrorCode;

/// 统一应用错误。
///
/// 业务 crate 应该优先使用 `AppError::business`、`AppError::not_found` 等
/// 构造函数,而不是直接 `AppError { ... }`,以便错误码集中管理。
#[derive(Debug, Error)]
pub enum AppError {
    /// 业务规则错误(HTTP 200,`code` 标识具体业务错误)
    #[error("{message}")]
    Business {
        code: ErrorCode,
        message: Cow<'static, str>,
    },

    /// 请求参数校验失败(HTTP 200,code=10002)
    #[error("参数校验失败: {message}")]
    Validation { message: Cow<'static, str> },

    /// 资源不存在(HTTP 200,code=10003)
    #[error("资源不存在: {what}")]
    NotFound { what: Cow<'static, str> },

    /// 未登录(HTTP 401,code=10401)
    #[error("未登录或登录已过期")]
    Unauthenticated,

    /// 无权限(HTTP 403,code=10403)
    #[error("无权限访问")]
    Forbidden { perm: Cow<'static, str> },

    /// 冲突(HTTP 200,code=10409,用于唯一约束、乐观锁等)
    #[error("资源冲突: {message}")]
    Conflict { message: Cow<'static, str> },

    /// 数据库错误(HTTP 500,code=10500)
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),

    /// 不可恢复的内部错误(HTTP 500,code=10500)
    #[error("内部错误: {0}")]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    /// 业务错误的通用构造
    #[must_use]
    pub fn business(code: ErrorCode, message: impl Into<Cow<'static, str>>) -> Self {
        Self::Business {
            code,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn validation(message: impl Into<Cow<'static, str>>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    #[must_use]
    pub fn not_found(what: impl Into<Cow<'static, str>>) -> Self {
        Self::NotFound { what: what.into() }
    }

    #[must_use]
    pub fn forbidden(perm: impl Into<Cow<'static, str>>) -> Self {
        Self::Forbidden { perm: perm.into() }
    }

    #[must_use]
    pub fn conflict(message: impl Into<Cow<'static, str>>) -> Self {
        Self::Conflict {
            message: message.into(),
        }
    }

    /// 返回对外暴露的 `code` 字段
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        match self {
            Self::Business { code, .. } => *code,
            Self::Validation { .. } => ErrorCode::VALIDATION,
            Self::NotFound { .. } => ErrorCode::NOT_FOUND,
            Self::Unauthenticated => ErrorCode::UNAUTHENTICATED,
            Self::Forbidden { .. } => ErrorCode::FORBIDDEN,
            Self::Conflict { .. } => ErrorCode::CONFLICT,
            Self::Database(_) | Self::Internal(_) => ErrorCode::INTERNAL,
        }
    }

    /// 返回对外展示的 HTTP 状态码数值。
    ///
    /// 约定:只有基础设施类错误(401/403/500)才脱离 HTTP 200。
    #[must_use]
    pub const fn http_status(&self) -> u16 {
        match self {
            Self::Unauthenticated => 401,
            Self::Forbidden { .. } => 403,
            Self::Database(_) | Self::Internal(_) => 500,
            _ => 200,
        }
    }
}

/// 对外响应结构(`cuba-api` 负责 `IntoResponse`)
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: u32,
    pub message: String,
    pub trace_id: Option<String>,
}

impl From<&AppError> for ErrorBody {
    fn from(err: &AppError) -> Self {
        Self {
            code: err.code().as_u32(),
            message: err.to_string(),
            trace_id: None,
        }
    }
}

/// 实现 IntoResponse 以便在 axum handler 中直接使用
impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        use axum::{http::StatusCode, response::Json};
        
        let status = StatusCode::from_u16(self.http_status()).unwrap_or(StatusCode::OK);

        // 对 500 级日志化
        if self.http_status() >= 500 {
            tracing::error!(error = %self, "server error");
        } else {
            tracing::debug!(error = %self, "business error");
        }

        let body: ErrorBody = (&self).into();
        (status, Json(body)).into_response()
    }
}
