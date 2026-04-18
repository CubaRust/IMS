//! Scheduler 运行时:启动、advisory lock、执行记录

use sqlx::PgPool;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use time::OffsetDateTime;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info, warn};

use crate::jobs;

#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub enabled: bool,
    /// 每个 job 的 cron 表达式(7 字段含秒;UTC)
    pub preissue_timeout_cron: String,
    pub dormant_refresh_cron: String,
    pub audit_archive_cron: String,
    pub jwt_cleanup_cron: String,
    /// 归档 N 天前的审计数据
    pub audit_archive_days: i64,
    /// 主机标识(写 log 用)
    pub host: String,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            // 每小时第 0 分 0 秒
            preissue_timeout_cron: "0 0 * * * *".into(),
            // 每日 02:00
            dormant_refresh_cron: "0 0 2 * * *".into(),
            // 每月 1 日 03:00
            audit_archive_cron: "0 0 3 1 * *".into(),
            // 每日 04:00
            jwt_cleanup_cron: "0 0 4 * * *".into(),
            audit_archive_days: 90,
            host: hostname(),
        }
    }
}

fn hostname() -> String {
    std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".into())
}

pub struct SchedulerHandle {
    inner: JobScheduler,
}

impl SchedulerHandle {
    pub async fn shutdown(mut self) {
        if let Err(e) = self.inner.shutdown().await {
            warn!(error=%e, "scheduler shutdown failed");
        }
    }
}

/// 启动调度器(幂等,重复调用会多起一份,调用方负责只跑一次)
pub async fn start(pool: PgPool, cfg: SchedulerConfig) -> anyhow::Result<Option<SchedulerHandle>> {
    if !cfg.enabled {
        info!("scheduler disabled by config");
        return Ok(None);
    }

    let sched = JobScheduler::new().await?;
    let cfg = Arc::new(cfg);

    // preissue 超期扫描
    {
        let pool = pool.clone();
        let cfg = cfg.clone();
        let expr = cfg.preissue_timeout_cron.clone();
        sched
            .add(Job::new_async(expr.as_str(), move |_uuid, _l| {
                let pool = pool.clone();
                let cfg = cfg.clone();
                Box::pin(async move {
                    run_job("preissue_timeout_scan", 10_001, &pool, &cfg, |p| {
                        Box::pin(jobs::preissue_timeout_scan(p))
                    })
                    .await;
                })
            })?)
            .await?;
    }

    // dormant 视图刷新
    {
        let pool = pool.clone();
        let cfg = cfg.clone();
        let expr = cfg.dormant_refresh_cron.clone();
        sched
            .add(Job::new_async(expr.as_str(), move |_uuid, _l| {
                let pool = pool.clone();
                let cfg = cfg.clone();
                Box::pin(async move {
                    run_job("dormant_refresh", 10_002, &pool, &cfg, |p| {
                        Box::pin(jobs::dormant_refresh(p))
                    })
                    .await;
                })
            })?)
            .await?;
    }

    // 审计归档
    {
        let pool = pool.clone();
        let cfg = cfg.clone();
        let expr = cfg.audit_archive_cron.clone();
        let days = cfg.audit_archive_days;
        sched
            .add(Job::new_async(expr.as_str(), move |_uuid, _l| {
                let pool = pool.clone();
                let cfg = cfg.clone();
                Box::pin(async move {
                    run_job("audit_log_archive", 10_003, &pool, &cfg, move |p| {
                        Box::pin(jobs::audit_log_archive(p, days))
                    })
                    .await;
                })
            })?)
            .await?;
    }

    // JWT 黑名单清理
    {
        let pool = pool.clone();
        let cfg = cfg.clone();
        let expr = cfg.jwt_cleanup_cron.clone();
        sched
            .add(Job::new_async(expr.as_str(), move |_uuid, _l| {
                let pool = pool.clone();
                let cfg = cfg.clone();
                Box::pin(async move {
                    run_job("jwt_revocation_cleanup", 10_004, &pool, &cfg, |p| {
                        Box::pin(jobs::jwt_revocation_cleanup(p))
                    })
                    .await;
                })
            })?)
            .await?;
    }

    sched.start().await?;
    info!("scheduler started");
    Ok(Some(SchedulerHandle { inner: sched }))
}

