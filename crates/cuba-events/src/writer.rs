//! 事件落库

use sqlx::{PgExecutor, PgPool, Postgres, Transaction};

use cuba_shared::{audit::AuditContext, error::AppError};

use crate::types::DomainEvent;

/// 写入事件所需的最小上下文
///
/// 从 `AuditContext` 派生。事件表不存完整 ctx,只存 tenant + actor + trace_id。
pub struct WriteEventCtx<'a> {
    pub tenant_id: i64,
    pub actor_id: Option<i64>,
    pub actor_name: Option<&'a str>,
    pub trace_id: Option<&'a str>,
}

impl<'a> From<&'a AuditContext> for WriteEventCtx<'a> {
    fn from(c: &'a AuditContext) -> Self {
        Self {
            tenant_id: c.tenant_id,
            actor_id: Some(c.user_id),
            actor_name: Some(&c.login_name),
            trace_id: Some(&c.trace_id),
        }
    }
}

/// 直接用 pool 写事件(不建议业务使用 — 会游离于业务事务外)
pub async fn write_event(
    pool: &PgPool,
    ctx: &WriteEventCtx<'_>,
    event: &DomainEvent,
) -> Result<(), AppError> {
    insert(pool, ctx, event).await
}

/// 在已有业务事务中写事件(**推荐**)
pub async fn write_event_tx(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &WriteEventCtx<'_>,
    event: &DomainEvent,
) -> Result<(), AppError> {
    insert(&mut **tx, ctx, event).await
}

async fn insert<'c, E>(
    exec: E,
    ctx: &WriteEventCtx<'_>,
    event: &DomainEvent,
) -> Result<(), AppError>
where
    E: PgExecutor<'c>,
{
    let payload = serde_json::to_value(event).map_err(|e| {
        AppError::Internal(anyhow::anyhow!("event serialize failed: {e}"))
    })?;

    sqlx::query(
        r#"
        insert into events.domain_event
            (tenant_id, aggregate_type, aggregate_id, event_type,
             event_version, payload, trace_id, actor_id, actor_name)
        values ($1, $2, $3, $4, 1, $5, $6, $7, $8)
        "#,
    )
    .bind(ctx.tenant_id)
    .bind(event.aggregate_type())
    .bind(event.aggregate_id())
    .bind(event.event_type())
    .bind(&payload)
    .bind(ctx.trace_id)
    .bind(ctx.actor_id)
    .bind(ctx.actor_name)
    .execute(exec)
    .await?;

    Ok(())
}
