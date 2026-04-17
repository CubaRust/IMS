//! 基础设施层:sqlx 实现的持久化
//!
//! - `repository` : 对外 trait + PG 实现
//! - 其他细节模块可以放 `read_model/*`(为报表查询优化的反范式读模型)

pub mod repository;
