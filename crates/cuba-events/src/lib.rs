//! cuba-events
//!
//! 领域事件定义 + 落库辅助。
//!
//! ## 使用
//! 业务 service 在状态变更时调 [`write_event`] 或 [`write_event_tx`] 写事件。
//! 事件和业务表的变更必须在同一 DB 事务中,保证原子性。
//!
//! ## 已定义事件(稳定 schema)
//! - `InventoryTxnCommitted` — 库存事务提交
//! - `InboundSubmitted` — 入库单提交
//! - `InboundVoided` — 入库单作废
//! - `OutboundSubmitted` — 出库单提交
//! - `OutboundVoided` — 出库单作废
//! - `PreissueCreated` — 异常先发建立
//! - `PreissueClosed` — 异常先发闭环
//!
//! 事件一旦发布 **不得修改 schema**。需要演进时新增 `event_version` 或新事件类型。

#![deny(unsafe_code)]

pub mod types;
pub mod writer;

pub use types::{DomainEvent, EventEnvelope};
pub use writer::{write_event, write_event_tx, WriteEventCtx};
