//! 服务门面
//!
//! 业务模块只需要依赖本类型:
//! ```ignore
//! let svc = InventoryService::new(pool);
//! svc.commit(&ctx, cmd).await?;
//! svc.query_balance(&q, page).await?;
//! ```
//!
//! service 内部顺序:
//! 1. `command.into_domain()` 得到 domain 对象
//! 2. `rules::validate_txn` 校验结构
//! 3. `rules::compute_deltas` 得到余额增量
//! 4. `repo.commit_txn` 一次 DB 事务写 head/lines/balance

use std::sync::Arc;

use sqlx::PgPool;

use cuba_shared::{
    audit::AuditContext,
    error::AppError,
    pagination::{PageQuery, PageResponse},
};

use super::{
    commands::{CommitTxnCommand, CommitTxnResult},
    dto::{BalanceView, TxnHeadView, TxnLineView},
    queries::{QueryBalance, QueryTxns},
};
use crate::domain::rules;
use crate::infrastructure::repository::{InventoryRepository, PgInventoryRepository};

/// 库存应用服务
///
/// `Arc<dyn InventoryRepository>` 让单元测试可以换假 repo
#[derive(Clone)]
pub struct InventoryService {
    repo: Arc<dyn InventoryRepository>,
}

impl InventoryService {
    /// 直接由连接池构造(生产默认路径)
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: Arc::new(PgInventoryRepository::new(pool)),
        }
    }

    /// 允许注入自定义 repo(测试用)
    #[must_use]
    pub fn with_repo(repo: Arc<dyn InventoryRepository>) -> Self {
        Self { repo }
    }

    /// 提交一笔库存事务
    pub async fn commit(
        &self,
        ctx: &AuditContext,
        cmd: CommitTxnCommand,
    ) -> Result<CommitTxnResult, AppError> {
        let txn_type = cmd.txn_type.as_str().to_string();
        let scene_code = cmd.scene_code.clone();
        let (head, lines) = cmd.into_domain(Some(ctx.user_id));
        let result = async {
            rules::validate_txn(&head, &lines)?;
            let deltas = rules::compute_deltas(&head, &lines)?;
            self.repo.commit_txn(ctx, head, lines, deltas).await
        }
        .await;
        cuba_metrics::record_txn(&txn_type, &scene_code, result.is_ok());
        result
    }

    pub async fn query_balance(
        &self,
        query: &QueryBalance,
        page: PageQuery,
    ) -> Result<PageResponse<BalanceView>, AppError> {
        self.repo.query_balance(query, page).await
    }

    pub async fn query_txns(
        &self,
        query: &QueryTxns,
        page: PageQuery,
    ) -> Result<PageResponse<TxnHeadView>, AppError> {
        self.repo.query_txns(query, page).await
    }

    pub async fn query_txn_lines(&self, txn_id: i64) -> Result<Vec<TxnLineView>, AppError> {
        self.repo.query_txn_lines(txn_id).await
    }
}
