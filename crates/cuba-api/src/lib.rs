//! cuba-api
//!
//! HTTP 聚合层:
//! - `router` : 各业务模块路由的聚合入口
//! - `middleware` : 通用中间件(trace-id、CORS、auth 守卫)
//! - `response` : 为 `AppError` / `ApiSuccess` 实现 `IntoResponse`
//! - `dto` : 跨业务共用的 HTTP DTO(目前是空占位)
//!
//! ## 架构约定(见 `docs/architecture/ddd-boundaries.md`)
//! 业务 crate 只导出 application 层 command/query handler。本 crate 负责:
//! 1. 从 HTTP 入参解析成 application 层的入参类型
//! 2. 调 application handler
//! 3. 把结果包装成 `ApiSuccess` / `AppError` 响应

#![deny(unsafe_code)]

pub mod dto;
pub mod middleware;
pub mod openapi;
pub mod response;
pub mod router;

pub use openapi::{swagger_router, ApiDoc};
pub use router::build_router;
