//! 并发测试:两个 outbound 同时出同一 locator
//!
//! 正确行为:库存引擎依赖 DB 行锁 + CHECK (book_qty >= 0 当非 PREISSUE_PENDING),
//! 应保证"最终 balance ≥ 0"且"至少其一成功"。

use std::sync::Arc;

use cuba_inbound::{CreateInboundCommand, CreateInboundLine, InboundService};
use cuba_outbound::{CreateOutboundCommand, CreateOutboundLine, OutboundService};
use cuba_testkit::{
    fixtures::{admin_ctx, dec, default_raw_wh_loc, seed_material, today},
    TestDb,
};
use rust_decimal::Decimal;
use sqlx::PgPool;

#[tokio::test]
async fn concurrent_outbound_never_goes_negative() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    let mat = seed_material(&pool, "CONC-MAT-001", "RAW", false).await;
    let (wh, loc) = default_raw_wh_loc(&pool).await;

    // 先入 100
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
                    batch_no: "CONC-B1".into(),
                    qty: dec("100"),
                    unit: "PCS".into(),
                    stock_status: None,
                    work_order_no: Some("WO-CONC".into()),
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

    assert_eq!(qty(&pool, wh, loc, mat, "QUALIFIED").await, dec("100"));

    // 并发两个出库,各 60(总 120 > 100),应该一个成功一个失败
    let ctx = Arc::new(ctx);
    let mk_out = |name: &str, qty_v: &str| {
        let n = name.to_string();
        let q = qty_v.to_string();
        let pool2 = pool.clone();
        let ctx2 = ctx.clone();
        async move {
            let svc = OutboundService::new(pool2);
            // 先建 DRAFT
            let head = svc
                .create(
                    &ctx2,
                    CreateOutboundCommand {
                        outbound_type: "PROD_ISSUE".into(),
                        target_object_type: None,
                        target_object_id: None,
                        work_order_no: Some(format!("WO-{n}")),
                        process_name: None,
                        route_id: None,
                        workshop_name: None,
                        wh_id: wh,
                        loc_id: loc,
                        outbound_date: today(),
                        remark: None,
                        lines: vec![CreateOutboundLine {
                            line_no: 1,
                            material_id: mat,
                            batch_no: "CONC-B1".into(),
                            suggest_qty: q.parse().unwrap(),
                            actual_qty: q.parse().unwrap(),
                            unit: "PCS".into(),
                            stock_status: "QUALIFIED".into(),
                            bom_recommended_flag: false,
                            public_material_flag: false,
                            preissue_flag: false,
                            note: None,
                        }],
                    },
                )
                .await?;
            svc.submit(&ctx2, head.id).await
        }
    };

    let (r1, r2) = tokio::join!(mk_out("A", "60"), mk_out("B", "60"));

    let ok_count = [r1.is_ok(), r2.is_ok()].iter().filter(|x| **x).count();
    let err_count = 2 - ok_count;
    assert!(
        ok_count >= 1,
        "至少有一个出库应成功(r1={:?}, r2={:?})",
        r1.as_ref().err(),
        r2.as_ref().err(),
    );
    assert!(
        err_count >= 1,
        "至少有一个应失败(总需求 120 > 库存 100);若都成功说明余额穿透了"
    );

    // 终局余额:若 1 成功 → 剩 40;若 2 成功(不应该)→ 负数。但我们已断言至少一失败。
    let final_qty = qty(&pool, wh, loc, mat, "QUALIFIED").await;
    assert_eq!(
        final_qty,
        dec("40"),
        "成功出库一笔,剩余应 100 - 60 = 40(实际:{final_qty})"
    );
}

#[tokio::test]
async fn concurrent_outbound_both_fit_both_succeed() {
    // 入 100,并发出 30 + 40,总 70 < 100,两个都应成功,剩 30
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    let mat = seed_material(&pool, "CONC-MAT-002", "RAW", false).await;
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
                    batch_no: "CONC-B2".into(),
                    qty: dec("100"),
                    unit: "PCS".into(),
                    stock_status: None,
                    work_order_no: Some("WO-CONC2".into()),
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

    let ctx = Arc::new(ctx);
    let mk = |q: &str| {
        let q = q.to_string();
        let pool = pool.clone();
        let ctx = ctx.clone();
        async move {
            let svc = OutboundService::new(pool);
            let head = svc
                .create(
                    &ctx,
                    CreateOutboundCommand {
                        outbound_type: "PROD_ISSUE".into(),
                        target_object_type: None,
                        target_object_id: None,
                        work_order_no: Some(format!("WO-{q}")),
                        process_name: None,
                        route_id: None,
                        workshop_name: None,
                        wh_id: wh,
                        loc_id: loc,
                        outbound_date: today(),
                        remark: None,
                        lines: vec![CreateOutboundLine {
                            line_no: 1,
                            material_id: mat,
                            batch_no: "CONC-B2".into(),
                            suggest_qty: q.parse().unwrap(),
                            actual_qty: q.parse().unwrap(),
                            unit: "PCS".into(),
                            stock_status: "QUALIFIED".into(),
                            bom_recommended_flag: false,
                            public_material_flag: false,
                            preissue_flag: false,
                            note: None,
                        }],
                    },
                )
                .await?;
            svc.submit(&ctx, head.id).await
        }
    };

    let (a, b) = tokio::join!(mk("30"), mk("40"));
    assert!(a.is_ok() && b.is_ok(), "{a:?} {b:?}");
    assert_eq!(qty(&pool, wh, loc, mat, "QUALIFIED").await, dec("30"));
}

async fn qty(
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
