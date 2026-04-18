//! JWT 吊销闭环 e2e:
//! - 登录拿 token
//! - 调 /auth/me 成功
//! - 调 /auth/logout
//! - 再用同一 token 调 /auth/me → 401
//! - refresh:拿到新 token,旧 token 无效

use cuba_testkit::{http::spawn_server, TestDb};
use serde_json::{json, Value};

#[tokio::test]
async fn logout_invalidates_token() {
    let db = TestDb::new().await;
    let (base, _h) = spawn_server(db.pool_owned()).await;
    let client = reqwest::Client::new();

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

    // /me 成功
    let r = client
        .get(format!("{base}/api/v1/auth/me"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status().as_u16(), 200);

    // logout
    let r = client
        .post(format!("{base}/api/v1/auth/logout"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status().as_u16(), 200);

    // 再 /me → 401
    let r = client
        .get(format!("{base}/api/v1/auth/me"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status().as_u16(), 401, "吊销后旧 token 应返回 401");
}

#[tokio::test]
async fn refresh_gives_new_token_and_old_invalid() {
    let db = TestDb::new().await;
    let (base, _h) = spawn_server(db.pool_owned()).await;
    let client = reqwest::Client::new();

    let old: Value = client
        .post(format!("{base}/api/v1/auth/login"))
        .json(&json!({ "login_name": "admin", "password": "Admin@123" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let old_token = old["data"]["token"].as_str().unwrap().to_string();

    // refresh
    let refreshed: Value = client
        .post(format!("{base}/api/v1/auth/refresh"))
        .bearer_auth(&old_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        refreshed["code"].as_i64().unwrap_or(-1),
        0,
        "refresh: {refreshed}"
    );
    let new_token = refreshed["data"]["token"].as_str().unwrap().to_string();
    assert_ne!(old_token, new_token, "新旧 token 应不同");

    // 新 token 能用
    let r = client
        .get(format!("{base}/api/v1/auth/me"))
        .bearer_auth(&new_token)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status().as_u16(), 200);

    // 旧 token 失效
    let r = client
        .get(format!("{base}/api/v1/auth/me"))
        .bearer_auth(&old_token)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status().as_u16(), 401, "旧 token 应吊销");
}

#[tokio::test]
async fn live_probe_always_ok() {
    let db = TestDb::new().await;
    let (base, _h) = spawn_server(db.pool_owned()).await;
    let client = reqwest::Client::new();

    let r = client.get(format!("{base}/live")).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
    assert_eq!(r.text().await.unwrap(), "live");
}

#[tokio::test]
async fn ready_probe_ok_when_db_up() {
    let db = TestDb::new().await;
    let (base, _h) = spawn_server(db.pool_owned()).await;
    let client = reqwest::Client::new();

    let r = client.get(format!("{base}/ready")).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
}
