//! OpenAPI / Swagger UI 装载
//!
//! 本模块只提供**骨架**和若干关键端点的 schema(登录、入库、库存)。
//! 若要为所有端点出完整 schema,请给每个 handler 加 `#[utoipa::path(...)]`
//! 并在此 OpenApi derive 的 `paths` 里注册。

use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

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

/// 生成 swagger UI router,挂到 `/docs`
///
/// 访问:`http://<host>:<port>/docs`
/// JSON:`http://<host>:<port>/api-docs/openapi.json`
#[must_use]
pub fn swagger_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    SwaggerUi::new("/docs")
        .url("/api-docs/openapi.json", ApiDoc::openapi())
        .into()
}
