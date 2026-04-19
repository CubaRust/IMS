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

/// 空数据占位(utoipa 5 不支持 `()` 作为 schema)
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VoidData {}

// -- auth extensions ---------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ChangePasswordBody {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserView {
    pub id: i64,
    pub user_code: String,
    pub login_name: String,
    pub user_name: String,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RoleView {
    pub id: i64,
    pub role_code: String,
    pub role_name: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PermissionView {
    pub id: i64,
    pub perm_code: String,
    pub perm_name: String,
    pub module_code: String,
}

// -- warehouse ---------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WarehouseView {
    pub id: i64,
    pub wh_code: String,
    pub wh_name: String,
    pub wh_type: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WarehouseCreateBody {
    #[schema(example = "RAW01")]
    pub wh_code: String,
    pub wh_name: String,
    pub wh_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LocationView {
    pub id: i64,
    pub wh_id: i64,
    pub loc_code: String,
    pub loc_name: String,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LocationCreateBody {
    pub wh_id: i64,
    pub loc_code: String,
    pub loc_name: String,
}

// -- catalog (material/supplier/customer/bom/route/status-flow) ---------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MaterialView {
    pub id: i64,
    pub material_code: String,
    pub material_name: String,
    pub material_category: String,
    pub unit: String,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MaterialCreateBody {
    pub material_code: String,
    pub material_name: String,
    pub material_category: String,
    pub unit: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SupplierView {
    pub id: i64,
    pub supplier_code: String,
    pub supplier_name: String,
    pub contact_name: Option<String>,
    pub contact_phone: Option<String>,
    pub address: Option<String>,
    pub is_active: bool,
    pub remark: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SupplierCreateBody {
    #[schema(example = "SUP-001")]
    pub supplier_code: String,
    #[schema(example = "深圳市XX电子有限公司")]
    pub supplier_name: String,
    pub contact_name: Option<String>,
    pub contact_phone: Option<String>,
    pub address: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SupplierUpdateBody {
    #[schema(example = "深圳市XX电子有限公司")]
    pub supplier_name: String,
    pub contact_name: Option<String>,
    pub contact_phone: Option<String>,
    pub address: Option<String>,
    pub is_active: bool,
    pub remark: Option<String>,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CustomerView {
    pub id: i64,
    pub customer_code: String,
    pub customer_name: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BomHeadView {
    pub id: i64,
    pub bom_code: String,
    pub bom_version: String,
    pub product_material_id: i64,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BomCreateBody {
    pub bom_code: String,
    pub bom_version: String,
    pub product_material_id: i64,
    pub lines: Vec<BomCreateLine>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BomCreateLine {
    pub line_no: i32,
    pub material_id: i64,
    #[schema(value_type = String)]
    pub usage_qty: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RouteHeadView {
    pub id: i64,
    pub route_code: String,
    pub route_name: String,
    pub product_material_id: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RouteCreateBody {
    pub route_code: String,
    pub route_name: String,
    pub product_material_id: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StatusFlowView {
    pub id: i64,
    pub source_status: String,
    pub target_status: String,
    pub scene_code: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CustomerCreateBody {
    #[schema(example = "CUS-001")]
    pub customer_code: String,
    pub customer_name: String,
    pub contact_name: Option<String>,
    pub contact_phone: Option<String>,
    pub address: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CustomerUpdateBody {
    pub customer_name: String,
    pub contact_name: Option<String>,
    pub contact_phone: Option<String>,
    pub address: Option<String>,
    pub is_active: bool,
    pub remark: Option<String>,
}

// -- inventory txn -----------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TxnHeadView {
    pub id: i64,
    pub txn_no: String,
    pub txn_type: String,
    pub scene_code: String,
    pub doc_type: String,
    pub doc_no: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TxnLineView {
    pub id: i64,
    pub line_no: i32,
    pub material_id: i64,
    #[schema(value_type = String)]
    pub qty: String,
    pub io_flag: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CommitTxnBody {
    pub txn_type: String,
    pub scene_code: String,
    pub doc_type: String,
    pub doc_no: String,
}

// -- preissue detail ---------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PreissueHeadView {
    pub id: i64,
    pub preissue_no: String,
    pub exception_status: String,
}

// -- defect ------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DefectHeadView {
    pub id: i64,
    pub defect_no: String,
    pub doc_status: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DefectCreateBody {
    pub defect_source: String,
    pub found_date: String,
}

// -- scrap -------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ScrapHeadView {
    pub id: i64,
    pub scrap_no: String,
    pub doc_status: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ScrapCreateBody {
    pub scrap_source: String,
    pub scrap_date: String,
}

// -- recovery ----------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RecoveryHeadView {
    pub id: i64,
    pub recovery_no: String,
    pub doc_status: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RecoveryCreateBody {
    pub source_defect_id: i64,
    pub recovery_date: String,
}

// -- customer-return ---------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CustomerReturnHeadView {
    pub id: i64,
    pub return_no: String,
    pub customer_id: i64,
    pub doc_status: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CustomerReturnCreateBody {
    pub customer_id: i64,
    pub return_date: String,
    pub return_wh_id: i64,
    pub return_loc_id: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct JudgeBody {
    pub line_id: i64,
    pub judge_method: String,
    pub judge_note: Option<String>,
}

// -- supplier-return ---------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SupplierReturnHeadView {
    pub id: i64,
    pub return_no: String,
    pub supplier_id: i64,
    pub doc_status: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SupplierReturnCreateBody {
    pub supplier_id: i64,
    pub return_date: String,
    pub source_wh_id: i64,
    pub source_loc_id: i64,
}

// -- pmc (outsource) ---------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OutsourceHeadView {
    pub id: i64,
    pub outsource_no: String,
    pub supplier_id: i64,
    pub doc_status: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OutsourceCreateBody {
    pub supplier_id: i64,
    pub issue_date: String,
    pub send_wh_id: i64,
    pub send_loc_id: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OutsourceBackBody {
    pub back_lines: Vec<OutsourceBackLine>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OutsourceBackLine {
    pub material_id: i64,
    #[schema(value_type = String)]
    pub qty: String,
    pub unit: String,
}

// -- stocktake ---------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StocktakeHeadView {
    pub id: i64,
    pub stocktake_no: String,
    pub doc_status: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StocktakeCreateBody {
    pub wh_id: i64,
    pub stocktake_date: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StocktakeCountBody {
    pub counts: Vec<StocktakeCountLine>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StocktakeCountLine {
    pub line_id: i64,
    #[schema(value_type = String)]
    pub actual_qty: String,
}

// -- reporting ---------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct InventoryByMaterialRow {
    pub material_id: i64,
    pub material_code: String,
    pub material_name: Option<String>,
    pub material_category: Option<String>,
    #[schema(value_type = String)]
    pub book_qty_total: String,
    #[schema(value_type = String)]
    pub available_qty_total: String,
    #[schema(value_type = String)]
    pub occupied_qty_total: String,
    #[schema(value_type = String)]
    pub bad_qty_total: String,
    #[schema(value_type = String)]
    pub scrap_qty_total: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct InventoryByLocationRow {
    pub id: i64,
    pub material_code: String,
    pub wh_code: String,
    pub loc_code: String,
    pub batch_no: Option<String>,
    pub stock_status: Option<String>,
    #[schema(value_type = String)]
    pub book_qty: String,
    #[schema(value_type = String)]
    pub available_qty: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LowStockWarningRow {
    pub material_id: i64,
    pub material_code: String,
    pub material_name: Option<String>,
    #[schema(value_type = String)]
    pub safety_stock: String,
    #[schema(value_type = String)]
    pub available_qty_total: String,
    #[schema(example = "WARNING")]
    pub warning_level: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AnomalyTodoRow {
    #[schema(example = "PREISSUE")]
    pub anomaly_type: String,
    pub doc_id: i64,
    pub doc_no: String,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TodayIoRow {
    pub txn_id: i64,
    pub txn_no: String,
    pub txn_type: String,
    pub material_code: String,
    #[schema(value_type = String)]
    pub qty: String,
    pub io_flag: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DefectStats30dRow {
    pub material_code: String,
    pub defect_source: Option<String>,
    pub process_method: Option<String>,
    pub line_count: i64,
    #[schema(value_type = String)]
    pub total_qty: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OutsourceInTransitRow {
    pub outsource_id: i64,
    pub outsource_no: String,
    pub supplier_name: Option<String>,
    pub doc_status: String,
    #[schema(value_type = String)]
    pub total_sent_qty: String,
    #[schema(value_type = String)]
    pub in_transit_qty: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DashboardData {
    pub today_inbound_count: i64,
    pub today_outbound_count: i64,
    #[schema(value_type = String)]
    pub today_inbound_qty: String,
    #[schema(value_type = String)]
    pub today_outbound_qty: String,
    pub low_stock_warning_count: i64,
    pub anomaly_todo_count: i64,
    pub outsource_in_transit_count: i64,
    pub defect_pending_count: i64,
    pub total_material_count: i64,
    pub total_sku_with_stock: i64,
}

// -- catalog extensions ------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BomRecommendLine {
    pub line_no: i32,
    pub material_id: i64,
    pub material_code: Option<String>,
    #[schema(value_type = String)]
    pub usage_qty: String,
    #[schema(value_type = String)]
    pub recommend_qty: String,
    pub public_material_flag: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BomRecommendResult {
    pub bom_id: i64,
    pub bom_code: String,
    pub bom_version: String,
    pub product_material_id: i64,
    pub lines: Vec<BomRecommendLine>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RecoveryTplCreateBody {
    #[schema(example = "TPL-001")]
    pub tpl_code: String,
    pub tpl_name: String,
    pub source_material_id: i64,
    pub lines: Vec<RecoveryTplCreateLine>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RecoveryTplCreateLine {
    pub line_no: i32,
    pub target_material_id: Option<i64>,
    #[schema(value_type = String)]
    pub default_recovery_qty: String,
    pub scrap_flag: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RecoveryTplHeadView {
    pub id: i64,
    pub tpl_code: String,
    pub tpl_name: String,
    pub source_material_id: i64,
    pub is_active: bool,
}

// -- system config -----------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DictView {
    pub id: i64,
    pub dict_type: String,
    pub dict_key: String,
    pub dict_value: String,
    pub dict_order: i32,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DictCreateBody {
    #[schema(example = "MATERIAL_CATEGORY")]
    pub dict_type: String,
    #[schema(example = "NEW_KEY")]
    pub dict_key: String,
    pub dict_value: String,
    pub dict_order: i32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DictUpdateBody {
    pub dict_value: Option<String>,
    pub dict_order: Option<i32>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DocNoRuleView {
    pub id: i64,
    pub doc_type: String,
    pub doc_prefix: String,
    pub date_pattern: String,
    pub seq_length: i32,
    pub current_seq: i32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DocNoRuleUpdateBody {
    pub doc_prefix: Option<String>,
    pub date_pattern: Option<String>,
    pub seq_length: Option<i32>,
}
