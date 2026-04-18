//! HTTP 层测试工具:起一个 axum 子进程监听 127.0.0.1:<ephemeral>,返回 base url。
//! 配套 reqwest 客户端做真实 HTTP 请求。

use std::net::SocketAddr;

use cuba_bootstrap::{
    config::{AppEnv, MigrationMode},
    AppConfig, AppState,
};
use sqlx::PgPool;
use tokio::net::TcpListener;

/// 启动一个测试 HTTP server,返回 (base_url, join_handle)。
/// 测试结束时 join_handle drop 会停掉任务。
pub async fn spawn_server(pool: PgPool) -> (String, tokio::task::JoinHandle<()>) {
    let cfg = AppConfig {
        app_name: "cuba-test".into(),
        app_env: AppEnv::Dev,
        http_host: "127.0.0.1".into(),
        http_port: 0,
        database_url: "postgres://test".into(),
        database_read_url: None,
        database_max_connections: 5,
        jwt_secret: "test-secret-min-32-chars-XXXXXXXXXX".into(),
        jwt_ttl_seconds: 3600,
        migration_mode: MigrationMode::Manual,
    };
    let state = AppState::new(pool, cfg);
    let app = cuba_api::build_router(state);

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral");
    let addr: SocketAddr = listener.local_addr().expect("local addr");
    let base = format!("http://{addr}");

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("axum serve");
    });

    // 给服务留 50ms 起来
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    (base, handle)
}
