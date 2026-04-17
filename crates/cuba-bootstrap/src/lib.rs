//! cuba-bootstrap
//!
//! 进程启动期的胶水:
//! - 读取配置(环境变量 + `.env`)
//! - 初始化 tracing
//! - 建立数据库连接池
//! - 按 `MIGRATION_MODE` 跑迁移(auto)或仅校验(manual)
//! - 组装 `AppState`
//!
//! 不写任何 HTTP / 业务逻辑。`cuba-server/main.rs` 调用本 crate,
//! 拿到 `AppState` 后交给 `cuba-api::build_router(state)` 起服务。

#![deny(unsafe_code)]

pub mod config;
pub mod database;
pub mod state;
pub mod telemetry;

pub use config::AppConfig;
pub use state::AppState;
