//! customer-return / supplier-return / pmc e2e

use cuba_customer_return::{
    CreateCustomerReturnCommand, CreateCustomerReturnLine, CustomerReturnService, JudgeLineCommand,
};
use cuba_inbound::{CreateInboundCommand, CreateInboundLine, InboundService};
use cuba_pmc::{
    CreateOutsourceCommand, CreateOutsourceLine, PmcService, SubmitBackCommand, SubmitBackLine,
};
use cuba_supplier_return::{
    CreateSupplierReturnCommand, CreateSupplierReturnLine, SupplierReturnService,
};
use cuba_testkit::{
    fixtures::{
        admin_ctx, dec, default_bad_wh_loc, default_raw_wh_loc, default_scrap_wh_loc,
        seed_customer, seed_material, seed_supplier, today,
    },
    TestDb,
};
use rust_decimal::Decimal;
use sqlx::PgPool;

#[tokio::test]
async fn customer_return_routes_by_judge() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    let cust = seed_customer(&pool, "CUST-001").await;
    let good_mat = seed_material(&pool, "CR-GOOD", "FINISHED", false).await;
    let bad_mat = seed_material(&pool, "CR-BAD", "FINISHED", false).await;

    let (raw_wh, raw_loc) = default_raw_wh_loc(&pool).await;
    let (bad_wh, bad_loc) = default_bad_wh_loc(&pool).await;
    let (scrap_wh, scrap_loc) = default_scrap_wh_loc(&pool).await;

    let svc = CustomerReturnService::new(pool.clone());
    let head = svc
        .create(
            &ctx,
            CreateCustomerReturnCommand {
                customer_id: cust,
                return_date: today(),
                original_doc_no: Some("SO-2026-001".into()),
                remark: None,
                return_wh_id: raw_wh,
                return_loc_id: raw_loc,
                defect_wh_id: Some(bad_wh),
                defect_loc_id: Some(bad_loc),
                scrap_wh_id: Some(scrap_wh),
                scrap_loc_id: Some(scrap_loc),
                lines: vec![
                    CreateCustomerReturnLine {
                        line_no: 1,
                        material_id: good_mat,
                        batch_no: "CR-B1".into(),
                        qty: dec("3"),
                        unit: "PCS".into(),
                        return_reason: "客户退货质量OK".into(),
                        judge_method: None,
                        judge_note: None,
                        note: None,
                    },
                    CreateCustomerReturnLine {
                        line_no: 2,
                        material_id: bad_mat,
                        batch_no: "CR-B2".into(),
                        qty: dec("2"),
                        unit: "PCS".into(),
                        return_reason: "客户退货不良".into(),
                        judge_method: None,
                        judge_note: None,
                        note: None,
                    },
                ],
            },
        )
        .await
        .unwrap();

    // 判定:line1 RETURN_TO_STOCK、line2 TO_DEFECT
    svc.judge(
        &ctx,
        head.id,
        vec![
            JudgeLineCommand {
                line_id: head.lines[0].id,
                judge_method: "RETURN_TO_STOCK".into(),
                judge_note: None,
            },
            JudgeLineCommand {
                line_id: head.lines[1].id,
                judge_method: "TO_DEFECT".into(),
                judge_note: None,
            },
        ],
    )
    .await
    .unwrap();

    let res = svc.submit(&ctx, head.id).await.unwrap();
    assert_eq!(res.doc_status, "COMPLETED");

    // 对账
    assert_eq!(
        qty_any(&pool, raw_wh, raw_loc, good_mat, "QUALIFIED").await,
        dec("3")
    );
    assert_eq!(
        qty_any(&pool, bad_wh, bad_loc, bad_mat, "BAD").await,
        dec("2")
    );
}

