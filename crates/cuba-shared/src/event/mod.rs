//! 领域事件 trait(预留)
//!
//! 本期不做事件总线,先定义 trait,未来可以接入 `tokio::sync::broadcast`
//! 或外部 MQ(NATS / Kafka)。
//!
//! 使用模式:
//! ```ignore
//! #[derive(Debug, Clone, Serialize)]
//! struct InventoryChanged { /* ... */ }
//! impl DomainEvent for InventoryChanged {
//!     fn event_name() -> &'static str { "inventory.changed" }
//! }
//! ```

use serde::Serialize;

/// 领域事件
pub trait DomainEvent: Serialize + Send + Sync + 'static {
    /// 事件名(点分命名:`module.action`,如 `inventory.txn.committed`)
    fn event_name() -> &'static str;
}
