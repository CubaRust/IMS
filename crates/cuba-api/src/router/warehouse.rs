//! 仓库 + 仓位 HTTP 路由
//!
//! ```text
//! GET    /api/v1/warehouses              -- 列表
//! POST   /api/v1/warehouses              -- 新建
//! GET    /api/v1/warehouses/:id          -- 详情
//! PUT    /api/v1/warehouses/:id          -- 更新
//!
//! GET    /api/v1/locations               -- 列表
//! POST   /api/v1/locations               -- 新建
//! GET    /api/v1/locations/:id
//! PUT    /api/v1/locations/:id
//! ```

use axum::{
    extract::{Extension, Path, Query, State},
    routing::get,
    Json, Router,
};

use cuba_bootstrap::AppState;
use cuba_shared::{audit::AuditContext, error::AppError};
use cuba_warehouse::{
    CreateLocationCommand, CreateWarehouseCommand, LocationView, QueryLocations, QueryWarehouses,
    UpdateLocationCommand, UpdateWarehouseCommand, WarehouseService, WarehouseView,
};

use crate::response::AppJson;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/warehouses", get(list_warehouses).post(create_warehouse))
        .route("/warehouses/:id", get(get_warehouse).put(update_warehouse))
        .route("/locations", get(list_locations).post(create_location))
        .route("/locations/:id", get(get_location).put(update_location))
}

// -- warehouses --------------------------------------------------------------

async fn list_warehouses(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryWarehouses>,
) -> Result<AppJson<Vec<WarehouseView>>, AppError> {
    ctx.require_permission("mdm.warehouse.view")?;
    let svc = WarehouseService::new(state.db().clone());
    Ok(AppJson(svc.list_warehouses(&q).await?))
}

async fn get_warehouse(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<WarehouseView>, AppError> {
    ctx.require_permission("mdm.warehouse.view")?;
    let svc = WarehouseService::new(state.db().clone());
    Ok(AppJson(svc.get_warehouse(id).await?))
}

async fn create_warehouse(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Json(cmd): Json<CreateWarehouseCommand>,
) -> Result<AppJson<WarehouseView>, AppError> {
    ctx.require_permission("mdm.warehouse.edit")?;
    let svc = WarehouseService::new(state.db().clone());
    Ok(AppJson(svc.create_warehouse(&ctx, cmd).await?))
}

async fn update_warehouse(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
    Json(cmd): Json<UpdateWarehouseCommand>,
) -> Result<AppJson<WarehouseView>, AppError> {
    ctx.require_permission("mdm.warehouse.edit")?;
    let svc = WarehouseService::new(state.db().clone());
    Ok(AppJson(svc.update_warehouse(&ctx, id, cmd).await?))
}

// -- locations ---------------------------------------------------------------

async fn list_locations(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(q): Query<QueryLocations>,
) -> Result<AppJson<Vec<LocationView>>, AppError> {
    ctx.require_permission("mdm.warehouse.view")?;
    let svc = WarehouseService::new(state.db().clone());
    Ok(AppJson(svc.list_locations(&q).await?))
}

async fn get_location(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<LocationView>, AppError> {
    ctx.require_permission("mdm.warehouse.view")?;
    let svc = WarehouseService::new(state.db().clone());
    Ok(AppJson(svc.get_location(id).await?))
}

async fn create_location(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Json(cmd): Json<CreateLocationCommand>,
) -> Result<AppJson<LocationView>, AppError> {
    ctx.require_permission("mdm.warehouse.edit")?;
    let svc = WarehouseService::new(state.db().clone());
    Ok(AppJson(svc.create_location(&ctx, cmd).await?))
}

async fn update_location(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
    Json(cmd): Json<UpdateLocationCommand>,
) -> Result<AppJson<LocationView>, AppError> {
    ctx.require_permission("mdm.warehouse.edit")?;
    let svc = WarehouseService::new(state.db().clone());
    Ok(AppJson(svc.update_location(&ctx, id, cmd).await?))
}