#[tokio::test]
async fn supplier_return_out_from_bad() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    let sup = seed_supplier(&pool, "SUP-001").await;
    let mat = seed_material(&pool, "SR-MAT", "RAW", false).await;
    let (bad_wh, bad_loc) = default_bad_wh_loc(&pool).await;

    // 先直接 PROD 入库进 BAD 仓(用 stock_status 手动指定 BAD)
    let inb = InboundService::new(pool.clone());
    let ih = inb
        .create(
            &ctx,
            CreateInboundCommand {
                inbound_type: "OTHER".into(),
                supplier_id: Some(sup),
                source_object_type: None,
                source_object_id: None,
                wh_id: bad_wh,
                loc_id: Some(bad_loc),
                inbound_date: today(),
                remark: None,
                lines: vec![CreateInboundLine {
                    line_no: 1,
                    material_id: mat,
                    batch_no: "SR-B1".into(),
                    qty: dec("10"),
                    unit: "PCS".into(),
                    stock_status: Some("BAD".into()),
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

    // 退 7 件给供应商
    let sr = SupplierReturnService::new(pool.clone());
    let rh = sr
        .create(
            &ctx,
            CreateSupplierReturnCommand {
                supplier_id: sup,
                return_date: today(),
                original_doc_no: None,
                remark: None,
                source_wh_id: bad_wh,
                source_loc_id: bad_loc,
                lines: vec![CreateSupplierReturnLine {
                    line_no: 1,
                    material_id: mat,
                    batch_no: "SR-B1".into(),
                    qty: dec("7"),
                    unit: "PCS".into(),
                    source_status: "BAD".into(),
                    return_reason: "来料来货质量差".into(),
                    note: None,
                }],
            },
        )
        .await
        .unwrap();
    sr.submit(&ctx, rh.id).await.unwrap();

    assert_eq!(qty_any(&pool, bad_wh, bad_loc, mat, "BAD").await, dec("3"));
}

#[tokio::test]
async fn pmc_send_and_partial_back() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    let sup = seed_supplier(&pool, "PMC-SUP").await;
    let mat = seed_material(&pool, "PMC-MAT", "SEMI", false).await;
    let (raw_wh, raw_loc) = default_raw_wh_loc(&pool).await;

    // 先入 20 到 raw(QUALIFIED)
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
                    material_id: mat,
                    batch_no: "PMC-B1".into(),
                    qty: dec("20"),
                    unit: "PCS".into(),
                    stock_status: None,
                    work_order_no: Some("WO-PMC".into()),
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

    // 建委外单:送 20
    let pmc = PmcService::new(pool.clone());
    let oh = pmc
        .create(
            &ctx,
            CreateOutsourceCommand {
                supplier_id: sup,
                issue_date: today(),
                remark: None,
                send_wh_id: raw_wh,
                send_loc_id: raw_loc,
                back_wh_id: raw_wh,
                back_loc_id: raw_loc,
                send_lines: vec![CreateOutsourceLine {
                    line_no: 1,
                    material_id: mat,
                    batch_no: "PMC-B1".into(),
                    qty: dec("20"),
                    unit: "PCS".into(),
                    note: None,
                }],
            },
        )
        .await
        .unwrap();
    pmc.submit_send(&ctx, oh.id).await.unwrap();

    // 送完后 raw 应 = 0
    assert_eq!(
        qty_any(&pool, raw_wh, raw_loc, mat, "QUALIFIED").await,
        dec("0")
    );

    // 第一次回 12 → PARTIAL
    pmc.submit_back(
        &ctx,
        oh.id,
        SubmitBackCommand {
            back_lines: vec![SubmitBackLine {
                material_id: mat,
                batch_no: "PMC-B1".into(),
                qty: dec("12"),
                unit: "PCS".into(),
                note: None,
            }],
        },
    )
    .await
    .unwrap();
    let head = pmc.get(oh.id).await.unwrap();
    assert_eq!(head.back_status, "PARTIAL");
    assert_eq!(
        qty_any(&pool, raw_wh, raw_loc, mat, "TO_CHECK").await,
        dec("12")
    );

    // 再回 8 → COMPLETED
    pmc.submit_back(
        &ctx,
        oh.id,
        SubmitBackCommand {
            back_lines: vec![SubmitBackLine {
                material_id: mat,
                batch_no: "PMC-B1".into(),
                qty: dec("8"),
                unit: "PCS".into(),
                note: None,
            }],
        },
    )
    .await
    .unwrap();
    let head = pmc.get(oh.id).await.unwrap();
    assert_eq!(head.back_status, "COMPLETED");
    assert_eq!(head.doc_status, "COMPLETED");
}

async fn qty_any(
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
