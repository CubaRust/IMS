//! e2e tests for newly implemented features:
//! - 7 report view APIs
//! - BOM recommend
//! - Dashboard
//! - Dict CRUD + DocNoRule CRUD
//! - Recovery template CRUD
//! - Customer return judge → wms_customer_return_judge

use cuba_catalog::{
    BomService, CreateBomCommand, CreateBomLine, CreateRecoveryTplCommand, CreateRecoveryTplLine,
    QueryBomRecommend, QueryRecoveryTpls, RecoveryTplService,
};
use cuba_customer_return::{
    CreateCustomerReturnCommand, CreateCustomerReturnLine, CustomerReturnService, JudgeLineCommand,
};
use cuba_identity::{
    CreateDictCommand, QueryDicts, SystemConfigService, UpdateDictCommand, UpdateDocNoRuleCommand,
};
use cuba_reporting::ReportingService;
use cuba_testkit::{
    fixtures::{
        admin_ctx, dec, default_bad_wh_loc, default_raw_wh_loc, default_scrap_wh_loc,
        seed_customer, seed_material, today,
    },
    http::spawn_server,
    TestDb,
};
use rust_decimal::Decimal;
use serde_json::{json, Value};

// ============================================================================
// 1. Report view APIs (HTTP level)
// ============================================================================

/// Helper: login and return token
async fn login_token(base: &str, client: &reqwest::Client) -> String {
    let r: Value = client
        .post(format!("{base}/api/v1/auth/login"))
        .json(&json!({ "login_name": "admin", "password": "Admin@123" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    r["data"]["token"].as_str().expect("token").to_string()
}

// PLACEHOLDER_REPORT_TESTS

#[tokio::test]
async fn report_endpoints_return_ok() {
    let db = TestDb::new().await;
    let (base, _h) = spawn_server(db.pool_owned()).await;
    let client = reqwest::Client::new();
    let token = login_token(&base, &client).await;

    let endpoints = [
        "/api/v1/reports/inventory-by-material",
        "/api/v1/reports/inventory-by-location",
        "/api/v1/reports/low-stock-warning",
        "/api/v1/reports/anomaly-todo",
        "/api/v1/reports/today-io",
        "/api/v1/reports/defect-stats",
        "/api/v1/reports/outsource-in-transit",
    ];

    for ep in endpoints {
        let r: Value = client
            .get(format!("{base}{ep}"))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(
            r["code"].as_i64().unwrap_or(-1),
            0,
            "endpoint {ep} failed: {r}"
        );
        assert!(r["data"].is_array(), "endpoint {ep} data not array: {r}");
    }
}

#[tokio::test]
async fn report_inventory_by_material_returns_seeded_data() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let svc = ReportingService::new(pool);

    let rows = svc
        .inventory_by_material(&cuba_reporting::QueryInventoryByMaterial::default())
        .await
        .expect("inventory_by_material");
    // seed 里有 active 物料,视图应该返回行(即使余额为 0)
    assert!(!rows.is_empty(), "should have materials from seed");
}

#[tokio::test]
async fn report_low_stock_warning_filters_by_level() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let svc = ReportingService::new(pool);

    // 查 CRITICAL 级别(可能为空,但不应报错)
    let rows = svc
        .low_stock_warning(&cuba_reporting::QueryLowStockWarning {
            warning_level: Some("CRITICAL".into()),
            material_category: None,
        })
        .await
        .expect("low_stock_warning CRITICAL");
    for r in &rows {
        assert_eq!(r.warning_level, "CRITICAL");
    }
}

#[tokio::test]
async fn report_anomaly_todo_filters_by_type() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let svc = ReportingService::new(pool);

    let rows = svc
        .anomaly_todo(&cuba_reporting::QueryAnomalyTodo {
            anomaly_type: Some("PREISSUE".into()),
        })
        .await
        .expect("anomaly_todo PREISSUE");
    for r in &rows {
        assert_eq!(r.anomaly_type, "PREISSUE");
    }
}

// ============================================================================
// 2. Dashboard
// ============================================================================

