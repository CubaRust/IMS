//! cuba-defect
//!
//! 不良单 (`wms_defect_h/d`)。
//!
//! 关键规则(需求 16.3 / 16.4):
//! - FOG/TP/ASM 级的 NG 不能走"普通退料",必须登记不良单
//! - 4 种 process_method:
//!   - `TO_BAD_STOCK` 转不良库:QUALIFIED/TO_CHECK → BAD,库存状态流转,不改数量
//!   - `TO_DISMANTLE` 拆解:本模块仅登记,实际拆解走 cuba-recovery
//!   - `TO_SCRAP` 报废:本模块仅登记,实际报废走 cuba-scrap
//!   - `TO_REWORK` 返工:当前先只登记,状态改回 IN_PROCESS,后续由工单回补
//!
//! 本期 defect 提交(submit)只处理 `TO_BAD_STOCK`:
//! 发起一笔 TRANSFER 行(同仓位内状态变更 QUALIFIED→BAD)。
//! 其他 3 种方式由各自下游单据完成库存动作。

#![deny(unsafe_code)]

pub mod application;
pub mod domain;
pub mod infrastructure;

pub use application::{
    CreateDefectCommand, CreateDefectLine, DefectHeadView, DefectLineView, DefectService,
    QueryDefects, SubmitDefectResult,
};
pub use infrastructure::repository::{DefectRepository, PgDefectRepository};
