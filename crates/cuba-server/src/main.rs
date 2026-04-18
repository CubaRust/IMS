//! cuba-server - EQYCC_CUBA_IMS HTTP 服务二进制
//!
//! 启动顺序:
//! 1. 加载 `.env` / 环境变量 → `AppConfig`
//! 2. 初始化 tracing
//! 3. 连库 + 按 `MIGRATION_MODE` 跑迁移
//! 4. 构建 `AppState`
//! 5. `cuba-api::build_router(state)` 拿到 `Router`
//! 6. `axum::serve` 启动
//!
//! 优雅关闭:收到 Ctrl-C 后等请求跑完再退出。

use std::net::SocketAddr;

use anyhow::Context;
use tokio::net::TcpListener;
use tracing::info;

use cuba_api::build_router;
use cuba_bootstrap::{config::AppConfig, database, state::AppState, telemetry};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 配置
    let cfg = AppConfig::from_env().context("加载配置失败")?;

    // 2. 日志
    telemetry::init(cfg.app_env);
    info!(
        app = %cfg.app_name,
        env = ?cfg.app_env,
        bind = %cfg.bind_addr(),
        "starting cuba-server"
    );

    // 3. DB + migration
    let db = database::connect(&cfg).await.context("连接数据库失败")?;
    database::run_migrations(&db, &cfg).await.context("迁移失败")?;

    // 4. State
    let state = AppState::new(db.clone(), cfg.clone());

    // 4.5 Scheduler(可由 env SCHEDULER_ENABLED=false 关闭)
    let scheduler_enabled = std::env::var("SCHEDULER_ENABLED")
        .map(|v| v != "false" && v != "0")
        .unwrap_or(true);
    let scheduler_handle = if scheduler_enabled {
        let sc_cfg = cuba_scheduler::SchedulerConfig {
            enabled: true,
            ..Default::default()
        };
        match cuba_scheduler::start(db.clone(), sc_cfg).await {
            Ok(h) => h,
            Err(e) => {
                tracing::warn!(error=%e, "scheduler 启动失败,继续无 scheduler 运行");
                None
            }
        }
    } else {
        info!("SCHEDULER_ENABLED=false, 跳过调度器");
        None
    };

    // 5. Router
    let router = build_router(state);

    // 6. Serve
    let addr: SocketAddr = cfg
        .bind_addr()
        .parse()
        .with_context(|| format!("非法的 bind 地址: {}", cfg.bind_addr()))?;
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("端口 {addr} 占用"))?;
    info!(%addr, "listening");

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("axum serve 异常退出")?;

    // 停调度器(若有)
    if let Some(h) = scheduler_handle {
        h.shutdown().await;
    }

    info!("bye");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("无法安装 Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        signal(SignalKind::terminate())
            .expect("无法安装 SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { tracing::info!("received ctrl-c, shutting down"); }
        _ = terminate => { tracing::info!("received SIGTERM, shutting down"); }
    }
}