#[tokio::test]
async fn dashboard_returns_aggregated_metrics() {
    let db = TestDb::new().await;
    let (base, _h) = spawn_server(db.pool_owned()).await;
    let client = reqwest::Client::new();
    let token = login_token(&base, &client).await;

    let r: Value = client
        .get(format!("{base}/api/v1/dashboard"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(r["code"].as_i64().unwrap_or(-1), 0, "dashboard: {r}");
    let data = &r["data"];
    // All counters should be non-negative integers
    assert!(data["total_material_count"].as_i64().unwrap_or(-1) >= 0);
    assert!(data["low_stock_warning_count"].as_i64().unwrap_or(-1) >= 0);
    assert!(data["anomaly_todo_count"].as_i64().unwrap_or(-1) >= 0);
}

#[tokio::test]
async fn dashboard_service_level() {
    let db = TestDb::new().await;
    let svc = ReportingService::new(db.pool_owned());

    let d = svc.dashboard().await.expect("dashboard");
    // seed 里有物料
    assert!(d.total_material_count > 0, "should have seeded materials");
    assert!(d.today_inbound_count >= 0);
    assert!(d.today_outbound_count >= 0);
}

// ============================================================================
// 3. BOM Recommend
// ============================================================================

#[tokio::test]
async fn bom_recommend_expands_lines() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    let product = seed_material(&pool, "BOM-PROD-001", "FINISHED", false).await;
    let comp_a = seed_material(&pool, "BOM-COMP-A", "RAW", false).await;
    let comp_b = seed_material(&pool, "BOM-COMP-B", "RAW", false).await;

    let bom_svc = BomService::new(pool.clone());
    let bom = bom_svc
        .create(
            &ctx,
            CreateBomCommand {
                bom_code: "BOM-TEST-001".into(),
                bom_version: "V1".into(),
                product_material_id: product,
                route_id: None,
                remark: None,
                lines: vec![
                    CreateBomLine {
                        line_no: 1,
                        material_id: comp_a,
                        usage_qty: dec("2"),
                        loss_rate: dec("0.05"),
                        public_material_flag: false,
                        remark: None,
                    },
                    CreateBomLine {
                        line_no: 2,
                        material_id: comp_b,
                        usage_qty: dec("1"),
                        loss_rate: Decimal::ZERO,
                        public_material_flag: true,
                        remark: None,
                    },
                ],
            },
        )
        .await
        .unwrap();

    // Recommend for production_qty = 10
    let result = bom_svc
        .recommend(&QueryBomRecommend {
            product_material_id: product,
            production_qty: dec("10"),
            bom_id: None,
        })
        .await
        .expect("bom recommend");

    assert_eq!(result.bom_id, bom.id);
    assert_eq!(result.lines.len(), 2);

    // comp_a: 2 * 10 * 1.05 = 21
    let line_a = result
        .lines
        .iter()
        .find(|l| l.material_id == comp_a)
        .unwrap();
    assert_eq!(line_a.recommend_qty, dec("21"));

    // comp_b: 1 * 10 * 1.0 = 10
    let line_b = result
        .lines
        .iter()
        .find(|l| l.material_id == comp_b)
        .unwrap();
    assert_eq!(line_b.recommend_qty, dec("10"));
    assert!(line_b.public_material_flag);
}

#[tokio::test]
async fn bom_recommend_no_active_bom_returns_not_found() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();

    let mat = seed_material(&pool, "BOM-ORPHAN", "FINISHED", false).await;
    let bom_svc = BomService::new(pool);

    let err = bom_svc
        .recommend(&QueryBomRecommend {
            product_material_id: mat,
            production_qty: dec("1"),
            bom_id: None,
        })
        .await
        .expect_err("should fail without BOM");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("没有激活的 BOM") || msg.contains("not_found") || msg.contains("404"),
        "expected not_found: {msg}"
    );
}

#[tokio::test]
async fn bom_recommend_zero_qty_rejected() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();

    let mat = seed_material(&pool, "BOM-ZERO", "FINISHED", false).await;
    let bom_svc = BomService::new(pool);

    let err = bom_svc
        .recommend(&QueryBomRecommend {
            product_material_id: mat,
            production_qty: dec("0"),
            bom_id: None,
        })
        .await
        .expect_err("zero qty should fail");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("生产数量") || msg.contains("validation"),
        "expected validation error: {msg}"
    );
}

