//! OpenAPI / Swagger UI 装载
//!
//! 本模块只提供**骨架**:全局 info/tags + 若干端点的签名示例。
//! 生产使用的 DTO 在业务 crate 里定义;此处的 `openapi_schema` 是为
//! 文档渲染而做的镜像类型,避免给业务 crate 引入 utoipa 依赖。

use axum::Router;
use cuba_bootstrap::AppState;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::openapi_schema::*;

/// 登录(示例)
///
/// 返回 JWT,用于后续接口的 `Authorization: Bearer <token>`
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "auth",
    request_body = LoginBody,
    responses(
        (status = 200, description = "成功", body = ApiSuccessEnvelope<LoginData>),
        (status = 200, description = "业务错误(code != 0)", body = ApiErrorEnvelope),
    )
)]
#[allow(dead_code)]
fn _doc_login() {}

/// 查询库存余额(示例)
#[utoipa::path(
    get,
    path = "/api/v1/inventory/balance",
    tag = "inventory",
    params(
        ("wh_id" = Option<i64>, Query, description = "仓库 id"),
        ("loc_id" = Option<i64>, Query, description = "仓位 id"),
        ("material_id" = Option<i64>, Query, description = "物料 id"),
        ("stock_status" = Option<String>, Query, description = "库存状态")
    ),
    responses(
        (status = 200, description = "成功", body = ApiSuccessEnvelope<Vec<BalanceRow>>)
    ),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_balance() {}

/// 创建入库单(示例)
#[utoipa::path(
    post,
    path = "/api/v1/inbounds",
    tag = "inbound",
    request_body = InboundCreateBody,
    responses(
        (status = 200, description = "成功,返回单据 id", body = ApiSuccessEnvelope<SubmitResult>),
    ),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_inbound_create() {}

/// 提交入库单(示例)
#[utoipa::path(
    post,
    path = "/api/v1/inbounds/{id}/submit",
    tag = "inbound",
    params(("id" = i64, Path, description = "入库单 id")),
    responses(
        (status = 200, description = "成功", body = ApiSuccessEnvelope<SubmitResult>),
    ),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_inbound_submit() {}

/// 查询入库单列表
#[utoipa::path(
    get,
    path = "/api/v1/inbounds",
    tag = "inbound",
    params(
        ("inbound_no" = Option<String>, Query, description = "按单号过滤"),
        ("inbound_type" = Option<String>, Query, description = "按类型过滤"),
        ("doc_status" = Option<String>, Query, description = "按状态过滤"),
        ("date_from" = Option<String>, Query, description = "日期起 YYYY-MM-DD"),
        ("date_to" = Option<String>, Query, description = "日期止 YYYY-MM-DD"),
    ),
    responses(
        (status = 200, description = "成功", body = ApiSuccessEnvelope<Vec<InboundHeadView>>),
    ),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_inbound_list() {}

/// 查询入库单详情
#[utoipa::path(
    get,
    path = "/api/v1/inbounds/{id}",
    tag = "inbound",
    params(("id" = i64, Path, description = "入库单 id")),
    responses(
        (status = 200, description = "成功", body = ApiSuccessEnvelope<InboundHeadView>),
        (status = 200, description = "不存在(code=10404)", body = ApiErrorEnvelope),
    ),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_inbound_get() {}

/// 作废入库单(仅 DRAFT/SUBMITTED)
#[utoipa::path(
    post,
    path = "/api/v1/inbounds/{id}/void",
    tag = "inbound",
    params(("id" = i64, Path, description = "入库单 id")),
    responses(
        (status = 200, description = "成功", body = ApiSuccessEnvelope<VoidData>),
        (status = 200, description = "状态非法(code=30103)", body = ApiErrorEnvelope),
    ),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_inbound_void() {}

// -- outbound --------------------------------------------------------------

/// 创建出库单
#[utoipa::path(
    post,
    path = "/api/v1/outbounds",
    tag = "outbound",
    request_body = OutboundCreateBody,
    responses(
        (status = 200, description = "成功,返回单据", body = ApiSuccessEnvelope<OutboundHeadView>),
    ),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_outbound_create() {}

/// 提交出库单(核心扣库存动作,可能抛 20101 库存不足)
#[utoipa::path(
    post,
    path = "/api/v1/outbounds/{id}/submit",
    tag = "outbound",
    params(("id" = i64, Path, description = "出库单 id")),
    responses(
        (status = 200, description = "成功", body = ApiSuccessEnvelope<SubmitResult>),
        (status = 200, description = "库存不足(code=20101)", body = ApiErrorEnvelope),
    ),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_outbound_submit() {}

