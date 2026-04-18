//! `AppState` — 被整个 axum 应用共享的只读状态
//!
//! 仅包含启动时初始化、运行期不变更的内容。业务仓储的具体实例由 `cuba-api`
//! 在路由层按需构造(从 `state.db.clone()` 构造 repo)。

use std::sync::Arc;

use crate::config::AppConfig;
use crate::database::Db;

/// 应用状态
///
/// 按 axum 0.7 惯例,用 `Arc<Inner>` 包装,`AppState` 自身是 Clone 开销极小的 handle。
#[derive(Clone)]
pub struct AppState {
    inner: Arc<Inner>,
}

struct Inner {
    pub db: Db,
    /// 读副本池,None 时 db_read() 回落到主库
    pub db_read: Option<Db>,
    pub config: AppConfig,
}

impl AppState {
    #[must_use]
    pub fn new(db: Db, config: AppConfig) -> Self {
        Self {
            inner: Arc::new(Inner { db, db_read: None, config }),
        }
    }

    /// 带读副本池的构造(若读副本不可用传 None,行为与 new 相同)
    #[must_use]
    pub fn new_with_read(db: Db, db_read: Option<Db>, config: AppConfig) -> Self {
        Self {
            inner: Arc::new(Inner { db, db_read, config }),
        }
    }

    #[must_use]
    pub fn db(&self) -> &Db {
        &self.inner.db
    }

    /// 只读查询专用池:有读副本走副本,否则走主库
    #[must_use]
    pub fn db_read(&self) -> &Db {
        self.inner.db_read.as_ref().unwrap_or(&self.inner.db)
    }

    #[must_use]
    pub fn config(&self) -> &AppConfig {
        &self.inner.config
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("app_name", &self.inner.config.app_name)
            .field("app_env", &self.inner.config.app_env)
            .finish()
    }
}