// ============================================================================
// 4. Dict CRUD + DocNoRule CRUD
// ============================================================================

#[tokio::test]
async fn dict_crud_lifecycle() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let svc = SystemConfigService::new(pool);

    // List existing dicts (seed has many)
    let all = svc
        .list_dicts(&QueryDicts::default())
        .await
        .expect("list all dicts");
    assert!(!all.is_empty(), "seed should have dicts");

    // Filter by type
    let cats = svc
        .list_dicts(&QueryDicts {
            dict_type: Some("MATERIAL_CATEGORY".into()),
        })
        .await
        .expect("list MATERIAL_CATEGORY");
    assert!(cats.len() >= 5, "should have RAW/SEMI/FINISHED/...");
    for d in &cats {
        assert_eq!(d.dict_type, "MATERIAL_CATEGORY");
    }

    // Create
    let created = svc
        .create_dict(CreateDictCommand {
            dict_type: "TEST_TYPE".into(),
            dict_key: "KEY1".into(),
            dict_value: "值1".into(),
            dict_order: 1,
            remark: Some("测试".into()),
        })
        .await
        .expect("create dict");
    assert_eq!(created.dict_type, "TEST_TYPE");
    assert_eq!(created.dict_key, "KEY1");
    assert!(created.is_active);

    // Update
    let updated = svc
        .update_dict(
            created.id,
            UpdateDictCommand {
                dict_value: Some("值1-修改".into()),
                dict_order: Some(99),
                is_active: None,
                remark: None,
            },
        )
        .await
        .expect("update dict");
    assert_eq!(updated.dict_value, "值1-修改");
    assert_eq!(updated.dict_order, 99);

    // Deactivate
    let deactivated = svc
        .update_dict(
            created.id,
            UpdateDictCommand {
                dict_value: None,
                dict_order: None,
                is_active: Some(false),
                remark: None,
            },
        )
        .await
        .expect("deactivate dict");
    assert!(!deactivated.is_active);
}

#[tokio::test]
async fn dict_duplicate_key_rejected() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let svc = SystemConfigService::new(pool);

    svc.create_dict(CreateDictCommand {
        dict_type: "DUP_TEST".into(),
        dict_key: "SAME".into(),
        dict_value: "first".into(),
        dict_order: 1,
        remark: None,
    })
    .await
    .expect("first insert");

    let err = svc
        .create_dict(CreateDictCommand {
            dict_type: "DUP_TEST".into(),
            dict_key: "SAME".into(),
            dict_value: "second".into(),
            dict_order: 2,
            remark: None,
        })
        .await
        .expect_err("duplicate should fail");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("唯一约束") || msg.contains("conflict") || msg.contains("23505"),
        "expected conflict: {msg}"
    );
}

#[tokio::test]
async fn doc_no_rule_list_and_update() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let svc = SystemConfigService::new(pool);

    let rules = svc.list_doc_no_rules().await.expect("list rules");
    assert!(!rules.is_empty(), "seed should have doc_no_rules");

    // Find INBOUND rule
    let inb_rule = rules
        .iter()
        .find(|r| r.doc_type == "INBOUND")
        .expect("INBOUND rule");
    assert_eq!(inb_rule.doc_prefix, "INB");

    // Update prefix
    let updated = svc
        .update_doc_no_rule(
            inb_rule.id,
            UpdateDocNoRuleCommand {
                doc_prefix: Some("IN".into()),
                date_pattern: None,
                seq_length: Some(6),
            },
        )
        .await
        .expect("update rule");
    assert_eq!(updated.doc_prefix, "IN");
    assert_eq!(updated.seq_length, 6);
}

// ============================================================================
// 5. Recovery Template CRUD
// ============================================================================

