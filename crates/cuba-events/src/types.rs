//! 领域事件定义

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 事件信封:统一元信息 + payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub event_id: String,
    pub tenant_id: i64,
    pub aggregate_type: String,
    pub aggregate_id: Option<i64>,
    pub event_type: String,
    pub event_version: i32,
    pub payload: serde_json::Value,
    pub occurred_at: i64,
    pub trace_id: Option<String>,
    pub actor_id: Option<i64>,
    pub actor_name: Option<String>,
}

/// 具体领域事件(tagged union)
///
/// serde 序列化后作为 `payload` 字段落库。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "PascalCase")]
pub enum DomainEvent {
    /// 库存事务提交(最底层)
    InventoryTxnCommitted {
        txn_id: i64,
        txn_no: String,
        txn_type: String,
        scene_code: String,
        doc_type: String,
        doc_no: String,
        line_count: usize,
        /// 摘要:各行的 material_id / qty
        lines_summary: Vec<InventoryLineSummary>,
    },
    /// 入库单提交
    InboundSubmitted {
        inbound_id: i64,
        inbound_no: String,
        inbound_type: String,
        wh_id: i64,
        txn_no: String,
    },
    /// 入库单作废
    InboundVoided {
        inbound_id: i64,
        inbound_no: String,
        prev_status: String,
    },
    /// 出库单提交
    OutboundSubmitted {
        outbound_id: i64,
        outbound_no: String,
        outbound_type: String,
        wh_id: i64,
        txn_no: String,
    },
    /// 出库单作废
    OutboundVoided {
        outbound_id: i64,
        outbound_no: String,
        prev_status: String,
    },
    /// 异常先发建立
    PreissueCreated {
        preissue_id: i64,
        preissue_no: String,
        wh_id: i64,
        loc_id: i64,
        txn_no: String,
    },
    /// 异常先发闭环(被入库冲销)
    PreissueClosed {
        preissue_id: i64,
        preissue_no: String,
        /// 被触发的单据(入库单 id)
        triggered_by_inbound_id: Option<i64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryLineSummary {
    pub material_id: i64,
    pub batch_no: String,
    #[serde(with = "decimal_as_string")]
    pub qty: Decimal,
    pub io_flag: String,
}

impl DomainEvent {
    /// 事件名,对应 `events.domain_event.event_type`
    #[must_use]
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::InventoryTxnCommitted { .. } => "InventoryTxnCommitted",
            Self::InboundSubmitted { .. } => "InboundSubmitted",
            Self::InboundVoided { .. } => "InboundVoided",
            Self::OutboundSubmitted { .. } => "OutboundSubmitted",
            Self::OutboundVoided { .. } => "OutboundVoided",
            Self::PreissueCreated { .. } => "PreissueCreated",
            Self::PreissueClosed { .. } => "PreissueClosed",
        }
    }

    #[must_use]
    pub fn aggregate_type(&self) -> &'static str {
        match self {
            Self::InventoryTxnCommitted { .. } => "Inventory",
            Self::InboundSubmitted { .. } | Self::InboundVoided { .. } => "Inbound",
            Self::OutboundSubmitted { .. } | Self::OutboundVoided { .. } => "Outbound",
            Self::PreissueCreated { .. } | Self::PreissueClosed { .. } => "Preissue",
        }
    }

    #[must_use]
    pub fn aggregate_id(&self) -> Option<i64> {
        match self {
            Self::InventoryTxnCommitted { txn_id, .. } => Some(*txn_id),
            Self::InboundSubmitted { inbound_id, .. } | Self::InboundVoided { inbound_id, .. } => {
                Some(*inbound_id)
            }
            Self::OutboundSubmitted { outbound_id, .. }
            | Self::OutboundVoided { outbound_id, .. } => Some(*outbound_id),
            Self::PreissueCreated { preissue_id, .. }
            | Self::PreissueClosed { preissue_id, .. } => Some(*preissue_id),
        }
    }
}

mod decimal_as_string {
    use rust_decimal::Decimal;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(d: &Decimal, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&d.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Decimal, D::Error> {
        let s = String::deserialize(de)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}
