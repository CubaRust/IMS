//! catalog 基础设施
//!
//! 每个实体一个 repo,避免一个巨型 file。

pub mod bom_repo;
pub mod material_repo;
pub mod party_repo;
pub mod route_repo;
pub mod status_flow_repo;

use cuba_shared::error::AppError;

/// 把 sqlx 23505(unique violation)映射为 CONFLICT
pub(crate) fn map_unique_err(e: sqlx::Error) -> AppError {
    match &e {
        sqlx::Error::Database(db) if db.code().as_deref() == Some("23505") => {
            AppError::conflict(format!("唯一约束冲突: {}", db.message()))
        }
        _ => e.into(),
    }
}
