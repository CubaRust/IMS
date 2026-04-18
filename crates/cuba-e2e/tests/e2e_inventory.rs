//! inventory 引擎 e2e:直接调 InventoryService,不经业务单据

use cuba_inventory::{CommitTxnCommand, InventoryService, TxnLineInput, TxnSideInput};
use cuba_shared::types::{IoFlag, StockStatus, TxnType};
use cuba_testkit::{
    fixtures::{admin_ctx, dec, default_raw_wh_loc, seed_material},
    TestDb,
};

#[tokio::test]
async fn inventory_raw_in_then_transfer_between_status() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    let mat = seed_material(&pool, "INV-MAT-001", "RAW", false).await;
    let (wh, loc) = default_raw_wh_loc(&pool).await;

    let svc = InventoryService::new(pool.clone());

    // 1. 原生 IN 50 到 TO_CHECK
    svc.commit(
        &ctx,
        CommitTxnCommand {
            txn_type: TxnType::In,
            scene_code: "RAW_TEST_IN".into(),
            scene_name: None,
            doc_type: "TEST".into(),
            doc_no: "TEST-1".into(),
            source_object_type: None,
            source_object_id: None,
            target_object_type: None,
            target_object_id: None,
            source: None,
            target: Some(TxnSideInput {
                wh_id: wh,
                loc_id: loc,
                status: Some(StockStatus::ToCheck),
            }),
            lines: vec![TxnLineInput {
                line_no: 1,
                material_id: mat,
                batch_no: "INV-B1".into(),
                qty: dec("50"),
                unit: "PCS".into(),
                io_flag: IoFlag::In,
                source_material_id: None,
                target_material_id: None,
                stock_status: Some(StockStatus::ToCheck),
                status_change_flag: false,
                location_change_flag: false,
                item_change_flag: false,
                recoverable_flag: false,
                scrap_flag: false,
                note: None,
            }],
            is_exception: false,
            exception_type: None,
            related_doc_no: None,
            snapshot_json: None,
            remark: None,
        },
    )
    .await
    .expect("commit IN");

    // 2. TRANSFER 30 件从 TO_CHECK → QUALIFIED(同仓位,只改状态)
    svc.commit(
        &ctx,
        CommitTxnCommand {
            txn_type: TxnType::Transfer,
            scene_code: "RAW_TEST_TRANSFER".into(),
            scene_name: None,
            doc_type: "TEST".into(),
            doc_no: "TEST-2".into(),
            source_object_type: None,
            source_object_id: None,
            target_object_type: None,
            target_object_id: None,
            source: Some(TxnSideInput {
                wh_id: wh,
                loc_id: loc,
                status: Some(StockStatus::ToCheck),
            }),
            target: Some(TxnSideInput {
                wh_id: wh,
                loc_id: loc,
                status: Some(StockStatus::Qualified),
            }),
            lines: vec![
                TxnLineInput {
                    line_no: 1,
                    material_id: mat,
                    batch_no: "INV-B1".into(),
                    qty: dec("30"),
                    unit: "PCS".into(),
                    io_flag: IoFlag::Out,
                    source_material_id: None,
                    target_material_id: None,
                    stock_status: Some(StockStatus::ToCheck),
                    status_change_flag: true,
                    location_change_flag: false,
                    item_change_flag: false,
                    recoverable_flag: false,
                    scrap_flag: false,
                    note: None,
                },
                TxnLineInput {
                    line_no: 2,
                    material_id: mat,
                    batch_no: "INV-B1".into(),
                    qty: dec("30"),
                    unit: "PCS".into(),
                    io_flag: IoFlag::In,
                    source_material_id: None,
                    target_material_id: None,
                    stock_status: Some(StockStatus::Qualified),
                    status_change_flag: true,
                    location_change_flag: false,
                    item_change_flag: false,
                    recoverable_flag: false,
                    scrap_flag: false,
                    note: None,
                },
            ],
            is_exception: false,
            exception_type: None,
            related_doc_no: None,
            snapshot_json: None,
            remark: None,
        },
    )
    .await
    .expect("commit TRANSFER");

    // 对账
    let to_check: rust_decimal::Decimal = sqlx::query_scalar(
        r#"select coalesce(sum(book_qty),0) from inv.balance
            where wh_id=$1 and loc_id=$2 and material_id=$3
              and batch_no='INV-B1' and stock_status='TO_CHECK'"#,
    )
    .bind(wh)
    .bind(loc)
    .bind(mat)
    .fetch_one(&pool)
    .await
    .unwrap();
    let qualified: rust_decimal::Decimal = sqlx::query_scalar(
        r#"select coalesce(sum(book_qty),0) from inv.balance
            where wh_id=$1 and loc_id=$2 and material_id=$3
              and batch_no='INV-B1' and stock_status='QUALIFIED'"#,
    )
    .bind(wh)
    .bind(loc)
    .bind(mat)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(to_check, dec("20"));
    assert_eq!(qualified, dec("30"));
}
