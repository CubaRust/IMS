//! 数据库连接池 + migration 管理

use std::time::Duration;

use sqlx::postgres::{PgPool, PgPoolOptions};
use tracing::{info, warn};

use cuba_shared::error::AppError;

use crate::config::{AppConfig, MigrationMode};

/// 工作区统一的数据库连接池类型别名
pub type Db = PgPool;

/// 建立连接池
pub async fn connect(cfg: &AppConfig) -> Result<Db, AppError> {
    info!(
        max_connections = cfg.database_max_connections,
        "connecting to postgres"
    );
    let pool = PgPoolOptions::new()
        .max_connections(cfg.database_max_connections)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&cfg.database_url)
        .await
        .map_err(|e| {
            AppError::Internal(anyhow::anyhow!("数据库连接失败: {e}"))
        })?;

    // 轻量 ping 确认
    sqlx::query("select 1").execute(&pool).await?;
    info!("postgres connected");
    Ok(pool)
}

/// 建立读副本连接池(若配置了);否则返回 None
pub async fn connect_read_pool(cfg: &AppConfig) -> Result<Option<Db>, AppError> {
    let Some(url) = cfg.database_read_url.as_ref() else {
        return Ok(None);
    };
    info!(
        max_connections = cfg.database_max_connections,
        "connecting to postgres read replica"
    );
    let pool = PgPoolOptions::new()
        .max_connections(cfg.database_max_connections)
        .acquire_timeout(Duration::from_secs(10))
        .connect(url)
        .await
        .map_err(|e| {
            warn!(error=%e, "读副本连接失败,fallback 到主库");
            AppError::Internal(anyhow::anyhow!("read replica 连接失败: {e}"))
        })?;

    if sqlx::query("select 1").execute(&pool).await.is_err() {
        warn!("read replica ping failed, 将 fallback 到主库");
        return Ok(None);
    }
    info!("postgres read replica connected");
    Ok(Some(pool))
}

/// 按配置运行 migration
///
/// - `Auto`   : `sqlx::migrate!` 自动跑(dev/test)
/// - `Manual` : 只校验,不执行(prod)
pub async fn run_migrations(pool: &Db, cfg: &AppConfig) -> Result<(), AppError> {
    match cfg.migration_mode {
        MigrationMode::Auto => {
            info!("MIGRATION_MODE=auto, running migrations...");
            sqlx::migrate!("../../migrations")
                .run(pool)
                .await
                .map_err(|e| AppError::Internal(anyhow::anyhow!("migration 执行失败: {e}")))?;
            info!("migrations applied");
        }
        MigrationMode::Manual => {
            warn!("MIGRATION_MODE=manual, skipping migrate (ensure you ran `sqlx migrate run` before deploy)");
            // TODO: 后续可以加"查询 _sqlx_migrations 表,对比本地 migrations 目录,
            // 若有未应用项直接 panic 拒启动",目前先放过
        }
    }
    Ok(())
}
