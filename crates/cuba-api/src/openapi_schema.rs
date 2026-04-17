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