#[tokio::test]
async fn recovery_tpl_create_and_get() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    let source = seed_material(&pool, "RCV-SRC", "FINISHED", false).await;
    let target_a = seed_material(&pool, "RCV-TGT-A", "SEMI", false).await;
    let target_b = seed_material(&pool, "RCV-TGT-B", "RECOVERY", false).await;

    let svc = RecoveryTplService::new(pool.clone());

    let head = svc
        .create(
            &ctx,
            CreateRecoveryTplCommand {
                tpl_code: "TPL-001".into(),
                tpl_name: "测试拆解模板".into(),
                source_material_id: source,
                remark: Some("e2e test".into()),
                lines: vec![
                    CreateRecoveryTplLine {
                        line_no: 1,
                        target_material_id: Some(target_a),
                        default_recovery_qty: dec("0.5"),
                        target_default_status: Some("QUALIFIED".into()),
                        scrap_flag: false,
                        remark: None,
                    },
                    CreateRecoveryTplLine {
                        line_no: 2,
                        target_material_id: Some(target_b),
                        default_recovery_qty: dec("0.3"),
                        target_default_status: None,
                        scrap_flag: true,
                        remark: Some("报废件".into()),
                    },
                ],
            },
        )
        .await
        .expect("create recovery tpl");

    assert_eq!(head.tpl_code, "TPL-001");
    assert_eq!(head.lines.len(), 2);
    assert!(head.is_active);

    // Get by id
    let fetched = svc.get(head.id).await.expect("get recovery tpl");
    assert_eq!(fetched.tpl_code, "TPL-001");
    assert_eq!(fetched.lines.len(), 2);
    assert_eq!(fetched.lines[0].default_recovery_qty, dec("0.5"));
    assert!(fetched.lines[1].scrap_flag);
}

