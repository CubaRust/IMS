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
        _doc_login,
        _doc_me,
        _doc_logout,
        _doc_refresh,
        _doc_balance,
        _doc_inbound_create,
        _doc_inbound_submit,
        _doc_inbound_list,
        _doc_inbound_get,
        _doc_inbound_void,
        _doc_outbound_create,
        _doc_outbound_submit,
        _doc_outbound_list,
        _doc_preissue_create,
        _doc_preissue_void,
    ),
    components(schemas(
        LoginBody, LoginData,
        BalanceRow,
        InboundCreateBody, InboundCreateLine, SubmitResult, SubmitInboundResult,
        InboundHeadView, InboundLineView,
        OutboundCreateBody, OutboundCreateLine, OutboundHeadView,
        PreissueCreateBody, PreissueCreateLine, PreissueCreateResult,
        ApiErrorEnvelope, VoidData,
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
