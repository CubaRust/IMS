//! reporting e2e:只要视图能 SELECT 不报错,各查询接口返回 Vec 即可通过。
//! 具体内容深度校验放业务用户接受测试。

use cuba_reporting::{
    QueryAging, QueryDormant, QueryExceptionSummary, QueryTxnFlow, ReportingService,
};
use cuba_testkit::TestDb;

#[tokio::test]
async fn reporting_queries_dont_panic() {
    let db = TestDb::new().await;
    let svc = ReportingService::new(db.pool_owned());

    let _a = svc
        .aging(&QueryAging::default())
        .await
        .expect("aging 查询不应失败");
    let _d = svc
        .dormant(&QueryDormant::default())
        .await
        .expect("dormant 查询不应失败");
    let _e = svc
        .exception_summary(&QueryExceptionSummary::default())
        .await
        .expect("exception_summary 查询不应失败");
    let _t = svc
        .txn_flow(&QueryTxnFlow::default())
        .await
        .expect("txn_flow 查询不应失败");
}

#[tokio::test]
async fn reporting_txn_flow_sees_inbound_submit() {
    // 先造一笔入库 → 再查 txn_flow 看得到
    use cuba_inbound::{CreateInboundCommand, CreateInboundLine, InboundService};
    use cuba_testkit::fixtures::{admin_ctx, dec, default_raw_wh_loc, seed_material, today};

    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    let mat = seed_material(&pool, "RPT-MAT-001", "RAW", false).await;
    let (wh, loc) = default_raw_wh_loc(&pool).await;

    let inb = InboundService::new(pool.clone());
    let ih = inb
        .create(
            &ctx,
            CreateInboundCommand {
                inbound_type: "PROD".into(),
                supplier_id: None,
                source_object_type: None,
                source_object_id: None,
                wh_id: wh,
                loc_id: Some(loc),
                inbound_date: today(),
                remark: None,
                lines: vec![CreateInboundLine {
                    line_no: 1,
                    material_id: mat,
                    batch_no: "RPT-B1".into(),
                    qty: dec("8"),
                    unit: "PCS".into(),
                    stock_status: None,
                    work_order_no: Some("WO-RPT".into()),
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

    let svc = ReportingService::new(pool.clone());
    let rows = svc
        .txn_flow(&QueryTxnFlow {
            material_id: Some(mat),
            wh_id: Some(wh),
            doc_type: Some("INBOUND".into()),
            scene_code: None,
            date_from: None,
            date_to: None,
            limit: Some(100),
        })
        .await
        .expect("txn_flow");
    assert!(!rows.is_empty(), "应该至少有一条 INBOUND 流水");
    assert!(rows.iter().any(|r| r.material_code == "RPT-MAT-001"));
}
