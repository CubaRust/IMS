//! HTTP 层 e2e:起真 axum server,用 reqwest 打请求

use cuba_testkit::{fixtures::seed_material, http::spawn_server, TestDb};
use serde_json::{json, Value};

#[tokio::test]
async fn http_login_then_list_inbounds() {
    let db = TestDb::new().await;
    let (base, _h) = spawn_server(db.pool_owned()).await;

    let client = reqwest::Client::new();

    // 1. login
    let r: Value = client
        .post(format!("{base}/api/v1/auth/login"))
        .json(&json!({ "login_name": "admin", "password": "Admin@123" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(r["code"].as_i64().unwrap_or(-1), 0, "login envelope: {r}");
    let token = r["data"]["token"].as_str().expect("token").to_string();

    // 2. me
    let me: Value = client
        .get(format!("{base}/api/v1/auth/me"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(me["data"]["login_name"], "admin");

    // 3. list inbounds
    let list: Value = client
        .get(format!("{base}/api/v1/inbounds"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(list["code"].as_i64().unwrap_or(-1), 0);
    assert!(list["data"].is_array());
}

#[tokio::test]
async fn http_unauthenticated_returns_401() {
    let db = TestDb::new().await;
    let (base, _h) = spawn_server(db.pool_owned()).await;

    let resp = reqwest::Client::new()
        .get(format!("{base}/api/v1/inbounds"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 401);
}

#[tokio::test]
async fn http_create_and_submit_inbound_moves_balance() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let mat_id = seed_material(&pool, "HTTP-MAT-001", "RAW", false).await;

    let (base, _h) = spawn_server(pool.clone()).await;
    let client = reqwest::Client::new();

    // login
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

    // 取 RAW01 仓的 id
    let wh_id: i64 = sqlx::query_scalar("select id from mdm.mdm_warehouse where wh_code='RAW01'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let loc_id: i64 =
        sqlx::query_scalar("select id from mdm.mdm_location where wh_id = $1 order by id limit 1")
            .bind(wh_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    // 建入库单
    let create: Value = client
        .post(format!("{base}/api/v1/inbounds"))
        .bearer_auth(&token)
        .json(&json!({
            "inbound_type": "PROD",
            "wh_id": wh_id,
            "loc_id": loc_id,
            "inbound_date": chrono_today_str(),
            "lines": [
                {
                    "line_no": 1,
                    "material_id": mat_id,
                    "batch_no": "HTTP-B1",
                    "qty": "25",
                    "unit": "PCS",
                    "work_order_no": "WO-HTTP"
                }
            ]
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(create["code"].as_i64().unwrap_or(-1), 0, "create: {create}");
    let inbound_id = create["data"]["id"].as_i64().expect("id");

    // submit
    let submit: Value = client
        .post(format!("{base}/api/v1/inbounds/{inbound_id}/submit"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(submit["code"].as_i64().unwrap_or(-1), 0, "submit: {submit}");
    assert_eq!(submit["data"]["doc_status"], "COMPLETED");

    // 余额对账
    let qty: rust_decimal::Decimal = sqlx::query_scalar(
        r#"select coalesce(sum(book_qty), 0) from inv.balance
            where wh_id=$1 and loc_id=$2 and material_id=$3
              and batch_no='HTTP-B1' and stock_status='QUALIFIED'"#,
    )
    .bind(wh_id)
    .bind(loc_id)
    .bind(mat_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(qty.to_string(), "25");
}

fn chrono_today_str() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Iso8601::DATE)
        .unwrap()
}
