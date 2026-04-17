//! cuba-shared
//!
//! 全工作区最底层的公共 crate。不依赖 axum / sqlx(除了极少数必要场合),
//! 提供:
//! - 统一错误码与错误类型 ([`error`])
//! - 统一 Result 别名 ([`result`])
//! - 分页请求/响应 ([`pagination`])
//! - 审计上下文(操作人/IP/trace-id)([`audit`])
//! - JWT Claims ([`auth`])
//! - 时间与时区辅助 ([`time`])
//! - 通用业务枚举 ([`types`])
//! - 领域事件 trait ([`event`])
//!
//! 设计原则:
//! - 只定义数据结构与 trait,不写 axum Handler、不写 sqlx 查询。
//! - HTTP 响应的 `IntoResponse` impl 放在 `cuba-api`,避免循环依赖。

#![deny(unsafe_code)]
#![warn(
    clippy::pedantic,
    clippy::nursery,
    rust_2018_idioms,
    unreachable_pub,
    missing_debug_implementations
)]
#![allow(
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::multiple_crate_versions
)]

pub mod audit;
pub mod auth;
pub mod error;
pub mod event;
pub mod money;
pub mod pagination;
pub mod result;
pub mod time;
pub mod types;

// 常用重导出,业务 crate 基本只需要 `use cuba_shared::prelude::*;`
pub mod prelude {
    pub use crate::audit::AuditContext;
    pub use crate::error::{AppError, ErrorCode};
    pub use crate::pagination::{PageQuery, PageResponse};
    pub use crate::result::{ApiSuccess, AppResult};
    pub use crate::types::{DocStatus, IoFlag, StockStatus, TxnType};
}
