//! cuba-outbound
//!
//! 出库单 (`wms_outbound_h/d`)。出库即扣减合格库存(或指定状态)。
//!
//! ## 出库类型(与 DDL CHECK 对齐)
//! `PROD_ISSUE / PROCESS_ISSUE / PUBLIC_ISSUE / OUTSOURCE_SEND / SUPPLIER_RETURN / SALES_SEND / SCRAP_OUT / OTHER`
//!
//! ## 流程
//! 1. `create`:DRAFT,不动库存
//! 2. `submit`:调 `InventoryService::commit(OUT)` 扣减库存,DB CHECK 不够时抛 20101
//! 3. `void`:作废,仅 DRAFT/SUBMITTED

#![deny(unsafe_code)]

pub mod application;
pub mod domain;
pub mod infrastructure;

pub use application::{
    CreateOutboundCommand, CreateOutboundLine, OutboundHeadView, OutboundLineView, OutboundService,
    QueryOutbounds, SubmitOutboundResult,
};
pub use infrastructure::repository::{OutboundRepository, PgOutboundRepository};
