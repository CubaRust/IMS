//! 路由聚合入口
//!
//! 结构:
//! ```text
//! /health                       -- liveness
//! /api/v1/
//!   auth/login                  -- 登录(无守卫)
//!   auth/me                     -- 当前用户(auth_guard)
//!   auth/password               -- 改密(auth_guard)
//!   users / roles / permissions -- 基础管理(auth_guard + 权限点)
//!   inventory/...               -- 库存(auth_guard)
//!   inbound/...                 -- 入库(后续)
//! ```

use axum::{middleware as axum_mw, routing::get, Router};
use tower_http::{
    compression::CompressionLayer, cors::CorsLayer, timeout::TimeoutLayer, trace::TraceLayer,
};

use cuba_bootstrap::AppState;

use crate::{
    middleware::{auth_guard, health, trace_id},
    response::not_found_fallback,
};

pub mod catalog;
pub mod identity;
pub mod inventory;
pub mod warehouse;

/// 构建根 Router
pub fn build_router(state: AppState) -> Router {
    // 公开路由(不走 auth_guard)
    let public: Router<AppState> = Router::new()
        .route("/health", get(health))
        .merge(identity::public_routes());

    // 带鉴权路由
    let protected: Router<AppState> = Router::new()
        .merge(identity::protected_routes())
        .merge(warehouse::routes())
        .merge(catalog::routes())
        .nest("/inventory", inventory::routes())
        .route_layer(axum_mw::from_fn_with_state(state.clone(), auth_guard));

    let api_v1 = Router::new().merge(public).merge(protected);

    Router::new()
        .nest("/api/v1", api_v1)
        .route("/health", get(health))
        .fallback(not_found_fallback)
        .layer(axum_mw::from_fn(trace_id))
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::new(std::time::Duration::from_secs(30)))
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive()) // TODO: prod 改白名单
        .with_state(state)
}
