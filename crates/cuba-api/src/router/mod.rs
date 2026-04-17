//! 路由聚合入口
//!
//! 结构:
//! ```text
//! /health                       -- liveness
//! /api/v1/
//!   auth/login                  -- 登录(无守卫)
//!   inventory/...               -- 库存(auth_guard)
//!   inbound/...                 -- 入库(auth_guard)
//!   ...
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

pub mod inventory;

/// 构建根 Router
pub fn build_router(state: AppState) -> Router {
    // 公开路由(无需 auth)
    let public: Router<AppState> = Router::new().route("/health", get(health));

    // 带鉴权的路由
    let protected: Router<AppState> = Router::new()
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
