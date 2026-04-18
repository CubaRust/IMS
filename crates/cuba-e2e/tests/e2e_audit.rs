//! 审计日志 e2e:打一个受保护请求,看 sys_audit_log 有没有写成

use cuba_testkit::{http::spawn_server, TestDb};
use serde_json::{json, Value};

#[tokio::test]
async fn audit_log_records_protected_request() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let (base, _h) = spawn_server(pool.clone()).await;
    let client = reqwest::Client::new();

    // 登录
    let token = client
        .post(format!("{base}/api/v1/auth/login"))
        .json(&json!({ "login_name": "admin", "password": "Admin@123" }))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap()["data"]["token"]
        .as_str()
        .unwrap()
        .to_string();

    // 调受保护接口
    let _ = client
        .get(format!("{base}/api/v1/auth/me"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    // audit 是 spawn 的,给 200ms 等它落库
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    let cnt: i64 = sqlx::query_scalar(
        "select count(*) from sys.sys_audit_log where http_path = '/api/v1/auth/me'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(cnt >= 1, "审计日志应至少记录一行 /auth/me 调用");
}