#[tokio::test]
async fn recovery_tpl_list_filters() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    let source = seed_material(&pool, "RCV-LIST-SRC", "FINISHED", false).await;
    let target = seed_material(&pool, "RCV-LIST-TGT", "SEMI", false).await;

    let svc = RecoveryTplService::new(pool.clone());
    svc.create(
        &ctx,
        CreateRecoveryTplCommand {
            tpl_code: "TPL-LIST-001".into(),
            tpl_name: "列表测试".into(),
            source_material_id: source,
            remark: None,
            lines: vec![CreateRecoveryTplLine {
                line_no: 1,
                target_material_id: Some(target),
                default_recovery_qty: dec("1"),
                target_default_status: None,
                scrap_flag: false,
                remark: None,
            }],
        },
    )
    .await
    .unwrap();

    // Filter by code
    let by_code = svc
        .list(&QueryRecoveryTpls {
            tpl_code: Some("TPL-LIST-001".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(by_code.len(), 1);

    // Filter by source material
    let by_mat = svc
        .list(&QueryRecoveryTpls {
            source_material_id: Some(source),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(!by_mat.is_empty());

    // Non-existent code
    let empty = svc
        .list(&QueryRecoveryTpls {
            tpl_code: Some("NONEXIST".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(empty.is_empty());
}

#[tokio::test]
async fn recovery_tpl_empty_lines_rejected() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    let source = seed_material(&pool, "RCV-EMPTY", "FINISHED", false).await;
    let svc = RecoveryTplService::new(pool);

    let err = svc
        .create(
            &ctx,
            CreateRecoveryTplCommand {
                tpl_code: "TPL-EMPTY".into(),
                tpl_name: "空行".into(),
                source_material_id: source,
                remark: None,
                lines: vec![],
            },
        )
        .await
        .expect_err("empty lines should fail");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("不能为空") || msg.contains("22106"),
        "expected empty error: {msg}"
    );
}

// ============================================================================
// 6. Customer Return Judge → wms_customer_return_judge
// ============================================================================

#[tokio::test]
async fn customer_return_judge_inserts_judge_record() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    let cust = seed_customer(&pool, "JUDGE-CUST").await;
    let mat = seed_material(&pool, "JUDGE-MAT", "FINISHED", false).await;
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
                original_doc_no: None,
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
                        material_id: mat,
                        batch_no: "JDG-B1".into(),
                        qty: dec("5"),
                        unit: "PCS".into(),
                        return_reason: "质量OK".into(),
                        judge_method: None,
                        judge_note: None,
                        note: None,
                    },
                    CreateCustomerReturnLine {
                        line_no: 2,
                        material_id: mat,
                        batch_no: "JDG-B2".into(),
                        qty: dec("3"),
                        unit: "PCS".into(),
                        return_reason: "不良品".into(),
                        judge_method: None,
                        judge_note: None,
                        note: None,
                    },
                ],
            },
        )
        .await
        .unwrap();

    let line1_id = head.lines[0].id;
    let line2_id = head.lines[1].id;

    // Judge: line1 → RETURN_TO_STOCK, line2 → TO_SCRAP
    svc.judge(
        &ctx,
        head.id,
        vec![
            JudgeLineCommand {
                line_id: line1_id,
                judge_method: "RETURN_TO_STOCK".into(),
                judge_note: Some("合格退回".into()),
            },
            JudgeLineCommand {
                line_id: line2_id,
                judge_method: "TO_SCRAP".into(),
                judge_note: Some("报废处理".into()),
            },
        ],
    )
    .await
    .expect("judge");

    // Verify wms_customer_return_judge records were inserted
    let judge_count: i64 = sqlx::query_scalar(
        "select count(*) from wms.wms_customer_return_judge where customer_return_id = $1",
    )
    .bind(head.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(judge_count, 2, "should have 2 judge records");

    // Verify line1: RETURN_TO_STOCK → QUALIFIED, qty=5
    let (result1, qty1): (String, Decimal) = sqlx::query_as(
        r#"select judge_result, judge_qty
             from wms.wms_customer_return_judge
            where customer_return_line_id = $1"#,
    )
    .bind(line1_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(result1, "QUALIFIED");
    assert_eq!(qty1, dec("5"));

    // Verify line2: TO_SCRAP → SCRAPPED, qty=3
    let (result2, qty2): (String, Decimal) = sqlx::query_as(
        r#"select judge_result, judge_qty
             from wms.wms_customer_return_judge
            where customer_return_line_id = $1"#,
    )
    .bind(line2_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(result2, "SCRAPPED");
    assert_eq!(qty2, dec("3"));

    // Verify judge_user_id is set
    let user_id: Option<i64> = sqlx::query_scalar(
        r#"select judge_user_id from wms.wms_customer_return_judge
            where customer_return_line_id = $1"#,
    )
    .bind(line1_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(user_id, Some(ctx.user_id));
}

#[tokio::test]
async fn customer_return_judge_to_defect_maps_to_bad() {
    let db = TestDb::new().await;
    let pool = db.pool_owned();
    let ctx = admin_ctx();

    let cust = seed_customer(&pool, "JUDGE-CUST2").await;
    let mat = seed_material(&pool, "JUDGE-MAT2", "FINISHED", false).await;
    let (raw_wh, raw_loc) = default_raw_wh_loc(&pool).await;
    let (bad_wh, bad_loc) = default_bad_wh_loc(&pool).await;

    let svc = CustomerReturnService::new(pool.clone());
    let head = svc
        .create(
            &ctx,
            CreateCustomerReturnCommand {
                customer_id: cust,
                return_date: today(),
                original_doc_no: None,
                remark: None,
                return_wh_id: raw_wh,
                return_loc_id: raw_loc,
                defect_wh_id: Some(bad_wh),
                defect_loc_id: Some(bad_loc),
                scrap_wh_id: None,
                scrap_loc_id: None,
                lines: vec![CreateCustomerReturnLine {
                    line_no: 1,
                    material_id: mat,
                    batch_no: "JDG-DEF".into(),
                    qty: dec("7"),
                    unit: "PCS".into(),
                    return_reason: "不良".into(),
                    judge_method: None,
                    judge_note: None,
                    note: None,
                }],
            },
        )
        .await
        .unwrap();

    svc.judge(
        &ctx,
        head.id,
        vec![JudgeLineCommand {
            line_id: head.lines[0].id,
            judge_method: "TO_DEFECT".into(),
            judge_note: None,
        }],
    )
    .await
    .unwrap();

    // TO_DEFECT → BAD
    let result: String = sqlx::query_scalar(
        r#"select judge_result from wms.wms_customer_return_judge
            where customer_return_line_id = $1"#,
    )
    .bind(head.lines[0].id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(result, "BAD");
}
