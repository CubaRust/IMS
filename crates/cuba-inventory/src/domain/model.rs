//! 领域模型(值对象)
//!
//! 这些类型只在 domain/application 层流动。infrastructure 层负责在它们和
//! 数据库行之间互转。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use cuba_shared::types::{IoFlag, StockStatus, TxnType};

/// 库存定位键(物料 + 仓 + 位 + 批次 + 状态)
///
/// 对应 DDL 里 `wms_inventory_balance` 的唯一约束。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StockLocator {
    pub material_id: i64,
    pub wh_id: i64,
    pub loc_id: i64,
    /// 不启用批次的物料统一存 ""
    pub batch_no: String,
    pub stock_status: StockStatus,
}

impl StockLocator {
    #[must_use]
    pub fn new(
        material_id: i64,
        wh_id: i64,
        loc_id: i64,
        batch_no: impl Into<String>,
        stock_status: StockStatus,
    ) -> Self {
        Self {
            material_id,
            wh_id,
            loc_id,
            batch_no: batch_no.into(),
            stock_status,
        }
    }
}

/// 事务头描述(领域对象层,不含 id/txn_no,由 infra 生成)
#[derive(Debug, Clone)]
pub struct TxnHead {
    pub txn_type: TxnType,
    pub scene_code: String,
    pub scene_name: Option<String>,
    pub doc_type: String,
    pub doc_no: String,
    pub source_object_type: Option<String>,
    pub source_object_id: Option<i64>,
    pub target_object_type: Option<String>,
    pub target_object_id: Option<i64>,
    pub source: Option<TxnSide>,
    pub target: Option<TxnSide>,
    pub is_exception: bool,
    pub exception_type: Option<String>,
    pub operator_id: Option<i64>,
    pub related_doc_no: Option<String>,
    pub snapshot_json: serde_json::Value,
    pub remark: Option<String>,
}

/// 事务一端(源或目标)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxnSide {
    pub wh_id: i64,
    pub loc_id: i64,
    /// 可选:TRANSFER 里两端状态可能不同(例如 IQC 合格搬到合格库位)
    pub status: Option<StockStatus>,
}

/// 事务行描述(领域对象)
#[derive(Debug, Clone)]
pub struct TxnLine {
    pub line_no: i32,
    pub material_id: i64,
    pub batch_no: String,
    pub qty: Decimal,
    pub unit: String,
    pub io_flag: IoFlag,
    /// 换料场景:记录换之前的物料
    pub source_material_id: Option<i64>,
    /// 换料场景:换之后的物料
    pub target_material_id: Option<i64>,
    pub stock_status: Option<StockStatus>,
    pub status_change_flag: bool,
    pub location_change_flag: bool,
    pub item_change_flag: bool,
    pub recoverable_flag: bool,
    pub scrap_flag: bool,
    pub note: Option<String>,
}

/// 余额 delta(对单个定位键的增减)
///
/// infra 层会把一组 `StockDelta` 折叠(同 locator 相加)然后 upsert 到余额表。
#[derive(Debug, Clone)]
pub struct StockDelta {
    pub locator: StockLocator,
    /// 按业务口径拆分的增量(字段含义与 `wms_inventory_balance` 列名一致)
    pub book: Decimal,
    pub available: Decimal,
    pub occupied: Decimal,
    pub bad: Decimal,
    pub scrap: Decimal,
    pub pending: Decimal,
}

impl StockDelta {
    #[must_use]
    pub fn zero(locator: StockLocator) -> Self {
        Self {
            locator,
            book: Decimal::ZERO,
            available: Decimal::ZERO,
            occupied: Decimal::ZERO,
            bad: Decimal::ZERO,
            scrap: Decimal::ZERO,
            pending: Decimal::ZERO,
        }
    }
}
