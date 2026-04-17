//! Query: 查余额、查流水
//!
//! 这里只放入参结构,真正的执行在 `InventoryRepository`。

use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use cuba_shared::types::StockStatus;

/// 查询余额
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct QueryBalance {
    pub material_id: Option<i64>,
    pub wh_id: Option<i64>,
    pub loc_id: Option<i64>,
    pub batch_no: Option<String>,
    pub stock_status: Option<StockStatus>,
    /// 只看 book_qty > 0 的记录(过滤历史残留 0 行)
    pub only_positive: bool,
}

/// 查询事务流水
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct QueryTxns {
    pub doc_no: Option<String>,
    pub scene_code: Option<String>,
    pub doc_type: Option<String>,
    pub date_from: Option<PrimitiveDateTime>,
    pub date_to: Option<PrimitiveDateTime>,
}