/// 执行一个 job 的包装:拿 advisory lock → 写 RUNNING 日志 → 跑 fn → 写结果
///
/// `lock_key` 在本库全局唯一(见各 job 常量),保证多实例互斥
async fn run_job<F>(
    name: &str,
    lock_key: i64,
    pool: &PgPool,
    cfg: &SchedulerConfig,
    f: F,
) where
    F: FnOnce(PgPool) -> Pin<Box<dyn Future<Output = anyhow::Result<serde_json::Value>> + Send>>,
{
    let host = cfg.host.clone();

    // 拿 session-level advisory lock(非阻塞)
    let got: (bool,) = match sqlx::query_as("select pg_try_advisory_lock($1)")
        .bind(lock_key)
        .fetch_one(pool)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            error!(job=name, error=%e, "advisory lock query failed");
            return;
        }
    };

    if !got.0 {
        // 别的实例正在跑,记一行 SKIPPED
        let _ = sqlx::query(
            r#"insert into sys.sys_scheduled_job_log
               (job_name, status, host, finished_at, duration_ms)
               values ($1, 'SKIPPED', $2, now(), 0)"#,
        )
        .bind(name)
        .bind(&host)
        .execute(pool)
        .await;
        return;
    }

    // 写 RUNNING
    let start = std::time::Instant::now();
    let log_id: Result<i64, _> = sqlx::query_scalar(
        r#"insert into sys.sys_scheduled_job_log (job_name, status, host)
           values ($1, 'RUNNING', $2) returning id"#,
    )
    .bind(name)
    .bind(&host)
    .fetch_one(pool)
    .await;

    let log_id = match log_id {
        Ok(id) => id,
        Err(e) => {
            error!(job=name, error=%e, "write RUNNING log failed");
            let _: Result<(bool,), _> = sqlx::query_as("select pg_advisory_unlock($1)")
                .bind(lock_key)
                .fetch_one(pool)
                .await;
            return;
        }
    };

    // 跑
    let result = f(pool.clone()).await;
    let elapsed_ms = start.elapsed().as_millis() as i32;
    let ok = result.is_ok();

    match result {
        Ok(json) => {
            let _ = sqlx::query(
                r#"update sys.sys_scheduled_job_log
                   set status='OK', finished_at=$1, duration_ms=$2, result_json=$3
                   where id=$4"#,
            )
            .bind(now_ts())
            .bind(elapsed_ms)
            .bind(&json)
            .bind(log_id)
            .execute(pool)
            .await;
            info!(job = name, duration_ms = elapsed_ms, "job ok");
        }
        Err(e) => {
            let msg = format!("{e:?}");
            let _ = sqlx::query(
                r#"update sys.sys_scheduled_job_log
                   set status='FAIL', finished_at=$1, duration_ms=$2, error_message=$3
                   where id=$4"#,
            )
            .bind(now_ts())
            .bind(elapsed_ms)
            .bind(&msg)
            .bind(log_id)
            .execute(pool)
            .await;
            error!(job = name, error = %e, "job failed");
        }
    }

    // metrics
    cuba_metrics::record_txn(name, "SCHEDULER", ok);

    // 释放锁
    let _: Result<(bool,), _> = sqlx::query_as("select pg_advisory_unlock($1)")
        .bind(lock_key)
        .fetch_one(pool)
        .await;
}

fn now_ts() -> time::PrimitiveDateTime {
    let n = OffsetDateTime::now_utc();
    time::PrimitiveDateTime::new(n.date(), n.time())
}
