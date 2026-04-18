//! events e2e:入库 submit 后事件表应有两条事件
//! - InventoryTxnCommitted(inventory crate 写)
//! - InboundSubmitted(inbound crate 写)

use cuba_inbound::{CreateInboundCommand, CreateInboundLine, InboundService};
use cuba_testkit::{
    fixtures::{admin_ctx, dec, default_raw_wh_loc, seed_material, today},
    TestDb,
};

#[tokio::test]
async fn inbound_submit_writes_both_events() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    let mat_id = seed_material(&pool, "EVT-MAT-001", "RAW", false).await;
    let (wh_id, loc_id) = default_raw_wh_loc(&pool).await;

    let inb = InboundService::new(pool.clone());
    let ih = inb
        .create(
            &ctx,
            CreateInboundCommand {
                inbound_type: "PROD".into(),
                supplier_id: None,
                source_object_type: None,
                source_object_id: None,
                wh_id,
                loc_id: Some(loc_id),
                inbound_date: today(),
                remark: None,
                lines: vec![CreateInboundLine {
                    line_no: 1,
                    material_id: mat_id,
                    batch_no: "EVT-B1".into(),
                    qty: dec("30"),
                    unit: "PCS".into(),
                    stock_status: None,
                    work_order_no: Some("WO-EVT".into()),
                    process_name: None,
                    outsource_no: None,
                    related_preissue_line_id: None,
                    note: None,
                }],
            },
        )
        .await
        .unwrap();
    inb.submit(&ctx, ih.id).await.unwrap();

    // 等待异步事件落(InboundSubmitted 是 write_event 非事务版,应已落)
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // 必有 InventoryTxnCommitted
    let inv_cnt: i64 = sqlx::query_scalar(
        "select count(*) from events.domain_event where event_type = 'InventoryTxnCommitted'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(inv_cnt >= 1, "expected InventoryTxnCommitted, got {inv_cnt}");

    // 必有 InboundSubmitted
    let in_cnt: i64 = sqlx::query_scalar(
        "select count(*) from events.domain_event where event_type = 'InboundSubmitted'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(in_cnt >= 1, "expected InboundSubmitted, got {in_cnt}");

    // payload 是合法 JSON
    let payload: serde_json::Value = sqlx::query_scalar(
        "select payload from events.domain_event
           where event_type='InboundSubmitted' order by id desc limit 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(payload["inbound_id"].as_i64().unwrap(), ih.id);
    assert!(payload["inbound_no"].as_str().unwrap().starts_with("IN"));
}

#[tokio::test]
async fn outbox_flag_is_unpublished_by_default() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    let mat_id = seed_material(&pool, "EVT-OUTBOX", "RAW", false).await;
    let (wh_id, loc_id) = default_raw_wh_loc(&pool).await;

    let inb = InboundService::new(pool.clone());
    let ih = inb
        .create(
            &ctx,
            CreateInboundCommand {
                inbound_type: "PROD".into(),
                supplier_id: None,
                source_object_type: None,
                source_object_id: None,
                wh_id,
                loc_id: Some(loc_id),
                inbound_date: today(),
                remark: None,
                lines: vec![CreateInboundLine {
                    line_no: 1,
                    material_id: mat_id,
                    batch_no: "OUTBOX-B1".into(),
                    qty: dec("5"),
                    unit: "PCS".into(),
                    stock_status: None,
                    work_order_no: None,
                    process_name: None,
                    outsource_no: None,
                    related_preissue_line_id: None,
                    note: None,
                }],
            },
        )
        .await
        .unwrap();
    inb.submit(&ctx, ih.id).await.unwrap();

    let unpub: i64 = sqlx::query_scalar(
        "select count(*) from events.domain_event where published = false",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(unpub >= 2, "outbox 应有至少 2 条未 publish,实际 {unpub}");
}
