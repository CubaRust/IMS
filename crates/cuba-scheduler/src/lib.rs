//! cuba-scheduler
//!
//! 异步任务调度器。
//!
//! ## 特性
//! - **单实例执行**:每个 job 用 pg advisory lock 做跨 Pod 互斥,
//!   多副本部署时只有一个实例真正跑。抢不到锁的实例写 `status=SKIPPED`。
//! - **执行记录**:每次跑结果落 `sys.sys_scheduled_job_log`,含耗时/错误信息
//! - **可关闭**:通过 `SCHEDULER_ENABLED=false` 或 builder 配置关闭整个调度器
//!
//! ## 内置 job
//! - `preissue_timeout_scan` — 每小时扫超期未冲销的异常先发,打 `timeout_flag`
//! - `dormant_refresh` — 每日 02:00 刷呆滞物料视图(目前用普通视图不需刷,预留)
//! - `audit_log_archive` — 每月 1 日 03:00 把 90 天前审计日志搬到归档表
//! - `jwt_revocation_cleanup` — 每日 04:00 清理过期 jti 黑名单

#![deny(unsafe_code)]

pub mod jobs;
pub mod runtime;

pub use runtime::{start, SchedulerConfig, SchedulerHandle};
