//! cuba-scrap
//!
//! 报废单 (`wms_scrap_h/d`)。
//!
//! ## 流程
//! 1. `create`:DRAFT,不动库存
//! 2. `submit`:OUT 出原状态库存(通常是 BAD),状态流转到 SCRAPPED
//!    本期实现:从源仓位 OUT (status=源状态) → IN 报废仓 status=SCRAPPED
//!    即 TRANSFER 到报废仓
//! 3. `void`:仅 DRAFT/SUBMITTED

#![deny(unsafe_code)]

pub mod application;
pub mod domain;
pub mod infrastructure;

pub use application::{
    CreateScrapCommand, CreateScrapLine, QueryScraps, ScrapHeadView, ScrapLineView,
    ScrapService, SubmitScrapResult,
};
pub use infrastructure::repository::{PgScrapRepository, ScrapRepository};
