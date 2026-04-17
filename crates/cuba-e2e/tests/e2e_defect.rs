//! defect e2e:TO_BAD_STOCK 会产生 TRANSFER(QUALIFIED → BAD)

use cuba_defect::{CreateDefectCommand, CreateDefectLine, DefectService};
use cuba_inbound::{CreateInboundCommand, CreateInboundLine, InboundService};
use cuba_testkit::{
    fixtures::{admin_ctx, dec, default_bad_wh_loc, default_raw_wh_loc, seed_material, today},
    TestDb,
};
use rust_decimal::Decimal;
use sqlx::PgPool;

#[tokio::test]
async fn defect_to_bad_stock_triggers_transfer() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    let mat_id = seed_material(&pool, "DEF-MAT-001", "RAW", false).await;
    let (raw_wh, raw_loc) = default_raw_wh_loc(&pool).await;
    let (bad_wh, bad_loc) = default_bad_wh_loc(&pool).await;

    // 先入 30 到 QUALIFIED
    let inb = InboundService::new(pool.clone());
    let ih = inb
        .create(
            &ctx,
            CreateInboundCommand {
                inbound_type: "PROD".into(),
                supplier_id: None,
                source_object_type: None,
                source_object_id: None,
                wh_id: raw_wh,
                loc_id: Some(raw_loc),
                inbound_date: today(),
                remark: None,
                lines: vec![CreateInboundLine {
                    line_no: 1,
                    material_id: mat_id,
                    batch_no: "DEF-B1".into(),
                    qty: dec("30"),
                    unit: "PCS".into(),
                    stock_status: None,
                    work_order_no: Some("WO-DEF".into()),
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

    // 登记 10 件 NG,处理方式 TO_BAD_STOCK
    let defect = DefectService::new(pool.clone());
    let dh = defect
        .create(
            &ctx,
            CreateDefectCommand {
                defect_source: "IQC".into(),
                work_order_no: Some("WO-DEF".into()),
                process_name: None,
                product_stage: "RAW".into(),
                found_date: today(),
                finder_name: Some("张三".into()),
                process_method: "TO_BAD_STOCK".into(),
                remark: None,
                source_wh_id: raw_wh,
                source_loc_id: raw_loc,
                target_wh_id: Some(bad_wh),
                target_loc_id: Some(bad_loc),
                lines: vec![CreateDefectLine {
                    line_no: 1,
                    material_id: mat_id,
                    batch_no: "DEF-B1".into(),
                    qty: dec("10"),
                    unit: "PCS".into(),
                    defect_reason: "来料划痕".into(),
                    defect_desc: None,
                    source_doc_type: None,
                    source_doc_no: None,
                    note: None,
                }],
            },
        )
        .await
        .unwrap();
    let res = defect.submit(&ctx, dh.id).await.unwrap();
    assert!(res.txn_no.is_some(), "TO_BAD_STOCK 应该产生 TRANSFER");

    // QUALIFIED 剩 20
    let q = qty_of(&pool, raw_wh, raw_loc, mat_id, "DEF-B1", "QUALIFIED").await;
    assert_eq!(q, dec("20"));
    // BAD 有 10
    let b = qty_of(&pool, bad_wh, bad_loc, mat_id, "DEF-B1", "BAD").await;
    assert_eq!(b, dec("10"));
}

#[tokio::test]
async fn defect_to_dismantle_does_not_move_stock() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    let mat_id = seed_material(&pool, "DEF-MAT-002", "RAW", false).await;
    let (raw_wh, raw_loc) = default_raw_wh_loc(&pool).await;

    // 入 5
    let inb = InboundService::new(pool.clone());
    let ih = inb
        .create(
            &ctx,
            CreateInboundCommand {
                inbound_type: "PROD".into(),
                supplier_id: None,
                source_object_type: None,
                source_object_id: None,
                wh_id: raw_wh,
                loc_id: Some(raw_loc),
                inbound_date: today(),
                remark: None,
                lines: vec![CreateInboundLine {
                    line_no: 1,
                    material_id: mat_id,
                    batch_no: "DEF-B2".into(),
                    qty: dec("5"),
                    unit: "PCS".into(),
                    stock_status: None,
                    work_order_no: Some("WO-DEF2".into()),
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

    // TO_DISMANTLE 不动库存
    let defect = DefectService::new(pool.clone());
    let dh = defect
        .create(
            &ctx,
            CreateDefectCommand {
                defect_source: "PROD".into(),
                work_order_no: Some("WO-DEF2".into()),
                process_name: None,
                product_stage: "FOG".into(),
                found_date: today(),
                finder_name: None,
                process_method: "TO_DISMANTLE".into(),
                remark: None,
                source_wh_id: raw_wh,
                source_loc_id: raw_loc,
                target_wh_id: None,
                target_loc_id: None,
                lines: vec![CreateDefectLine {
                    line_no: 1,
                    material_id: mat_id,
                    batch_no: "DEF-B2".into(),
                    qty: dec("2"),
                    unit: "PCS".into(),
                    defect_reason: "贴合气泡".into(),
                    defect_desc: None,
                    source_doc_type: None,
                    source_doc_no: None,
                    note: None,
                }],
            },
        )
        .await
        .unwrap();
    let res = defect.submit(&ctx, dh.id).await.unwrap();
    assert!(res.txn_no.is_none(), "TO_DISMANTLE 不应产生库存事务");

    // QUALIFIED 仍然 5
    let q = qty_of(&pool, raw_wh, raw_loc, mat_id, "DEF-B2", "QUALIFIED").await;
    assert_eq!(q, dec("5"));
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
