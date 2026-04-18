//! scheduler jobs e2e — 直接调 job 函数,不起完整调度器

use cuba_scheduler::jobs::{
    audit_log_archive, dormant_refresh, jwt_revocation_cleanup, preissue_timeout_scan,
};
use cuba_testkit::TestDb;

#[tokio::test]
async fn preissue_timeout_scan_idempotent() {
    let db = TestDb::new().await;
    // 无过期数据时,返回 flagged=0
    let r = preissue_timeout_scan(db.pool_owned()).await.unwrap();
    assert_eq!(r["flagged"].as_i64().unwrap(), 0);
}

#[tokio::test]
async fn dormant_refresh_handles_non_matview() {
    let db = TestDb::new().await;
    let r = dormant_refresh(db.pool_owned()).await.unwrap();
    // 目前是普通视图,返回 refreshed=false
    assert!(!r["refreshed"].as_bool().unwrap());
}

#[tokio::test]
async fn audit_archive_with_no_data() {
    let db = TestDb::new().await;
    let r = audit_log_archive(db.pool_owned(), 90).await.unwrap();
    assert_eq!(r["archived"].as_i64().unwrap(), 0);
    assert_eq!(r["cutoff_days"].as_i64().unwrap(), 90);
}

#[tokio::test]
async fn jwt_revocation_cleanup_with_no_data() {
    let db = TestDb::new().await;
    let r = jwt_revocation_cleanup(db.pool_owned()).await.unwrap();
    assert_eq!(r["deleted"].as_i64().unwrap(), 0);
}

#[tokio::test]
async fn preissue_timeout_flags_overdue() {
    use cuba_inbound::InboundService;
    use cuba_preissue::{CreatePreissueCommand, CreatePreissueLine, PreissueService};
    use cuba_testkit::fixtures::{admin_ctx, dec, default_raw_wh_loc, seed_material};

    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    // 物料 + 仓
    let mat_id = seed_material(&pool, "TIMEOUT-MAT", "RAW", true).await;
    let (wh_id, loc_id) = default_raw_wh_loc(&pool).await;

    // 建 preissue,expected_close_date 设成昨天
    let yesterday = time::OffsetDateTime::now_utc().date() - time::Duration::days(1);
    let pre = PreissueService::new(pool.clone());
    let pres = pre
        .create_and_issue(
            &ctx,
            CreatePreissueCommand {
                exception_type: Some("PREISSUE".into()),
                supplier_id: None,
                work_order_no: Some("WO-TO".into()),
                process_name: None,
                workshop_name: None,
                issue_date: yesterday,
                reason: "超期测试".into(),
                expected_close_date: Some(yesterday),
                remark: None,
                wh_id,
                loc_id,
                lines: vec![CreatePreissueLine {
                    line_no: 1,
                    material_id: mat_id,
                    qty: dec("5"),
                    unit: "PCS".into(),
                    expected_batch_no: Some("TO-B1".into()),
                    target_desc: None,
                    note: None,
                }],
            },
        )
        .await
        .unwrap();
    let _ = pres; // id 不强制使用

    // 跑扫描
    let r = preissue_timeout_scan(pool.clone()).await.unwrap();
    assert!(
        r["flagged"].as_i64().unwrap() >= 1,
        "至少标记一条超期,实际 {r}"
    );

    // 第二次跑,应该幂等(timeout_flag=true 已标过,不再动)
    let r2 = preissue_timeout_scan(pool.clone()).await.unwrap();
    assert_eq!(r2["flagged"].as_i64().unwrap(), 0, "二次应 0");

    // 避免未使用 warning
    let _ = InboundService::new(pool);
}
