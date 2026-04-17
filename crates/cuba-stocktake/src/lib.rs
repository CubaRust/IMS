//! cuba-stocktake
//!
//! 盘点单 (`wms_stocktake_h/d`)。
//!
//! ## 流程
//! 1. `create`:建立盘点任务,圈定仓位+物料范围。
//!    支持两种模式:
//!    - **snapshot** 模式:create 时从 `inv.balance` 快照账面数到 `wms_stocktake_d`
//!    - **explicit** 模式:上层直接提供 lines
//! 2. `record_counts`:登记实盘数量
//! 3. `submit`:对每行 `diff = actual - book`,产生 CONVERT 事务:
//!    - `diff > 0` → IN QUALIFIED (盈)
//!    - `diff < 0` → OUT QUALIFIED (亏)
//!    - `diff == 0` → 跳过
//! 4. `void`:仅 DRAFT

#![deny(unsafe_code)]

pub mod repo;
pub mod service;

pub use repo::{PgStocktakeRepository, StocktakeRepository};
pub use service::{
    CreateStocktakeCommand, CreateStocktakeLine, QueryStocktakes, RecordCountCommand,
    RecordCountLine, StocktakeHeadView, StocktakeLineView, StocktakeService, SubmitStocktakeResult,
};
