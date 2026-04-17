//! stocktake e2e:
//! - snapshot 模式从 balance 建盘点行
//! - 录实盘数,submit 产生盈亏 CONVERT
//! - 对账:调整后 book 应 = 实盘

use cuba_inbound::{CreateInboundCommand, CreateInboundLine, InboundService};
use cuba_stocktake::{
    CreateStocktakeCommand, RecordCountCommand, RecordCountLine, StocktakeService,
};
use cuba_testkit::{
    fixtures::{admin_ctx, dec, default_raw_wh_loc, seed_material, today},
    TestDb,
};
use rust_decimal::Decimal;
use sqlx::PgPool;

#[tokio::test]
async fn stocktake_snapshot_and_reconcile() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    // 先入 50
    let mat_id = seed_material(&pool, "ST-MAT-001", "RAW", false).await;
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
                    batch_no: "ST-B1".into(),
                    qty: dec("50"),
                    unit: "PCS".into(),
                    stock_status: None,
                    work_order_no: Some("WO-ST".into()),
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

    // 建盘点单(snapshot 模式)
    let st = StocktakeService::new(pool.clone());
    let head = st
        .create(
            &ctx,
            CreateStocktakeCommand {
                wh_id,
                loc_id: Some(loc_id),
                stocktake_date: today(),
                remark: None,
                snapshot_from_balance: true,
                lines: vec![],
            },
        )
        .await
        .unwrap();

    // snapshot 后应至少一行,且 book_qty = 50
    let snap_line = head
        .lines
        .iter()
        .find(|l| l.material_id == mat_id)
        .expect("snapshot 行存在");
    assert_eq!(snap_line.book_qty, dec("50"));
    assert!(!snap_line.counted);

    // 录实盘 47(少 3)
    st.record_counts(
        &ctx,
        head.id,
        RecordCountCommand {
            lines: vec![RecordCountLine {
                line_id: snap_line.id,
                actual_qty: dec("47"),
                note: Some("盘亏 3 件".into()),
            }],
        },
    )
    .await
    .unwrap();

    // submit → 生成盈亏 CONVERT
    let res = st.submit(&ctx, head.id).await.unwrap();
    assert_eq!(res.doc_status, "COMPLETED");
    assert_eq!(res.gain_line_count, 0);
    assert_eq!(res.loss_line_count, 1);

    // 余额应 = 47
    let q = qty_of(&pool, wh_id, loc_id, mat_id, "ST-B1", "QUALIFIED").await;
    assert_eq!(q, dec("47"));
}

#[tokio::test]
async fn stocktake_submit_fails_if_not_all_counted() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    let mat_id = seed_material(&pool, "ST-MAT-002", "RAW", false).await;
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
                    batch_no: "ST-B2".into(),
                    qty: dec("10"),
                    unit: "PCS".into(),
                    stock_status: None,
                    work_order_no: Some("WO-ST2".into()),
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

    let st = StocktakeService::new(pool.clone());
    let head = st
        .create(
            &ctx,
            CreateStocktakeCommand {
                wh_id,
                loc_id: Some(loc_id),
                stocktake_date: today(),
                remark: None,
                snapshot_from_balance: true,
                lines: vec![],
            },
        )
        .await
        .unwrap();

    // 不录数直接 submit,应失败(ST_NOT_COUNTED = 47103)
    let err = st.submit(&ctx, head.id).await.expect_err("未录数应失败");
    let dbg = format!("{err:?}");
    assert!(dbg.contains("47103") || dbg.contains("NOT_COUNTED") || dbg.contains("未录入"),
        "expected not-counted error, got: {dbg}");
}

async fn qty_of(
    pool: &PgPool,
    wh_id: i64,
    loc_id: i64,
    material_id: i64,
    batch_no: &str,
    status: &str,
) -> Decimal {
    sqlx::query_scalar::<_, Decimal>(
        r#"select coalesce(sum(book_qty), 0) from inv.balance
            where wh_id=$1 and loc_id=$2 and material_id=$3
              and batch_no=$4 and stock_status=$5"#,
    )
    .bind(wh_id)
    .bind(loc_id)
    .bind(material_id)
    .bind(batch_no)
    .bind(status)
    .fetch_one(pool)
    .await
    .unwrap_or(Decimal::ZERO)
}
