//! cuba-recovery
//!
//! 拆解回收单 (`wms_recovery_h/in/out/scrap`)。
//!
//! ## 典型场景(需求 16.4、文档第七节)
//! - FOG NG:回收 FPC
//! - TP NG:回收盖板 / FPC
//! - 总成 NG:回收盖板 / FPC / 屏幕
//!
//! ## 库存动作
//! submit 时走一笔 CONVERT:
//! - N 条 OUT (源物料 BAD 状态,即 recovery_in)
//! - N 条 IN  (回收物料 QUALIFIED 或 RECOVERY 状态,即 recovery_out)
//! - N 条 IN  (报废仓 SCRAPPED 状态,即 recovery_scrap)

#![deny(unsafe_code)]

pub mod application;
pub mod domain;
pub mod infrastructure;

pub use application::{
    CreateRecoveryCommand, CreateRecoveryIn, CreateRecoveryOut, CreateRecoveryScrap,
    QueryRecoveries, RecoveryHeadView, RecoveryInView, RecoveryOutView, RecoveryScrapView,
    RecoveryService, SubmitRecoveryResult,
};
pub use infrastructure::repository::{PgRecoveryRepository, RecoveryRepository};
