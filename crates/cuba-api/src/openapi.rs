//! OpenAPI / Swagger UI 装载
//!
//! 本模块只提供**骨架**:全局 info/tags + 若干端点的签名示例。
//! 生产使用的 DTO 在业务 crate 里定义;此处的 `openapi_schema` 是为
//! 文档渲染而做的镜像类型,避免给业务 crate 引入 utoipa 依赖。

use axum::Router;
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

#[derive(OpenApi)]
#[openapi(
    info(
        title = "EQYCC CUBA IMS API",
        version = "0.1.0",
        description = "手机屏幕组装厂 WMS 后端。\n\n\
            约定:\n\
            - 所有业务成功响应返回 `{code:0, message:\"ok\", data:...}`\n\
            - 业务失败走 HTTP 200 + code != 0\n\
            - 仅 401/403/500 使用对应 HTTP status\n\
            - JWT 走 `Authorization: Bearer <token>`",
        contact(name = "EQYCC")
    ),
    servers((url = "/", description = "当前环境")),
    paths(
        _doc_login,
        _doc_balance,
        _doc_inbound_create,
        _doc_inbound_submit,
    ),
    components(schemas(
        LoginBody, LoginData,
        BalanceRow,
        InboundCreateBody, InboundCreateLine, SubmitResult,
        ApiErrorEnvelope,
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
            SecurityScheme::Http(HttpBuilder::new().scheme(HttpAuthScheme::Bearer).bearer_format("JWT").build()),
        );
    }
}

/// 生成 swagger UI router(不持有应用 state)
///
/// 访问:`GET /docs`
/// OpenAPI JSON:`GET /api-docs/openapi.json`
#[must_use]
pub fn swagger_router() -> Router {
    SwaggerUi::new("/docs")
        .url("/api-docs/openapi.json", ApiDoc::openapi())
        .into()
}
