//! metrics e2e:打几个请求,然后查 /metrics,断言关键指标字段存在

use cuba_testkit::{http::spawn_server, TestDb};
use serde_json::{json, Value};

#[tokio::test]
async fn metrics_endpoint_exposes_counters() {
    let db = TestDb::new().await;
    let (base, _h) = spawn_server(db.pool_owned()).await;
    let client = reqwest::Client::new();

    // 健康请求,命中 http metrics
    let _ = client.get(format!("{base}/health")).send().await.unwrap();

    // 登录(成功) → 也会命中 HTTP 指标
    let login: Value = client
        .post(format!("{base}/api/v1/auth/login"))
        .json(&json!({ "login_name": "admin", "password": "Admin@123" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(login["code"], 0);

    // 登录失败 → 应上报 business_errors
    let _ = client
        .post(format!("{base}/api/v1/auth/login"))
        .json(&json!({ "login_name": "admin", "password": "WrongPass!!" }))
        .send()
        .await
        .unwrap();

    // 取 /metrics
    let body = client
        .get(format!("{base}/metrics"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    // 核心指标字段
    assert!(
        body.contains("cuba_http_requests_total"),
        "body should contain cuba_http_requests_total:\n{body}"
    );
    assert!(body.contains("cuba_http_request_duration_seconds"));
    assert!(
        body.contains("/health") || body.contains("/api/v1/auth/login"),
        "metrics 应该记了 path label"
    );
    // 登录失败应产生一条 business_errors
    assert!(
        body.contains("cuba_business_errors_total"),
        "business errors metric not present"
    );
}
