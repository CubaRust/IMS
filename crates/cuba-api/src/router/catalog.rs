//! Catalog HTTP 路由
//!
//! ```text
//! GET/POST/PUT /api/v1/materials[/:id]
//! GET/POST/PUT /api/v1/suppliers[/:id]
//! GET/POST/PUT /api/v1/customers[/:id]
//! GET/POST     /api/v1/boms[/:id]
//! GET/POST     /api/v1/routes[/:id]
//! GET          /api/v1/status-flows
//! ```

use axum::{
    extract::{Extension, Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};

use cuba_bootstrap::AppState;
use cuba_catalog::{
    BomHeadView, BomService, CreateBomCommand, CreateCustomerCommand, CreateMaterialCommand,
    CreateRouteCommand, CreateSupplierCommand, CustomerView, MaterialService, MaterialView,
    PartyService, QueryBoms, QueryCustomers, QueryMaterials, QueryRoutes, QueryStatusFlow,
    QuerySuppliers, RouteHeadView, RouteService, StatusFlowService, StatusFlowView, SupplierView,
    UpdateCustomerCommand, UpdateMaterialCommand, UpdateSupplierCommand,
};
use cuba_shared::{
    audit::AuditContext, error::AppError, pagination::PageResponse,
};

use crate::response::AppJson;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/materials", get(list_materials).post(create_material))
        .route("/materials/:id", get(get_material).put(update_material))
        .route("/suppliers", get(list_suppliers).post(create_supplier))
        .route("/suppliers/:id", get(get_supplier).put(update_supplier))
        .route("/customers", get(list_customers).post(create_customer))
        .route("/customers/:id", get(get_customer).put(update_customer))
        .route("/boms", get(list_boms).post(create_bom))
        .route("/boms/:id", get(get_bom))
        .route("/routes", get(list_routes).post(create_route))
        .route("/routes/:id", get(get_route))
        .route("/status-flows", get(list_status_flows))
}

// -- material ----------------------------------------------------------------

async fn list_materials(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryMaterials>,
) -> Result<AppJson<PageResponse<MaterialView>>, AppError> {
    ctx.require_permission("mdm.material.view")?;
    Ok(AppJson(MaterialService::new(state.db_read().clone()).list(&q).await?))
}

async fn get_material(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<MaterialView>, AppError> {
    ctx.require_permission("mdm.material.view")?;
    Ok(AppJson(MaterialService::new(state.db_read().clone()).get(id).await?))
}

async fn create_material(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Json(cmd): Json<CreateMaterialCommand>,
) -> Result<AppJson<MaterialView>, AppError> {
    ctx.require_permission("mdm.material.edit")?;
    Ok(AppJson(MaterialService::new(state.db().clone()).create(&ctx, cmd).await?))
}

async fn update_material(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
    Json(cmd): Json<UpdateMaterialCommand>,
) -> Result<AppJson<MaterialView>, AppError> {
    ctx.require_permission("mdm.material.edit")?;
    Ok(AppJson(
        MaterialService::new(state.db().clone()).update(&ctx, id, cmd).await?,
    ))
}

// -- supplier ----------------------------------------------------------------

async fn list_suppliers(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QuerySuppliers>,
) -> Result<AppJson<Vec<SupplierView>>, AppError> {
    ctx.require_permission("mdm.material.view")?; // 复用 mdm.view 权限
    Ok(AppJson(PartyService::new(state.db_read().clone()).list_suppliers(&q).await?))
}

async fn get_supplier(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<SupplierView>, AppError> {
    ctx.require_permission("mdm.material.view")?;
    Ok(AppJson(PartyService::new(state.db_read().clone()).get_supplier(id).await?))
}

async fn create_supplier(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Json(cmd): Json<CreateSupplierCommand>,
) -> Result<AppJson<SupplierView>, AppError> {
    ctx.require_permission("mdm.material.edit")?;
    Ok(AppJson(PartyService::new(state.db().clone()).create_supplier(&ctx, cmd).await?))
}

async fn update_supplier(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
    Json(cmd): Json<UpdateSupplierCommand>,
) -> Result<AppJson<SupplierView>, AppError> {
    ctx.require_permission("mdm.material.edit")?;
    Ok(AppJson(
        PartyService::new(state.db().clone()).update_supplier(&ctx, id, cmd).await?,
    ))
}

// -- customer ----------------------------------------------------------------

async fn list_customers(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryCustomers>,
) -> Result<AppJson<Vec<CustomerView>>, AppError> {
    ctx.require_permission("mdm.material.view")?;
    Ok(AppJson(PartyService::new(state.db_read().clone()).list_customers(&q).await?))
}

async fn get_customer(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<CustomerView>, AppError> {
    ctx.require_permission("mdm.material.view")?;
    Ok(AppJson(PartyService::new(state.db_read().clone()).get_customer(id).await?))
}

async fn create_customer(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Json(cmd): Json<CreateCustomerCommand>,
) -> Result<AppJson<CustomerView>, AppError> {
    ctx.require_permission("mdm.material.edit")?;
    Ok(AppJson(PartyService::new(state.db().clone()).create_customer(&ctx, cmd).await?))
}

async fn update_customer(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
    Json(cmd): Json<UpdateCustomerCommand>,
) -> Result<AppJson<CustomerView>, AppError> {
    ctx.require_permission("mdm.material.edit")?;
    Ok(AppJson(
        PartyService::new(state.db().clone()).update_customer(&ctx, id, cmd).await?,
    ))
}

// -- bom ---------------------------------------------------------------------

async fn list_boms(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryBoms>,
) -> Result<AppJson<Vec<BomHeadView>>, AppError> {
    ctx.require_permission("mdm.bom.view")?;
    Ok(AppJson(BomService::new(state.db_read().clone()).list(&q).await?))
}

async fn get_bom(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<BomHeadView>, AppError> {
    ctx.require_permission("mdm.bom.view")?;
    Ok(AppJson(BomService::new(state.db_read().clone()).get(id).await?))
}

async fn create_bom(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Json(cmd): Json<CreateBomCommand>,
) -> Result<AppJson<BomHeadView>, AppError> {
    ctx.require_permission("mdm.bom.edit")?;
    Ok(AppJson(BomService::new(state.db().clone()).create(&ctx, cmd).await?))
}

// -- route -------------------------------------------------------------------

async fn list_routes(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryRoutes>,
) -> Result<AppJson<Vec<RouteHeadView>>, AppError> {
    ctx.require_permission("mdm.route.view")?;
    Ok(AppJson(RouteService::new(state.db_read().clone()).list(&q).await?))
}

async fn get_route(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<RouteHeadView>, AppError> {
    ctx.require_permission("mdm.route.view")?;
    Ok(AppJson(RouteService::new(state.db_read().clone()).get(id).await?))
}

async fn create_route(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Json(cmd): Json<CreateRouteCommand>,
) -> Result<AppJson<RouteHeadView>, AppError> {
    ctx.require_permission("mdm.route.edit")?;
    Ok(AppJson(RouteService::new(state.db().clone()).create(&ctx, cmd).await?))
}

// -- status flow (只读) -------------------------------------------------------

async fn list_status_flows(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryStatusFlow>,
) -> Result<AppJson<Vec<StatusFlowView>>, AppError> {
    ctx.require_permission("mdm.material.view")?;
    Ok(AppJson(StatusFlowService::new(state.db().clone()).list(&q).await?))
}