/// 查询出库单列表
#[utoipa::path(
    get,
    path = "/api/v1/outbounds",
    tag = "outbound",
    responses(
        (status = 200, description = "成功", body = ApiSuccessEnvelope<Vec<OutboundHeadView>>),
    ),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_outbound_list() {}

// -- preissue --------------------------------------------------------------

/// 创建异常先发单(会产生 PREISSUE_PENDING 状态的负库存)
#[utoipa::path(
    post,
    path = "/api/v1/preissues",
    tag = "preissue",
    request_body = PreissueCreateBody,
    responses(
        (status = 200, description = "成功,负库存已生效", body = ApiSuccessEnvelope<PreissueCreateResult>),
    ),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_preissue_create() {}

/// 作废异常先发(仅 PENDING/PARTIAL,会回冲负库存)
#[utoipa::path(
    post,
    path = "/api/v1/preissues/{id}/void",
    tag = "preissue",
    params(("id" = i64, Path, description = "preissue id")),
    responses(
        (status = 200, description = "成功", body = ApiSuccessEnvelope<VoidData>),
    ),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_preissue_void() {}

// -- auth 扩展 -----------------------------------------------------------

/// 当前用户信息
#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    tag = "auth",
    responses(
        (status = 200, description = "成功", body = ApiSuccessEnvelope<LoginData>),
    ),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_me() {}

/// 登出(当前 token 拉黑)
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "auth",
    responses(
        (status = 200, description = "成功", body = ApiSuccessEnvelope<VoidData>),
    ),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_logout() {}

/// 刷新 token(旧 token 立即失效)
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    tag = "auth",
    responses(
        (status = 200, description = "成功,返回新 token", body = ApiSuccessEnvelope<LoginData>),
    ),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_refresh() {}

// -- auth extensions ---------------------------------------------------------

