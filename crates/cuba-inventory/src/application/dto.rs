//! 读模型视图
//!
//! 对外 HTTP/JSON 响应用。跟数据库行结构接近,也方便 sqlx row.get() 映射。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use cuba_shared::types::StockStatus;

/// 库存余额视图
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceView {
    pub id: i64,
    pub material_id: i64,
    pub material_code: String,
    pub material_name: String,
    pub wh_id: i64,
    pub wh_code: String,
    pub wh_name: String,
    pub loc_id: i64,
    pub loc_code: String,
    pub loc_name: String,
    pub batch_no: String,
    pub stock_status: StockStatus,
    pub book_qty: Decimal,
    pub available_qty: Decimal,
    pub occupied_qty: Decimal,
    pub bad_qty: Decimal,
    pub scrap_qty: Decimal,
    pub pending_qty: Decimal,
    pub updated_at: PrimitiveDateTime,
}

/// 事务头视图
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxnHeadView {
    pub id: i64,
    pub txn_no: String,
    pub txn_type: String,
    pub scene_code: String,
    pub scene_name: Option<String>,
    pub doc_type: String,
    pub doc_no: String,
    pub source_wh_id: Option<i64>,
    pub source_loc_id: Option<i64>,
    pub target_wh_id: Option<i64>,
    pub target_loc_id: Option<i64>,
    pub source_status: Option<String>,
    pub target_status: Option<String>,
    pub is_exception: bool,
    pub exception_type: Option<String>,
    pub operator_id: Option<i64>,
    pub related_doc_no: Option<String>,
    pub remark: Option<String>,
    pub operate_time: PrimitiveDateTime,
}

/// 事务行视图
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxnLineView {
    pub id: i64,
    pub txn_id: i64,
    pub line_no: i32,
    pub material_id: i64,
    pub material_code: String,
    pub batch_no: String,
    pub qty: Decimal,
    pub unit: String,
    pub io_flag: String,
    pub stock_status: Option<String>,
    pub status_change_flag: bool,
    pub location_change_flag: bool,
    pub item_change_flag: bool,
    pub recoverable_flag: bool,
    pub scrap_flag: bool,
    pub note: Option<String>,
}
