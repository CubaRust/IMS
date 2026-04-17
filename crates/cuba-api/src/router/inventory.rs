//! 库存模块 HTTP 路由
//!
//! 路径:
//! ```text
//! POST /api/v1/inventory/txn        -- 提交库存事务
//! GET  /api/v1/inventory/balance    -- 查余额(分页)
//! GET  /api/v1/inventory/txn        -- 查流水(分页)
//! GET  /api/v1/inventory/txn/{id}   -- 查流水行
//! ```
//!
//! 架构约束:handler 只做 "HTTP ↔ application 层 DTO" 的转换,
//! 业务逻辑一律调 `cuba_inventory::InventoryService`。

use axum::{
    extract::{Extension, Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

use cuba_bootstrap::AppState;
use cuba_inventory::{
    CommitTxnCommand, CommitTxnResult, InventoryService, QueryBalance, QueryTxns,
    BalanceView, TxnHeadView, TxnLineView,
};
use cuba_shared::{
    audit::AuditContext,
    error::AppError,
    pagination::{PageQuery, PageResponse},
};

use crate::response::AppJson;

/// 挂到 `/api/v1/inventory`
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/txn", post(commit_txn).get(list_txns))
        .route("/txn/:id", get(list_txn_lines))
        .route("/balance", get(list_balance))
}

// ---------------------------------------------------------------------------
// handlers
// ---------------------------------------------------------------------------

async fn commit_txn(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Json(cmd): Json<CommitTxnCommand>,
) -> Result<AppJson<CommitTxnResult>, AppError> {
    ctx.require_permission("inv.txn.commit")?;
    let svc = InventoryService::new(state.db().clone());
    let result = svc.commit(&ctx, cmd).await?;
    Ok(AppJson(result))
}

/// 余额查询的 HTTP 入参:把过滤参数和分页扁平到同一组 query string
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct BalanceQueryIn {
    #[serde(flatten)]
    filter: QueryBalance,
    #[serde(flatten)]
    page: PageQuery,
}

async fn list_balance(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(input): Query<BalanceQueryIn>,
) -> Result<AppJson<PageResponse<BalanceView>>, AppError> {
    ctx.require_permission("inv.balance.view")?;
    let svc = InventoryService::new(state.db().clone());
    let data = svc.query_balance(&input.filter, input.page).await?;
    Ok(AppJson(data))
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct TxnQueryIn {
    #[serde(flatten)]
    filter: QueryTxns,
    #[serde(flatten)]
    page: PageQuery,
}

async fn list_txns(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Query(input): Query<TxnQueryIn>,
) -> Result<AppJson<PageResponse<TxnHeadView>>, AppError> {
    ctx.require_permission("inv.txn.view")?;
    let svc = InventoryService::new(state.db().clone());
    let data = svc.query_txns(&input.filter, input.page).await?;
    Ok(AppJson(data))
}

async fn list_txn_lines(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuditContext>,
    Path(id): Path<i64>,
) -> Result<AppJson<Vec<TxnLineView>>, AppError> {
    ctx.require_permission("inv.txn.view")?;
    let svc = InventoryService::new(state.db().clone());
    let data = svc.query_txn_lines(id).await?;
    Ok(AppJson(data))
}
