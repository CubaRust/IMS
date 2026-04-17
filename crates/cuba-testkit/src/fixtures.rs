//! 测试 fixtures:常用数据构造

use rust_decimal::Decimal;
use sqlx::PgPool;
use time::Date;

use cuba_shared::audit::AuditContext;

/// 系统管理员上下文(权限全开,用于绕过权限校验)
#[must_use]
pub fn admin_ctx() -> AuditContext {
    let mut ctx = AuditContext::system("test-trace");
    ctx.user_id = 1;
    ctx.login_name = "admin".into();
    ctx
}

/// 构造一个物料,返回 id
pub async fn seed_material(
    pool: &PgPool,
    code: &str,
    category: &str,
    allow_preissue: bool,
) -> i64 {
    sqlx::query_scalar(
        r#"
        insert into mdm.mdm_material
            (material_code, material_name, material_category, unit,
             batch_required_flag, status_required_flag, allow_preissue_flag)
        values ($1, $1, $2, 'PCS', true, true, $3)
        on conflict (material_code) do update set updated_at = now()
        returning id
        "#,
    )
    .bind(code)
    .bind(category)
    .bind(allow_preissue)
    .fetch_one(pool)
    .await
    .expect("seed_material")
}

/// 构造一个供应商
pub async fn seed_supplier(pool: &PgPool, code: &str) -> i64 {
    sqlx::query_scalar(
        r#"
        insert into mdm.mdm_supplier (supplier_code, supplier_name)
        values ($1, $1)
        on conflict (supplier_code) do update set updated_at = now()
        returning id
        "#,
    )
    .bind(code)
    .fetch_one(pool)
    .await
    .expect("seed_supplier")
}

/// 构造一个客户
pub async fn seed_customer(pool: &PgPool, code: &str) -> i64 {
    sqlx::query_scalar(
        r#"
        insert into mdm.mdm_customer (customer_code, customer_name)
        values ($1, $1)
        on conflict (customer_code) do update set updated_at = now()
        returning id
        "#,
    )
    .bind(code)
    .fetch_one(pool)
    .await
    .expect("seed_customer")
}

/// 返回 0010 seed 里 RAW01 仓库的 id 和第一个 NORMAL 仓位的 id
pub async fn default_raw_wh_loc(pool: &PgPool) -> (i64, i64) {
    let wh_id: i64 = sqlx::query_scalar(
        "select id from mdm.mdm_warehouse where wh_code = 'RAW01'",
    )
    .fetch_one(pool)
    .await
    .expect("RAW01 warehouse");
    let loc_id: i64 = sqlx::query_scalar(
        "select id from mdm.mdm_location where wh_id = $1 order by id limit 1",
    )
    .bind(wh_id)
    .fetch_one(pool)
    .await
    .expect("RAW01 location");
    (wh_id, loc_id)
}

/// 返回 BAD01 仓/第一个仓位
pub async fn default_bad_wh_loc(pool: &PgPool) -> (i64, i64) {
    let wh_id: i64 = sqlx::query_scalar(
        "select id from mdm.mdm_warehouse where wh_code = 'BAD01'",
    )
    .fetch_one(pool)
    .await
    .expect("BAD01 warehouse");
    let loc_id: i64 = sqlx::query_scalar(
        "select id from mdm.mdm_location where wh_id = $1 order by id limit 1",
    )
    .bind(wh_id)
    .fetch_one(pool)
    .await
    .expect("BAD01 location");
    (wh_id, loc_id)
}

/// 返回 SCRAP01 仓/第一个仓位
pub async fn default_scrap_wh_loc(pool: &PgPool) -> (i64, i64) {
    let wh_id: i64 = sqlx::query_scalar(
        "select id from mdm.mdm_warehouse where wh_code = 'SCRAP01'",
    )
    .fetch_one(pool)
    .await
    .expect("SCRAP01 warehouse");
    let loc_id: i64 = sqlx::query_scalar(
        "select id from mdm.mdm_location where wh_id = $1 order by id limit 1",
    )
    .bind(wh_id)
    .fetch_one(pool)
    .await
    .expect("SCRAP01 location");
    (wh_id, loc_id)
}

/// 今天
#[must_use]
pub fn today() -> Date {
    time::OffsetDateTime::now_utc().date()
}

/// 构造 Decimal 方便写
#[must_use]
pub fn dec(s: &str) -> Decimal {
    s.parse().expect("decimal parse")
}