/// 修改密码
#[utoipa::path(put, path = "/api/v1/auth/password", tag = "auth",
    request_body = ChangePasswordBody,
    responses((status = 200, body = ApiSuccessEnvelope<VoidData>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_change_password() {}

/// 用户列表
#[utoipa::path(get, path = "/api/v1/users", tag = "auth",
    responses((status = 200, body = ApiSuccessEnvelope<Vec<UserView>>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_users() {}

/// 角色列表
#[utoipa::path(get, path = "/api/v1/roles", tag = "auth",
    responses((status = 200, body = ApiSuccessEnvelope<Vec<RoleView>>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_roles() {}

/// 权限列表
#[utoipa::path(get, path = "/api/v1/permissions", tag = "auth",
    responses((status = 200, body = ApiSuccessEnvelope<Vec<PermissionView>>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_permissions() {}

// -- warehouse ---------------------------------------------------------------

/// 仓库列表
#[utoipa::path(get, path = "/api/v1/warehouses", tag = "warehouse",
    responses((status = 200, body = ApiSuccessEnvelope<Vec<WarehouseView>>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_wh_list() {}

/// 创建仓库
#[utoipa::path(post, path = "/api/v1/warehouses", tag = "warehouse",
    request_body = WarehouseCreateBody,
    responses((status = 200, body = ApiSuccessEnvelope<WarehouseView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_wh_create() {}

/// 仓库详情
#[utoipa::path(get, path = "/api/v1/warehouses/{id}", tag = "warehouse",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<WarehouseView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_wh_get() {}

/// 修改仓库
#[utoipa::path(put, path = "/api/v1/warehouses/{id}", tag = "warehouse",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<WarehouseView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_wh_update() {}

/// 仓位列表
#[utoipa::path(get, path = "/api/v1/locations", tag = "warehouse",
    responses((status = 200, body = ApiSuccessEnvelope<Vec<LocationView>>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_loc_list() {}

/// 创建仓位
#[utoipa::path(post, path = "/api/v1/locations", tag = "warehouse",
    request_body = LocationCreateBody,
    responses((status = 200, body = ApiSuccessEnvelope<LocationView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_loc_create() {}

/// 仓位详情
#[utoipa::path(get, path = "/api/v1/locations/{id}", tag = "warehouse",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<LocationView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_loc_get() {}

/// 修改仓位
#[utoipa::path(put, path = "/api/v1/locations/{id}", tag = "warehouse",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<LocationView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_loc_update() {}

// -- catalog (material/supplier/customer/bom/route/status-flow) ---------------

/// 物料列表
#[utoipa::path(get, path = "/api/v1/materials", tag = "catalog",
    responses((status = 200, body = ApiSuccessEnvelope<PageResponse<MaterialView>>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_mat_list() {}
/// 创建物料
#[utoipa::path(post, path = "/api/v1/materials", tag = "catalog",
    request_body = MaterialCreateBody,
    responses((status = 200, body = ApiSuccessEnvelope<MaterialView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_mat_create() {}
/// 物料详情
#[utoipa::path(get, path = "/api/v1/materials/{id}", tag = "catalog",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<MaterialView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_mat_get() {}
/// 修改物料
#[utoipa::path(put, path = "/api/v1/materials/{id}", tag = "catalog",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<MaterialView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_mat_update() {}

/// 供应商列表
#[utoipa::path(get, path = "/api/v1/suppliers", tag = "catalog",
    responses((status = 200, body = ApiSuccessEnvelope<Vec<SupplierView>>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_sup_list() {}
/// 创建供应商
#[utoipa::path(post, path = "/api/v1/suppliers", tag = "catalog",
    request_body = SupplierCreateBody,
    responses((status = 200, body = ApiSuccessEnvelope<SupplierView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_sup_create() {}
/// 供应商详情
#[utoipa::path(get, path = "/api/v1/suppliers/{id}", tag = "catalog",
    params(("id" = i64, Path)),
    request_body = SupplierUpdateBody,
    responses((status = 200, body = ApiSuccessEnvelope<SupplierView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_sup_get() {}
/// 修改供应商
#[utoipa::path(put, path = "/api/v1/suppliers/{id}", tag = "catalog",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<SupplierView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_sup_update() {}
/// 停用供应商(软删除)
#[utoipa::path(
    delete,
    path = "/api/v1/suppliers/{id}", tag = "catalog",
    params(("id" = i64, Path, description = "供应商 id")),
    responses(
        (status = 200, description = "成功",
         body = crate::openapi_schema::ApiSuccessEnvelope<crate::openapi_schema::VoidData>)
    ),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_sup_delete() {}
/// 客户列表
#[utoipa::path(get, path = "/api/v1/customers", tag = "catalog",
    responses((status = 200, body = ApiSuccessEnvelope<Vec<CustomerView>>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_cust_list() {}
/// 创建客户
#[utoipa::path(post, path = "/api/v1/customers", tag = "catalog",
    request_body = CustomerCreateBody,
    responses((status = 200, body = ApiSuccessEnvelope<CustomerView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_cust_create() {}
/// 客户详情
#[utoipa::path(get, path = "/api/v1/customers/{id}", tag = "catalog",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<CustomerView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_cust_get() {}
/// 修改客户
#[utoipa::path(put, path = "/api/v1/customers/{id}", tag = "catalog",
    params(("id" = i64, Path)),
    request_body = CustomerUpdateBody,
    responses((status = 200, body = ApiSuccessEnvelope<CustomerView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_cust_update() {}

/// BOM 列表
#[utoipa::path(get, path = "/api/v1/boms", tag = "catalog",
    responses((status = 200, body = ApiSuccessEnvelope<Vec<BomHeadView>>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_bom_list() {}
/// 创建 BOM
#[utoipa::path(post, path = "/api/v1/boms", tag = "catalog",
    request_body = BomCreateBody,
    responses((status = 200, body = ApiSuccessEnvelope<BomHeadView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_bom_create() {}
/// BOM 详情
#[utoipa::path(get, path = "/api/v1/boms/{id}", tag = "catalog",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<BomHeadView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_bom_get() {}

/// 工艺路线列表
#[utoipa::path(get, path = "/api/v1/routes", tag = "catalog",
    responses((status = 200, body = ApiSuccessEnvelope<Vec<RouteHeadView>>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_route_list() {}
/// 创建工艺路线
#[utoipa::path(post, path = "/api/v1/routes", tag = "catalog",
    request_body = RouteCreateBody,
    responses((status = 200, body = ApiSuccessEnvelope<RouteHeadView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_route_create() {}
/// 工艺路线详情
#[utoipa::path(get, path = "/api/v1/routes/{id}", tag = "catalog",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<RouteHeadView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_route_get() {}

/// 状态流转规则
#[utoipa::path(get, path = "/api/v1/status-flows", tag = "catalog",
    responses((status = 200, body = ApiSuccessEnvelope<Vec<StatusFlowView>>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_status_flows() {}

// -- inventory extensions ----------------------------------------------------

/// 原始事务提交(危险接口)
#[utoipa::path(post, path = "/api/v1/inventory/txn", tag = "inventory",
    request_body = CommitTxnBody,
    responses((status = 200, body = ApiSuccessEnvelope<SubmitResult>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_inv_commit() {}
/// 事务流水列表
#[utoipa::path(get, path = "/api/v1/inventory/txn", tag = "inventory",
    responses((status = 200, body = ApiSuccessEnvelope<Vec<TxnHeadView>>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_inv_txn_list() {}
/// 事务明细行
#[utoipa::path(get, path = "/api/v1/inventory/txn/{id}", tag = "inventory",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<Vec<TxnLineView>>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_inv_txn_lines() {}

// -- outbound extensions -----------------------------------------------------

/// 出库单详情
#[utoipa::path(get, path = "/api/v1/outbounds/{id}", tag = "outbound",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<OutboundHeadView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_outbound_get() {}
/// 作废出库单
#[utoipa::path(post, path = "/api/v1/outbounds/{id}/void", tag = "outbound",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<VoidData>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_outbound_void() {}

// -- preissue extensions -----------------------------------------------------

/// 异常先发列表
#[utoipa::path(get, path = "/api/v1/preissues", tag = "preissue",
    responses((status = 200, body = ApiSuccessEnvelope<Vec<PreissueHeadView>>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_preissue_list() {}
/// 异常先发详情
#[utoipa::path(get, path = "/api/v1/preissues/{id}", tag = "preissue",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<PreissueHeadView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_preissue_get() {}

// -- defect ------------------------------------------------------------------

/// 不良单列表
#[utoipa::path(get, path = "/api/v1/defects", tag = "defect",
    responses((status = 200, body = ApiSuccessEnvelope<Vec<DefectHeadView>>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_defect_list() {}
/// 创建不良单
#[utoipa::path(post, path = "/api/v1/defects", tag = "defect",
    request_body = DefectCreateBody,
    responses((status = 200, body = ApiSuccessEnvelope<DefectHeadView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_defect_create() {}
/// 不良单详情
#[utoipa::path(get, path = "/api/v1/defects/{id}", tag = "defect",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<DefectHeadView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_defect_get() {}
/// 提交不良单
#[utoipa::path(post, path = "/api/v1/defects/{id}/submit", tag = "defect",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<SubmitResult>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_defect_submit() {}
/// 作废不良单
#[utoipa::path(post, path = "/api/v1/defects/{id}/void", tag = "defect",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<VoidData>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_defect_void() {}

// -- scrap -------------------------------------------------------------------

/// 报废列表
#[utoipa::path(get, path = "/api/v1/scraps", tag = "scrap",
    responses((status = 200, body = ApiSuccessEnvelope<Vec<ScrapHeadView>>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_scrap_list() {}
/// 创建报废
#[utoipa::path(post, path = "/api/v1/scraps", tag = "scrap",
    request_body = ScrapCreateBody,
    responses((status = 200, body = ApiSuccessEnvelope<ScrapHeadView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_scrap_create() {}
/// 报废详情
#[utoipa::path(get, path = "/api/v1/scraps/{id}", tag = "scrap",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<ScrapHeadView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_scrap_get() {}
/// 提交报废
#[utoipa::path(post, path = "/api/v1/scraps/{id}/submit", tag = "scrap",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<SubmitResult>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_scrap_submit() {}
/// 作废报废单
#[utoipa::path(post, path = "/api/v1/scraps/{id}/void", tag = "scrap",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<VoidData>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_scrap_void() {}

// -- recovery ----------------------------------------------------------------

/// 拆解回收列表
#[utoipa::path(get, path = "/api/v1/recoveries", tag = "recovery",
    responses((status = 200, body = ApiSuccessEnvelope<Vec<RecoveryHeadView>>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_recovery_list() {}
/// 创建拆解回收
#[utoipa::path(post, path = "/api/v1/recoveries", tag = "recovery",
    request_body = RecoveryCreateBody,
    responses((status = 200, body = ApiSuccessEnvelope<RecoveryHeadView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_recovery_create() {}
/// 拆解回收详情
#[utoipa::path(get, path = "/api/v1/recoveries/{id}", tag = "recovery",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<RecoveryHeadView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_recovery_get() {}
/// 提交拆解回收
#[utoipa::path(post, path = "/api/v1/recoveries/{id}/submit", tag = "recovery",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<SubmitResult>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_recovery_submit() {}
/// 作废拆解回收
#[utoipa::path(post, path = "/api/v1/recoveries/{id}/void", tag = "recovery",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<VoidData>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_recovery_void() {}

// -- customer-return ---------------------------------------------------------

/// 客退列表
#[utoipa::path(get, path = "/api/v1/customer-returns", tag = "customer-return",
    responses((status = 200, body = ApiSuccessEnvelope<Vec<CustomerReturnHeadView>>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_cr_list() {}
/// 创建客退
#[utoipa::path(post, path = "/api/v1/customer-returns", tag = "customer-return",
    request_body = CustomerReturnCreateBody,
    responses((status = 200, body = ApiSuccessEnvelope<CustomerReturnHeadView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_cr_create() {}
/// 客退详情
#[utoipa::path(get, path = "/api/v1/customer-returns/{id}", tag = "customer-return",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<CustomerReturnHeadView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_cr_get() {}
/// 客退判定
#[utoipa::path(post, path = "/api/v1/customer-returns/{id}/judge", tag = "customer-return",
    params(("id" = i64, Path)),
    request_body = Vec<JudgeBody>,
    responses((status = 200, body = ApiSuccessEnvelope<VoidData>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_cr_judge() {}
/// 客退提交
#[utoipa::path(post, path = "/api/v1/customer-returns/{id}/submit", tag = "customer-return",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<SubmitResult>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_cr_submit() {}
/// 客退作废
#[utoipa::path(post, path = "/api/v1/customer-returns/{id}/void", tag = "customer-return",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<VoidData>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_cr_void() {}

// -- supplier-return ---------------------------------------------------------

/// 退供列表
#[utoipa::path(get, path = "/api/v1/supplier-returns", tag = "supplier-return",
    responses((status = 200, body = ApiSuccessEnvelope<Vec<SupplierReturnHeadView>>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_sr_list() {}
/// 创建退供
#[utoipa::path(post, path = "/api/v1/supplier-returns", tag = "supplier-return",
    request_body = SupplierReturnCreateBody,
    responses((status = 200, body = ApiSuccessEnvelope<SupplierReturnHeadView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_sr_create() {}
/// 退供详情
#[utoipa::path(get, path = "/api/v1/supplier-returns/{id}", tag = "supplier-return",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<SupplierReturnHeadView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_sr_get() {}
/// 退供提交
#[utoipa::path(post, path = "/api/v1/supplier-returns/{id}/submit", tag = "supplier-return",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<SubmitResult>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_sr_submit() {}
/// 退供作废
#[utoipa::path(post, path = "/api/v1/supplier-returns/{id}/void", tag = "supplier-return",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<VoidData>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_sr_void() {}

// -- pmc (outsource) ---------------------------------------------------------

/// 委外列表
#[utoipa::path(get, path = "/api/v1/outsources", tag = "pmc",
    responses((status = 200, body = ApiSuccessEnvelope<Vec<OutsourceHeadView>>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_os_list() {}
/// 创建委外
#[utoipa::path(post, path = "/api/v1/outsources", tag = "pmc",
    request_body = OutsourceCreateBody,
    responses((status = 200, body = ApiSuccessEnvelope<OutsourceHeadView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_os_create() {}
/// 委外详情
#[utoipa::path(get, path = "/api/v1/outsources/{id}", tag = "pmc",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<OutsourceHeadView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_os_get() {}
/// 委外发出
#[utoipa::path(post, path = "/api/v1/outsources/{id}/send", tag = "pmc",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<SubmitResult>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_os_send() {}
/// 委外收回
#[utoipa::path(post, path = "/api/v1/outsources/{id}/back", tag = "pmc",
    params(("id" = i64, Path)),
    request_body = OutsourceBackBody,
    responses((status = 200, body = ApiSuccessEnvelope<SubmitResult>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_os_back() {}
/// 委外作废
#[utoipa::path(post, path = "/api/v1/outsources/{id}/void", tag = "pmc",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<VoidData>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_os_void() {}

// -- stocktake ---------------------------------------------------------------

/// 盘点列表
#[utoipa::path(get, path = "/api/v1/stocktakes", tag = "stocktake",
    responses((status = 200, body = ApiSuccessEnvelope<Vec<StocktakeHeadView>>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_st_list() {}
/// 创建盘点
#[utoipa::path(post, path = "/api/v1/stocktakes", tag = "stocktake",
    request_body = StocktakeCreateBody,
    responses((status = 200, body = ApiSuccessEnvelope<StocktakeHeadView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_st_create() {}
/// 盘点详情
#[utoipa::path(get, path = "/api/v1/stocktakes/{id}", tag = "stocktake",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<StocktakeHeadView>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_st_get() {}
/// 录入实盘数
#[utoipa::path(post, path = "/api/v1/stocktakes/{id}/counts", tag = "stocktake",
    params(("id" = i64, Path)),
    request_body = StocktakeCountBody,
    responses((status = 200, body = ApiSuccessEnvelope<VoidData>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_st_count() {}
/// 提交盘点差异
#[utoipa::path(post, path = "/api/v1/stocktakes/{id}/submit", tag = "stocktake",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<SubmitResult>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_st_submit() {}
/// 作废盘点
#[utoipa::path(post, path = "/api/v1/stocktakes/{id}/void", tag = "stocktake",
    params(("id" = i64, Path)),
    responses((status = 200, body = ApiSuccessEnvelope<VoidData>)),
    security(("bearer" = [])))]
#[allow(dead_code)]
fn _doc_st_void() {}

/// 按物料汇总库存
#[utoipa::path(
    get,
    path = "/api/v1/reports/inventory-by-material",
    tag = "reporting",
    params(
        ("material_category" = Option<String>, Query, description = "物料类别"),
        ("process_type" = Option<String>, Query, description = "工艺类型"),
        ("material_code" = Option<String>, Query, description = "物料编码(模糊)"),
    ),
    responses((status = 200, body = ApiSuccessEnvelope<Vec<InventoryByMaterialRow>>)),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_rpt_inv_material() {}

/// 按仓位库存明细
#[utoipa::path(
    get,
    path = "/api/v1/reports/inventory-by-location",
    tag = "reporting",
    params(
        ("wh_code" = Option<String>, Query, description = "仓库编码"),
        ("material_code" = Option<String>, Query, description = "物料编码(模糊)"),
        ("stock_status" = Option<String>, Query, description = "库存状态"),
    ),
    responses((status = 200, body = ApiSuccessEnvelope<Vec<InventoryByLocationRow>>)),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_rpt_inv_location() {}

/// 低库存预警
#[utoipa::path(
    get,
    path = "/api/v1/reports/low-stock-warning",
    tag = "reporting",
    params(
        ("warning_level" = Option<String>, Query, description = "WARNING / CRITICAL"),
        ("material_category" = Option<String>, Query, description = "物料类别"),
    ),
    responses((status = 200, body = ApiSuccessEnvelope<Vec<LowStockWarningRow>>)),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_rpt_low_stock() {}

/// 异常待办
#[utoipa::path(
    get,
    path = "/api/v1/reports/anomaly-todo",
    tag = "reporting",
    params(
        ("anomaly_type" = Option<String>, Query, description = "PREISSUE / DEFECT_PENDING / CUSTOMER_RETURN_PENDING_JUDGE / STOCKTAKE_DIFF"),
    ),
    responses((status = 200, body = ApiSuccessEnvelope<Vec<AnomalyTodoRow>>)),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_rpt_anomaly() {}

/// 当日出入库流水
#[utoipa::path(
    get,
    path = "/api/v1/reports/today-io",
    tag = "reporting",
    params(
        ("txn_type" = Option<String>, Query, description = "IN / OUT / TRANSFER"),
        ("scene_code" = Option<String>, Query, description = "场景码"),
    ),
    responses((status = 200, body = ApiSuccessEnvelope<Vec<TodayIoRow>>)),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_rpt_today_io() {}

/// 不良统计(近30天)
#[utoipa::path(
    get,
    path = "/api/v1/reports/defect-stats",
    tag = "reporting",
    params(
        ("material_code" = Option<String>, Query, description = "物料编码(模糊)"),
        ("defect_source" = Option<String>, Query, description = "不良来源"),
    ),
    responses((status = 200, body = ApiSuccessEnvelope<Vec<DefectStats30dRow>>)),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_rpt_defect_stats() {}

/// 委外在途
#[utoipa::path(
    get,
    path = "/api/v1/reports/outsource-in-transit",
    tag = "reporting",
    params(
        ("supplier_code" = Option<String>, Query, description = "供应商编码"),
        ("doc_status" = Option<String>, Query, description = "单据状态"),
    ),
    responses((status = 200, body = ApiSuccessEnvelope<Vec<OutsourceInTransitRow>>)),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_rpt_outsource() {}

/// 首页看板聚合
#[utoipa::path(
    get,
    path = "/api/v1/dashboard",
    tag = "reporting",
    responses((status = 200, body = ApiSuccessEnvelope<DashboardData>)),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_dashboard() {}

// -- catalog extensions ------------------------------------------------------

/// BOM 展开推荐发料
#[utoipa::path(
    get,
    path = "/api/v1/boms/recommend",
    tag = "catalog",
    params(
        ("product_material_id" = i64, Query, description = "产品物料 id"),
        ("production_qty" = Option<String>, Query, description = "生产数量(默认 1)"),
        ("bom_id" = Option<i64>, Query, description = "指定 BOM id(可选,默认取最新激活)"),
    ),
    responses(
        (status = 200, body = ApiSuccessEnvelope<BomRecommendResult>),
        (status = 200, description = "无激活 BOM(code=10404)", body = ApiErrorEnvelope),
    ),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_bom_recommend() {}

/// 回收模板列表
#[utoipa::path(
    get,
    path = "/api/v1/recovery-templates",
    tag = "catalog",
    params(
        ("tpl_code" = Option<String>, Query, description = "模板编码"),
        ("source_material_id" = Option<i64>, Query, description = "源物料 id"),
        ("is_active" = Option<bool>, Query, description = "是否激活"),
    ),
    responses((status = 200, body = ApiSuccessEnvelope<Vec<RecoveryTplHeadView>>)),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_recovery_tpl_list() {}

/// 创建回收模板
#[utoipa::path(
    post,
    path = "/api/v1/recovery-templates",
    tag = "catalog",
    request_body = RecoveryTplCreateBody,
    responses((status = 200, body = ApiSuccessEnvelope<RecoveryTplHeadView>)),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_recovery_tpl_create() {}

/// 回收模板详情
#[utoipa::path(
    get,
    path = "/api/v1/recovery-templates/{id}",
    tag = "catalog",
    params(("id" = i64, Path, description = "模板 id")),
    responses((status = 200, body = ApiSuccessEnvelope<RecoveryTplHeadView>)),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_recovery_tpl_get() {}

// -- system config -----------------------------------------------------------

/// 数据字典列表
#[utoipa::path(
    get,
    path = "/api/v1/dicts",
    tag = "auth",
    params(("dict_type" = Option<String>, Query, description = "字典类型")),
    responses((status = 200, body = ApiSuccessEnvelope<Vec<DictView>>)),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_dict_list() {}

/// 新增字典项
#[utoipa::path(
    post,
    path = "/api/v1/dicts",
    tag = "auth",
    request_body = DictCreateBody,
    responses((status = 200, body = ApiSuccessEnvelope<DictView>)),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_dict_create() {}

/// 修改字典项
#[utoipa::path(
    put,
    path = "/api/v1/dicts/{id}",
    tag = "auth",
    params(("id" = i64, Path, description = "字典 id")),
    request_body = DictUpdateBody,
    responses((status = 200, body = ApiSuccessEnvelope<DictView>)),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_dict_update() {}

/// 编码规则列表
#[utoipa::path(
    get,
    path = "/api/v1/doc-no-rules",
    tag = "auth",
    responses((status = 200, body = ApiSuccessEnvelope<Vec<DocNoRuleView>>)),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_rule_list() {}

/// 修改编码规则
#[utoipa::path(
    put,
    path = "/api/v1/doc-no-rules/{id}",
    tag = "auth",
    params(("id" = i64, Path, description = "规则 id")),
    request_body = DocNoRuleUpdateBody,
    responses((status = 200, body = ApiSuccessEnvelope<DocNoRuleView>)),
    security(("bearer" = []))
)]
#[allow(dead_code)]
fn _doc_rule_update() {}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "EQYCC CUBA IMS API",
        version = "0.4.0",
        description = "手机屏幕组装厂 WMS 后端。\n\n\
            约定:\n\
            - 所有业务成功响应返回 `{code:0, message:\"ok\", data:...}`\n\
            - 业务失败走 HTTP 200 + code != 0\n\
            - 仅 401/403/500 使用对应 HTTP status\n\
            - JWT 走 `Authorization: Bearer <token>`\n\
            - 数量字段统一字符串传 Decimal 避免精度丢失",
        contact(name = "EQYCC")
    ),
    servers((url = "/", description = "当前环境")),
    paths(
        // auth
        _doc_login, _doc_me, _doc_logout, _doc_refresh, _doc_change_password,
        _doc_users, _doc_roles, _doc_permissions,
        _doc_dict_list, _doc_dict_create, _doc_dict_update,
        _doc_rule_list, _doc_rule_update,
        // warehouse
        _doc_wh_list, _doc_wh_create, _doc_wh_get, _doc_wh_update,
        _doc_loc_list, _doc_loc_create, _doc_loc_get, _doc_loc_update,
        // catalog
        _doc_mat_list, _doc_mat_create, _doc_mat_get, _doc_mat_update,
        _doc_sup_list, _doc_sup_create, _doc_sup_get, _doc_sup_update, _doc_sup_delete,
        _doc_cust_list, _doc_cust_create, _doc_cust_get, _doc_cust_update,
        _doc_bom_list, _doc_bom_create, _doc_bom_get, _doc_bom_recommend,
        _doc_route_list, _doc_route_create, _doc_route_get,
        _doc_status_flows,
        _doc_recovery_tpl_list, _doc_recovery_tpl_create, _doc_recovery_tpl_get,
        // inventory
        _doc_balance, _doc_inv_commit, _doc_inv_txn_list, _doc_inv_txn_lines,
        // inbound
        _doc_inbound_list, _doc_inbound_create, _doc_inbound_get,
        _doc_inbound_submit, _doc_inbound_void,
        // outbound
        _doc_outbound_list, _doc_outbound_create, _doc_outbound_get,
        _doc_outbound_submit, _doc_outbound_void,
        // preissue
        _doc_preissue_list, _doc_preissue_get, _doc_preissue_create, _doc_preissue_void,
        // defect
        _doc_defect_list, _doc_defect_create, _doc_defect_get,
        _doc_defect_submit, _doc_defect_void,
        // scrap
        _doc_scrap_list, _doc_scrap_create, _doc_scrap_get,
        _doc_scrap_submit, _doc_scrap_void,
        // recovery
        _doc_recovery_list, _doc_recovery_create, _doc_recovery_get,
        _doc_recovery_submit, _doc_recovery_void,
        // customer-return
        _doc_cr_list, _doc_cr_create, _doc_cr_get,
        _doc_cr_judge, _doc_cr_submit, _doc_cr_void,
        // supplier-return
        _doc_sr_list, _doc_sr_create, _doc_sr_get, _doc_sr_submit, _doc_sr_void,
        // pmc
        _doc_os_list, _doc_os_create, _doc_os_get,
        _doc_os_send, _doc_os_back, _doc_os_void,
        // stocktake
        _doc_st_list, _doc_st_create, _doc_st_get,
        _doc_st_count, _doc_st_submit, _doc_st_void,
        // reporting
        _doc_rpt_inv_material, _doc_rpt_inv_location, _doc_rpt_low_stock,
        _doc_rpt_anomaly, _doc_rpt_today_io, _doc_rpt_defect_stats,
        _doc_rpt_outsource, _doc_dashboard,
    ),
    components(schemas(
        ApiErrorEnvelope, VoidData, PageResponse<MaterialView>,
        // auth
        LoginBody, LoginData, ChangePasswordBody,
        UserView, RoleView, PermissionView,
        DictView, DictCreateBody, DictUpdateBody,
        DocNoRuleView, DocNoRuleUpdateBody,
        // warehouse
        WarehouseView, WarehouseCreateBody, LocationView, LocationCreateBody,
        // catalog
        MaterialView, MaterialCreateBody,
        SupplierView, CustomerView,
        BomHeadView, BomCreateBody, BomCreateLine,
        BomRecommendResult, BomRecommendLine,
        RouteHeadView, RouteCreateBody, StatusFlowView,
        RecoveryTplHeadView, RecoveryTplCreateBody, RecoveryTplCreateLine,
        // inventory
        BalanceRow, CommitTxnBody, TxnHeadView, TxnLineView, SubmitResult,
        // inbound / outbound
        InboundCreateBody, InboundCreateLine, InboundHeadView, InboundLineView, SubmitInboundResult,
        OutboundCreateBody, OutboundCreateLine, OutboundHeadView,
        // preissue
        PreissueCreateBody, PreissueCreateLine, PreissueCreateResult, PreissueHeadView,
        // defect / scrap / recovery
        DefectHeadView, DefectCreateBody,
        ScrapHeadView, ScrapCreateBody,
        RecoveryHeadView, RecoveryCreateBody,
        // customer-return / supplier-return
        CustomerReturnHeadView, CustomerReturnCreateBody, JudgeBody,
        SupplierReturnHeadView, SupplierReturnCreateBody,
        // pmc
        OutsourceHeadView, OutsourceCreateBody, OutsourceBackBody, OutsourceBackLine,
        // stocktake
        StocktakeHeadView, StocktakeCreateBody, StocktakeCountBody, StocktakeCountLine,
        // reporting
        InventoryByMaterialRow, InventoryByLocationRow, LowStockWarningRow,
        AnomalyTodoRow, TodayIoRow, DefectStats30dRow, OutsourceInTransitRow,
        DashboardData,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "auth", description = "登录/用户/权限"),
        (name = "warehouse", description = "仓库/仓位"),
        (name = "catalog", description = "主数据:物料/供应商/客户/BOM/工艺路线"),
        (name = "inventory", description = "库存查询 + 原始事务提交"),
        (name = "inbound", description = "入库单"),
        (name = "outbound", description = "出库单"),
        (name = "preissue", description = "异常先发(负库存闭环)"),
        (name = "defect", description = "不良单"),
        (name = "recovery", description = "拆解回收"),
        (name = "scrap", description = "报废"),
        (name = "customer-return", description = "客户退货"),
        (name = "supplier-return", description = "退供商"),
        (name = "pmc", description = "委外(送/回)"),
        (name = "stocktake", description = "盘点"),
        (name = "reporting", description = "报表"),
    )
)]
pub struct ApiDoc;

struct SecurityAddon;
impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
        if openapi.components.is_none() {
            openapi.components = Some(utoipa::openapi::Components::new());
        }
        let comp = openapi.components.as_mut().unwrap();
        comp.add_security_scheme(
            "bearer",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}

/// 生成 swagger UI router(不持有应用 state)
///
/// 访问:`GET /docs`
/// OpenAPI JSON:`GET /api-docs/openapi.json`
pub fn swagger_router() -> Router<AppState> {
    SwaggerUi::new("/docs")
        .url("/api-docs/openapi.json", ApiDoc::openapi())
        .into()
}
