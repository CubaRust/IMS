//! 具体 job 实现
//!
//! 每个 job 返回 `anyhow::Result<serde_json::Value>`,JSON 落到 `result_json` 里

use serde_json::json;
use sqlx::PgPool;

/// preissue 超期扫描
///
/// 策略:`wms_preissue_h` 里
/// - `exception_status in ('PENDING', 'PARTIAL')` 还没闭环
/// - `expected_close_date is not null`
/// - `expected_close_date < current_date`(已逾期)
/// - `timeout_flag = false`(还没标记过)
///
/// 一次性标记,不做循环通知;后续可以把 `timeout_flag=true` 条件暴露给前端做高亮。
pub async fn preissue_timeout_scan(pool: PgPool) -> anyhow::Result<serde_json::Value> {
    let affected = sqlx::query(
        r#"
        update wms.wms_preissue_h
           set timeout_flag = true,
               timeout_at   = now()
         where exception_status in ('PENDING', 'PARTIAL')
           and expected_close_date is not null
           and expected_close_date < current_date
           and timeout_flag = false
        "#,
    )
    .execute(&pool)
    .await?
    .rows_affected();

    Ok(json!({ "flagged": affected }))
}

/// 呆滞物料视图刷新
///
/// 当前 `rpt.v_stock_dormant` 是普通视图,不需要刷新。
/// 留作入口,等视图升级为 materialized view 时改实现。
pub async fn dormant_refresh(pool: PgPool) -> anyhow::Result<serde_json::Value> {
    // 如果 dormant 是物化视图,用 refresh materialized view concurrently
    let refreshed: Option<bool> = sqlx::query_scalar(
        r#"
        select case
          when exists (
            select 1 from pg_matviews
            where schemaname = 'rpt' and matviewname = 'v_stock_dormant'
          )
          then true else false
        end
        "#,
    )
    .fetch_one(&pool)
    .await?;

    if refreshed == Some(true) {
        sqlx::query("refresh materialized view rpt.v_stock_dormant")
            .execute(&pool)
            .await?;
        Ok(json!({ "refreshed": true }))
    } else {
        Ok(json!({ "refreshed": false, "reason": "not a matview" }))
    }
}

/// 审计日志归档:把 N 天前的数据从 `sys.sys_audit_log` 搬到 `sys.sys_audit_log_archive`
pub async fn audit_log_archive(pool: PgPool, days: i64) -> anyhow::Result<serde_json::Value> {
    let mut tx = pool.begin().await?;

    let moved: i64 = sqlx::query_scalar::<_, i32>(
        r#"
        with moved as (
            delete from sys.sys_audit_log
             where created_at < now() - ($1 || ' days')::interval
             returning *
        )
        insert into sys.sys_audit_log_archive
        select * from moved
        returning 1
        "#,
    )
    .bind(days)
    .fetch_all(&mut *tx)
    .await
    .map(|r| r.len() as i64)
    .unwrap_or(0);

    tx.commit().await?;
    Ok(json!({ "archived": moved, "cutoff_days": days }))
}

/// JWT 黑名单清理:删除过期的 jti
pub async fn jwt_revocation_cleanup(pool: PgPool) -> anyhow::Result<serde_json::Value> {
    let affected = sqlx::query(
        r#"
        delete from sys.sys_jwt_revocation
         where expires_at < now() - interval '1 day'
        "#,
    )
    .execute(&pool)
    .await?
    .rows_affected();

    Ok(json!({ "deleted": affected }))
}
