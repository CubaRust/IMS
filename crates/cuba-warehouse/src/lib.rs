//! cuba-warehouse
//!
//! 仓库 + 仓位主数据管理。
//!
//! - 仓库(`mdm_warehouse`):按 `wh_type` 分类(RAW_WH / SEMI_WH / FG_WH / BAD_WH / ...)
//! - 仓位(`mdm_location`):挂在仓库下,`loc_type` 决定用途(NORMAL / IQC / BAD / ...)
//!
//! 仓库和仓位启停通过 `is_active` 软删除,不做物理删除(业务表里到处引用)。

#![deny(unsafe_code)]

pub mod application;
pub mod domain;
pub mod infrastructure;

pub use application::{
    CreateLocationCommand, CreateWarehouseCommand, LocationView, QueryLocations, QueryWarehouses,
    UpdateLocationCommand, UpdateWarehouseCommand, WarehouseService, WarehouseView,
};
pub use infrastructure::repository::{PgWarehouseRepository, WarehouseRepository};
