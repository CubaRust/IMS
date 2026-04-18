//! OpenAPI 的 schema 样例
//!
//! 为什么独立定义:业务 crate 是通用库(service/DTO),不想为它们加 utoipa
//! 依赖。这里用镜像结构体 + `ToSchema`,仅服务 `/docs` 文档渲染,不参与实际
//! 序列化。生产环境使用的结构体仍然是业务 crate 里的那份。

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 统一成功响应包装
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiSuccessEnvelope<T> {
    /// 业务状态码,0 = 成功
    pub code: i32,
    /// 提示信息
    pub message: String,
    /// 数据体
    pub data: Option<T>,
}

/// 错误响应包装
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiErrorEnvelope {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

// -- auth --------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LoginBody {
    #[schema(example = "admin")]
    pub login_name: String,
    #[schema(example = "Admin@123")]
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LoginData {
    pub token: String,
    pub user_id: i64,
    pub login_name: String,
    pub expires_at: i64,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

// -- inventory ---------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BalanceRow {
    pub material_id: i64,
    pub material_code: Option<String>,
    pub wh_id: i64,
    pub loc_id: i64,
    pub batch_no: String,
    pub stock_status: String,
    #[schema(value_type = String, example = "100.0000")]
    pub book_qty: String,
    #[schema(value_type = String, example = "0.0000")]
    pub pending_qty: String,
}

// -- inbound -----------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct InboundCreateBody {
    /// 入库类型:PURCHASE / PROD / RETURN / OUTSOURCE_BACK / CUSTOMER_RETURN / RECOVERY_IN / OTHER
    #[schema(example = "PROD")]
    pub inbound_type: String,
    pub wh_id: i64,
    pub loc_id: Option<i64>,
    #[schema(value_type = String, example = "2026-04-17")]
    pub inbound_date: String,
    pub lines: Vec<InboundCreateLine>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct InboundCreateLine {
    pub line_no: i32,
    pub material_id: i64,
    pub batch_no: String,
    #[schema(value_type = String, example = "100")]
    pub qty: String,
    pub unit: String,
    pub stock_status: Option<String>,
    pub work_order_no: Option<String>,
    pub related_preissue_line_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SubmitResult {
    pub txn_no: String,
    pub doc_status: String,
}

// -- inbound/outbound 列表视图 --------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct InboundHeadView {
    pub id: i64,
    pub inbound_no: String,
    pub inbound_type: String,
    pub wh_id: i64,
    pub loc_id: Option<i64>,
    #[schema(value_type = String, example = "2026-04-17")]
    pub inbound_date: String,
    pub doc_status: String,
    pub lines: Vec<InboundLineView>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct InboundLineView {
    pub id: i64,
    pub line_no: i32,
    pub material_id: i64,
    pub batch_no: String,
    #[schema(value_type = String)]
    pub qty: String,
    pub unit: String,
    pub stock_status: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SubmitInboundResult {
    pub inbound_id: i64,
    pub inbound_no: String,
    pub txn_no: String,
    pub doc_status: String,
}

// -- outbound --------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OutboundCreateBody {
    #[schema(example = "PROD_ISSUE")]
    pub outbound_type: String,
    pub wh_id: i64,
    pub loc_id: i64,
    #[schema(value_type = String, example = "2026-04-17")]
    pub outbound_date: String,
    pub work_order_no: Option<String>,
    pub lines: Vec<OutboundCreateLine>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OutboundCreateLine {
    pub line_no: i32,
    pub material_id: i64,
    pub batch_no: String,
    #[schema(value_type = String)]
    pub suggest_qty: String,
    #[schema(value_type = String)]
    pub actual_qty: String,
    pub unit: String,
    pub stock_status: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OutboundHeadView {
    pub id: i64,
    pub outbound_no: String,
    pub outbound_type: String,
    pub wh_id: i64,
    pub loc_id: i64,
    #[schema(value_type = String, example = "2026-04-17")]
    pub outbound_date: String,
    pub doc_status: String,
}

// -- preissue --------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PreissueCreateBody {
    pub wh_id: i64,
    pub loc_id: i64,
    #[schema(value_type = String, example = "2026-04-17")]
    pub issue_date: String,
    pub reason: String,
    pub work_order_no: Option<String>,
    #[schema(value_type = String, example = "2026-04-24")]
    pub expected_close_date: Option<String>,
    pub lines: Vec<PreissueCreateLine>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PreissueCreateLine {
    pub line_no: i32,
    pub material_id: i64,
    #[schema(value_type = String)]
    pub qty: String,
    pub unit: String,
    pub expected_batch_no: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PreissueCreateResult {
    pub preissue_id: i64,
    pub preissue_no: String,
    pub txn_no: String,
    pub exception_status: String,
}

// -- 通用分页响应 ---------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PageResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}
