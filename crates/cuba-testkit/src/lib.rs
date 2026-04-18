//! cuba-testkit
//!
//! 测试公共设施:
//! - `TestDb`:起一次性 postgres container,跑全部 migrations + seed
//! - `install_tracing()`:一次性安装 tracing(测试共享)
//! - `fixtures::*`:常用 seed 数据(demo 仓位/物料)辅助构造

#![deny(unsafe_code)]

pub mod fixtures;
pub mod http;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use once_cell::sync::OnceCell;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use testcontainers::{runners::AsyncRunner, ContainerAsync};
use testcontainers_modules::postgres::Postgres as PgImage;
use tracing_subscriber::EnvFilter;

static TRACING_INIT: OnceCell<()> = OnceCell::new();

/// 一次性安装 tracing(测试进程共享)
pub fn install_tracing() {
    TRACING_INIT.get_or_init(|| {
        let filter = EnvFilter::try_from_env("CUBA_TEST_LOG")
            .unwrap_or_else(|_| EnvFilter::new("warn,cuba_=debug"));
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_test_writer()
            .try_init();
    });
}

/// 一次性 postgres 容器 + pool + migrations + seed
pub struct TestDb {
    #[allow(dead_code)]
    container: Arc<ContainerAsync<PgImage>>,
    pool: PgPool,
}

impl TestDb {
    /// 起 container、建 pool、跑 migrations
    pub async fn new() -> Self {
        install_tracing();
        let container = PgImage::default()
            .with_db_name("cuba_ims_test")
            .with_user("cuba")
            .with_password("cuba")
            .start()
            .await
            .expect("failed to start postgres container");

        let port = container.get_host_port_ipv4(5432).await.expect("no port");
        let url = format!("postgres://cuba:cuba@127.0.0.1:{port}/cuba_ims_test");

        let pool = PgPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(30))
            .connect(&url)
            .await
            .expect("connect to test pg");

        run_migrations(&pool).await.expect("run migrations");

        Self {
            container: Arc::new(container),
            pool,
        }
    }

    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    #[must_use]
    pub fn pool_owned(&self) -> PgPool {
        self.pool.clone()
    }
}

async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    // 从 CARGO_MANIFEST_DIR 往上两级找 migrations
    let mig = find_migrations_dir().context("locate migrations dir")?;
    sqlx::migrate::Migrator::new(mig)
        .await
        .context("load migrations")?
        .run(pool)
        .await
        .context("apply migrations")?;
    Ok(())
}

fn find_migrations_dir() -> anyhow::Result<std::path::PathBuf> {
    let cur = std::env::var("CARGO_MANIFEST_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_default();
    let mut candidates = vec![
        cur.join("../../migrations"),
        cur.join("../migrations"),
        std::path::PathBuf::from("./migrations"),
        std::path::PathBuf::from("../migrations"),
        std::path::PathBuf::from("../../migrations"),
    ];
    if let Ok(env) = std::env::var("CUBA_MIGRATIONS_DIR") {
        candidates.insert(0, std::path::PathBuf::from(env));
    }
    for c in candidates {
        if c.join("0001_init.sql").exists() {
            return Ok(c);
        }
    }
    Err(anyhow::anyhow!("migrations directory not found"))
}
