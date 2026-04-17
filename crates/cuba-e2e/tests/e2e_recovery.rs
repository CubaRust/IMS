//! recovery e2e:不良品 → 拆解 → 回收件入库 + 碎料报废
//!
//! 场景:FOG NG 5 件,拆出 FPC 5 件(可回收,QUALIFIED),碎玻璃 5 件报废(SCRAPPED)

use cuba_defect::{CreateDefectCommand, CreateDefectLine, DefectService};
use cuba_inbound::{CreateInboundCommand, CreateInboundLine, InboundService};
use cuba_recovery::{
    CreateRecoveryCommand, CreateRecoveryIn, CreateRecoveryOut, CreateRecoveryScrap,
    RecoveryService,
};
use cuba_testkit::{
    fixtures::{
        admin_ctx, dec, default_bad_wh_loc, default_raw_wh_loc, default_scrap_wh_loc,
        seed_material, today,
    },
    TestDb,
};
use rust_decimal::Decimal;
use sqlx::PgPool;

#[tokio::test]
async fn recovery_convert_from_defect() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    let fog_id = seed_material(&pool, "FOG-SCREEN-001", "SEMI", false).await;
    let fpc_id = seed_material(&pool, "FPC-001", "RAW", false).await;
    let glass_id = seed_material(&pool, "GLASS-SCRAP-001", "SCRAP", false).await;

    let (raw_wh, raw_loc) = default_raw_wh_loc(&pool).await;
    let (bad_wh, bad_loc) = default_bad_wh_loc(&pool).await;
    let (scrap_wh, scrap_loc) = default_scrap_wh_loc(&pool).await;

    // 1. 先入 FOG 5 件 QUALIFIED(到 raw_wh)
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
                    material_id: fog_id,
                    batch_no: "REC-B1".into(),
                    qty: dec("5"),
                    unit: "PCS".into(),
                    stock_status: None,
                    work_order_no: Some("WO-REC".into()),
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

    // 2. 登记为不良 + TO_BAD_STOCK(→ BAD)
    let defect = DefectService::new(pool.clone());
    let dh = defect
        .create(
            &ctx,
            CreateDefectCommand {
                defect_source: "PROD".into(),
                work_order_no: Some("WO-REC".into()),
                process_name: None,
                product_stage: "FOG".into(),
                found_date: today(),
                finder_name: Some("QC".into()),
                process_method: "TO_BAD_STOCK".into(),
                remark: None,
                source_wh_id: raw_wh,
                source_loc_id: raw_loc,
                target_wh_id: Some(bad_wh),
                target_loc_id: Some(bad_loc),
                lines: vec![CreateDefectLine {
                    line_no: 1,
                    material_id: fog_id,
                    batch_no: "REC-B1".into(),
                    qty: dec("5"),
                    unit: "PCS".into(),
                    defect_reason: "气泡".into(),
                    defect_desc: None,
                    source_doc_type: None,
                    source_doc_no: None,
                    note: None,
                }],
            },
        )
        .await
        .unwrap();
    defect.submit(&ctx, dh.id).await.unwrap();

    // 3. 拆解:5 件 FOG(BAD)→ 5 件 FPC(QUALIFIED)+ 5 件碎玻璃(SCRAPPED)
    let rec = RecoveryService::new(pool.clone());
    let rh = rec
        .create(
            &ctx,
            CreateRecoveryCommand {
                source_defect_id: dh.id,
                tpl_id: None,
                recovery_date: today(),
                remark: None,
                source_wh_id: bad_wh,
                source_loc_id: bad_loc,
                scrap_wh_id: scrap_wh,
                scrap_loc_id: scrap_loc,
                inputs: vec![CreateRecoveryIn {
                    line_no: 1,
                    material_id: fog_id,
                    batch_no: "REC-B1".into(),
                    qty: dec("5"),
                    unit: "PCS".into(),
                    note: None,
                }],
                outputs: vec![CreateRecoveryOut {
                    line_no: 1,
                    material_id: fpc_id,
                    qty: dec("5"),
                    unit: "PCS".into(),
                    target_wh_id: raw_wh,
                    target_loc_id: raw_loc,
                    target_status: "QUALIFIED".into(),
                    note: None,
                }],
                scraps: vec![CreateRecoveryScrap {
                    line_no: 1,
                    material_id: Some(glass_id),
                    qty: dec("5"),
                    unit: "PCS".into(),
                    scrap_reason: Some("玻璃破碎".into()),
                    note: None,
                }],
            },
        )
        .await
        .unwrap();
    let res = rec.submit(&ctx, rh.id).await.unwrap();
    assert_eq!(res.doc_status, "COMPLETED");
    assert!(!res.txn_no.is_empty());

    // 4. 对账:
    //    - FOG BAD @ bad_wh 应 = 0(被消费)
    //    - FPC QUALIFIED @ raw_wh 应 = 5
    //    - Glass SCRAPPED 应 = 5
    assert_eq!(
        qty_any_batch(&pool, bad_wh, bad_loc, fog_id, "BAD").await,
        dec("0")
    );
    assert_eq!(
        qty_any_batch(&pool, raw_wh, raw_loc, fpc_id, "QUALIFIED").await,
        dec("5")
    );
    assert_eq!(
        qty_any_batch(&pool, scrap_wh, scrap_loc, glass_id, "SCRAPPED").await,
        dec("5")
    );
}

async fn qty_any_batch(
    pool: &PgPool,
    wh_id: i64,
    loc_id: i64,
    material_id: i64,
    status: &str,
) -> Decimal {
    sqlx::query_scalar::<_, Decimal>(
        r#"select coalesce(sum(book_qty), 0) from inv.balance
            where wh_id=$1 and loc_id=$2 and material_id=$3 and stock_status=$4"#,
    )
    .bind(wh_id)
    .bind(loc_id)
    .bind(material_id)
    .bind(status)
    .fetch_one(pool)
    .await
    .unwrap_or(Decimal::ZERO)
}
