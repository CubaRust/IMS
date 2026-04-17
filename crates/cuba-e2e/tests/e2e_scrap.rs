//! scrap e2e:从 BAD 仓 → SCRAPPED 仓

use cuba_inbound::{CreateInboundCommand, CreateInboundLine, InboundService};
use cuba_scrap::{CreateScrapCommand, CreateScrapLine, ScrapService};
use cuba_testkit::{
    fixtures::{
        admin_ctx, dec, default_bad_wh_loc, default_scrap_wh_loc, seed_material, today,
    },
    TestDb,
};
use rust_decimal::Decimal;
use sqlx::PgPool;

#[tokio::test]
async fn scrap_from_bad_to_scrapped() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    let mat = seed_material(&pool, "SCRAP-MAT", "RAW", false).await;
    let (bad_wh, bad_loc) = default_bad_wh_loc(&pool).await;
    let (scrap_wh, scrap_loc) = default_scrap_wh_loc(&pool).await;

    // 先把 10 件 BAD 料造到 BAD 仓(用 stock_status=BAD 的显式入库)
    let inb = InboundService::new(pool.clone());
    let ih = inb
        .create(
            &ctx,
            CreateInboundCommand {
                inbound_type: "OTHER".into(),
                supplier_id: None,
                source_object_type: None,
                source_object_id: None,
                wh_id: bad_wh,
                loc_id: Some(bad_loc),
                inbound_date: today(),
                remark: None,
                lines: vec![CreateInboundLine {
                    line_no: 1,
                    material_id: mat,
                    batch_no: "SCR-B1".into(),
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

    // 报废 6 件
    let scrap = ScrapService::new(pool.clone());
    let sh = scrap
        .create(
            &ctx,
            CreateScrapCommand {
                scrap_source: "PROD_BAD".into(),
                source_doc_type: None,
                source_doc_no: None,
                scrap_date: today(),
                remark: None,
                source_wh_id: bad_wh,
                source_loc_id: bad_loc,
                scrap_wh_id: scrap_wh,
                scrap_loc_id: scrap_loc,
                lines: vec![CreateScrapLine {
                    line_no: 1,
                    material_id: mat,
                    batch_no: "SCR-B1".into(),
                    qty: dec("6"),
                    unit: "PCS".into(),
                    stock_status: "BAD".into(),
                    scrap_reason: "不可修复".into(),
                    note: None,
                }],
            },
        )
        .await
        .unwrap();
    scrap.submit(&ctx, sh.id).await.expect("submit scrap");

    // 对账:BAD 仓剩 4,SCRAP 仓 +6
    assert_eq!(qty(&pool, bad_wh, bad_loc, mat, "BAD").await, dec("4"));
    assert_eq!(
        qty(&pool, scrap_wh, scrap_loc, mat, "SCRAPPED").await,
        dec("6")
    );
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
