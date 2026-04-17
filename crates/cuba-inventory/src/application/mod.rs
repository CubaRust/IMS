//! 库存应用层
//!
//! 按 CQRS 习惯分 commands(写)和 queries(读),service 是对外聚合门面。

pub mod commands;
pub mod dto;
pub mod queries;
pub mod service;
