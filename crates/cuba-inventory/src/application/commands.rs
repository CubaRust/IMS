//! Command: 提交库存事务
//!
//! 业务模块(入库、出库、退料等)调用 `InventoryService::commit(ctx, cmd)`
//! 完成一笔库存变化。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use cuba_shared::types::{IoFlag, StockStatus, TxnType};

use crate::domain::model::{TxnHead, TxnLine, TxnSide};

/// 提交事务命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitTxnCommand {
    pub txn_type: TxnType,
    pub scene_code: String,
    #[serde(default)]
    pub scene_name: Option<String>,
    pub doc_type: String,
    pub doc_no: String,

    #[serde(default)]
    pub source_object_type: Option<String>,
    #[serde(default)]
    pub source_object_id: Option<i64>,
    #[serde(default)]
    pub target_object_type: Option<String>,
    #[serde(default)]
    pub target_object_id: Option<i64>,

    #[serde(default)]
    pub source: Option<TxnSideInput>,
    #[serde(default)]
    pub target: Option<TxnSideInput>,

    pub lines: Vec<TxnLineInput>,

    #[serde(default)]
    pub is_exception: bool,
    #[serde(default)]
    pub exception_type: Option<String>,
    #[serde(default)]
    pub related_doc_no: Option<String>,
    #[serde(default)]
    pub snapshot_json: Option<serde_json::Value>,
    #[serde(default)]
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxnSideInput {
    pub wh_id: i64,
    pub loc_id: i64,
    #[serde(default)]
    pub status: Option<StockStatus>,
}

impl From<TxnSideInput> for TxnSide {
    fn from(v: TxnSideInput) -> Self {
        Self {
            wh_id: v.wh_id,
            loc_id: v.loc_id,
            status: v.status,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxnLineInput {
    pub line_no: i32,
    pub material_id: i64,
    #[serde(default)]
    pub batch_no: String,
    pub qty: Decimal,
    pub unit: String,
    pub io_flag: IoFlag,
    #[serde(default)]
    pub source_material_id: Option<i64>,
    #[serde(default)]
    pub target_material_id: Option<i64>,
    #[serde(default)]
    pub stock_status: Option<StockStatus>,
    #[serde(default)]
    pub status_change_flag: bool,
    #[serde(default)]
    pub location_change_flag: bool,
    #[serde(default)]
    pub item_change_flag: bool,
    #[serde(default)]
    pub recoverable_flag: bool,
    #[serde(default)]
    pub scrap_flag: bool,
    #[serde(default)]
    pub note: Option<String>,
}

impl From<TxnLineInput> for TxnLine {
    fn from(v: TxnLineInput) -> Self {
        Self {
            line_no: v.line_no,
            material_id: v.material_id,
            batch_no: v.batch_no,
            qty: v.qty,
            unit: v.unit,
            io_flag: v.io_flag,
            source_material_id: v.source_material_id,
            target_material_id: v.target_material_id,
            stock_status: v.stock_status,
            status_change_flag: v.status_change_flag,
            location_change_flag: v.location_change_flag,
            item_change_flag: v.item_change_flag,
            recoverable_flag: v.recoverable_flag,
            scrap_flag: v.scrap_flag,
            note: v.note,
        }
    }
}

/// 提交结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitTxnResult {
    pub id: i64,
    pub txn_no: String,
    pub line_count: u32,
}

impl CommitTxnCommand {
    /// 把 command 转成 domain 层的 TxnHead + TxnLine[]
    pub(crate) fn into_domain(
        self,
        operator_id: Option<i64>,
    ) -> (TxnHead, Vec<TxnLine>) {
        let head = TxnHead {
            txn_type: self.txn_type,
            scene_code: self.scene_code,
            scene_name: self.scene_name,
            doc_type: self.doc_type,
            doc_no: self.doc_no,
            source_object_type: self.source_object_type,
            source_object_id: self.source_object_id,
            target_object_type: self.target_object_type,
            target_object_id: self.target_object_id,
            source: self.source.map(Into::into),
            target: self.target.map(Into::into),
            is_exception: self.is_exception,
            exception_type: self.exception_type,
            operator_id,
            related_doc_no: self.related_doc_no,
            snapshot_json: self.snapshot_json.unwrap_or_else(|| serde_json::json!({})),
            remark: self.remark,
        };
        let lines = self.lines.into_iter().map(Into::into).collect();
        (head, lines)
    }
}
