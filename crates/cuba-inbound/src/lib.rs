//! cuba-inbound
//!
//! 入库单 (`wms_inbound_h/d`)。
//!
//! ## 入库类型(与 DDL CHECK 对齐)
//! `PURCHASE / PROD / RETURN / OUTSOURCE_BACK / CUSTOMER_RETURN / RECOVERY_IN / OTHER`
//!
//! ## 流程
//! 1. `create_inbound`:DRAFT 状态,写 `wms_inbound_h/d`,不动库存
//! 2. `submit_inbound`:校验 → 调 `InventoryService::commit(IN txn)` 增加库存 → 改单据状态 COMPLETED
//! 3. `void_inbound`:作废,仅允许 DRAFT/SUBMITTED;COMPLETED 要靠逆向单据(退供/不良)

#![deny(unsafe_code)]

pub mod application;
pub mod domain;
pub mod infrastructure;

pub use application::{
    CreateInboundCommand, CreateInboundLine, InboundHeadView, InboundLineView, InboundService,
    QueryInbounds, SubmitInboundResult,
};
pub use infrastructure::repository::{InboundRepository, PgInboundRepository};
