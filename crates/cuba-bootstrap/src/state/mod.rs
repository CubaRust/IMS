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
    pub config: AppConfig,
}

impl AppState {
    #[must_use]
    pub fn new(db: Db, config: AppConfig) -> Self {
        Self {
            inner: Arc::new(Inner { db, config }),
        }
    }

    #[must_use]
    pub fn db(&self) -> &Db {
        &self.inner.db
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
