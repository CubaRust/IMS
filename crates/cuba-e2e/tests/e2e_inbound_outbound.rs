//! inbound + outbound + inventory 端到端集成测试

use rust_decimal::Decimal;

use cuba_inbound::{CreateInboundCommand, CreateInboundLine, InboundService};
use cuba_outbound::{CreateOutboundCommand, CreateOutboundLine, OutboundService};
use cuba_testkit::{
    fixtures::{admin_ctx, dec, default_raw_wh_loc, seed_material, today},
    TestDb,
};

/// 端到端:建物料 → 入库 100 → 出库 30 → 余额 = 70(QUALIFIED)
#[tokio::test]
async fn e2e_inbound_then_outbound_balance_correct() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    let mat_id = seed_material(&pool, "TEST-MAT-001", "RAW", false).await;
    let (wh_id, loc_id) = default_raw_wh_loc(&pool).await;

    // 入库 100(PURCHASE → 默认 TO_CHECK,但 outbound 要从 QUALIFIED 出,
    //  所以我们建一个 PROD 类型的入库,默认 QUALIFIED)
    let inbound = InboundService::new(pool.clone());
    let head = inbound
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
                remark: Some("e2e test".into()),
                lines: vec![CreateInboundLine {
                    line_no: 1,
                    material_id: mat_id,
                    batch_no: "B-E2E-01".into(),
                    qty: dec("100"),
                    unit: "PCS".into(),
                    stock_status: None,
                    work_order_no: Some("WO-TEST".into()),
                    process_name: None,
                    outsource_no: None,
                    related_preissue_line_id: None,
                    note: None,
                }],
            },
        )
        .await
        .expect("create inbound");
    let submit = inbound.submit(&ctx, head.id).await.expect("submit inbound");
    assert_eq!(submit.doc_status, "COMPLETED");

    // 验余额 100 QUALIFIED
    let qty = query_balance(&pool, wh_id, loc_id, mat_id, "B-E2E-01", "QUALIFIED").await;
    assert_eq!(qty, dec("100"));

    // 出库 30
    let outbound = OutboundService::new(pool.clone());
    let out_head = outbound
        .create(
            &ctx,
            CreateOutboundCommand {
                outbound_type: "PROD_ISSUE".into(),
                target_object_type: None,
                target_object_id: None,
                work_order_no: Some("WO-TEST".into()),
                process_name: None,
                route_id: None,
                workshop_name: None,
                wh_id,
                loc_id,
                outbound_date: today(),
                remark: None,
                lines: vec![CreateOutboundLine {
                    line_no: 1,
                    material_id: mat_id,
                    batch_no: "B-E2E-01".into(),
                    suggest_qty: dec("30"),
                    actual_qty: dec("30"),
                    unit: "PCS".into(),
                    stock_status: "QUALIFIED".into(),
                    bom_recommended_flag: false,
                    public_material_flag: false,
                    preissue_flag: false,
                    note: None,
                }],
            },
        )
        .await
        .expect("create outbound");
    outbound
        .submit(&ctx, out_head.id)
        .await
        .expect("submit outbound");

    // 验余额 70
    let qty = query_balance(&pool, wh_id, loc_id, mat_id, "B-E2E-01", "QUALIFIED").await;
    assert_eq!(qty, dec("70"));
}

/// 出库超库存:应抛 20101 INV_INSUFFICIENT
#[tokio::test]
async fn e2e_outbound_insufficient_stock_returns_20101() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    let mat_id = seed_material(&pool, "TEST-MAT-SHORT", "RAW", false).await;
    let (wh_id, loc_id) = default_raw_wh_loc(&pool).await;

    // 先入 10
    let inbound = InboundService::new(pool.clone());
    let h = inbound
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
                    batch_no: "B-X".into(),
                    qty: dec("10"),
                    unit: "PCS".into(),
                    stock_status: None,
                    work_order_no: Some("WO-X".into()),
                    process_name: None,
                    outsource_no: None,
                    related_preissue_line_id: None,
                    note: None,
                }],
            },
        )
        .await
        .unwrap();
    inbound.submit(&ctx, h.id).await.unwrap();

    // 出 100,应失败
    let outbound = OutboundService::new(pool.clone());
    let oh = outbound
        .create(
            &ctx,
            CreateOutboundCommand {
                outbound_type: "PROD_ISSUE".into(),
                target_object_type: None,
                target_object_id: None,
                work_order_no: Some("WO-X".into()),
                process_name: None,
                route_id: None,
                workshop_name: None,
                wh_id,
                loc_id,
                outbound_date: today(),
                remark: None,
                lines: vec![CreateOutboundLine {
                    line_no: 1,
                    material_id: mat_id,
                    batch_no: "B-X".into(),
                    suggest_qty: dec("100"),
                    actual_qty: dec("100"),
                    unit: "PCS".into(),
                    stock_status: "QUALIFIED".into(),
                    bom_recommended_flag: false,
                    public_material_flag: false,
                    preissue_flag: false,
                    note: None,
                }],
            },
        )
        .await
        .unwrap();

    let err = outbound.submit(&ctx, oh.id).await.expect_err("should fail");
    let code = err.error_code_value();
    assert_eq!(code, 20101, "expected INV_INSUFFICIENT, got {err:?}");
}

// -- helpers --

async fn query_balance(
    pool: &sqlx::PgPool,
    wh_id: i64,
    loc_id: i64,
    material_id: i64,
    batch_no: &str,
    status: &str,
) -> Decimal {
    sqlx::query_scalar::<_, Decimal>(
        r#"
        select coalesce(sum(book_qty), 0)
          from inv.balance
         where wh_id = $1 and loc_id = $2
           and material_id = $3 and batch_no = $4
           and stock_status = $5
        "#,
    )
    .bind(wh_id)
    .bind(loc_id)
    .bind(material_id)
    .bind(batch_no)
    .bind(status)
    .fetch_one(pool)
    .await
    .expect("query balance")
}

// ---------------------------------------------------------------------------
// 需要 AppError 暴露 error_code_value;若现成没有则使用 debug 字符串检查
// ---------------------------------------------------------------------------
use cuba_shared::error::AppError;

trait AppErrorExt {
    fn error_code_value(&self) -> u32;
}

impl AppErrorExt for AppError {
    fn error_code_value(&self) -> u32 {
        // 回退方案:通过 code 字段访问;实际 AppError 内部有 code: ErrorCode
        let dbg = format!("{self:?}");
        for p in dbg.split(|c: char| !c.is_ascii_digit()) {
            if let Ok(n) = p.parse::<u32>() {
                if n > 1000 {
                    return n;
                }
            }
        }
        0
    }
}
