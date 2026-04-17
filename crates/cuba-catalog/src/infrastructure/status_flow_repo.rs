//! status_flow repo(只读)

use async_trait::async_trait;
use sqlx::{postgres::PgRow, PgPool, Postgres, Row};

use cuba_shared::error::AppError;

use crate::application::status_flow::{QueryStatusFlow, StatusFlowView};

#[async_trait]
pub trait StatusFlowRepository: Send + Sync {
    async fn list(&self, q: &QueryStatusFlow) -> Result<Vec<StatusFlowView>, AppError>;
}

pub struct PgStatusFlowRepository {
    pool: PgPool,
}

impl PgStatusFlowRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StatusFlowRepository for PgStatusFlowRepository {
    async fn list(&self, q: &QueryStatusFlow) -> Result<Vec<StatusFlowView>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select id, source_status, target_status, scene_code,
                   need_auth_flag, is_active, remark, created_at
              from mdm.mdm_status_flow
             where 1 = 1
            "#,
        );
        if let Some(s) = &q.source_status {
            qb.push(" and source_status = ").push_bind(s.clone());
        }
        if let Some(s) = &q.scene_code {
            qb.push(" and scene_code = ").push_bind(s.clone());
        }
        if let Some(a) = q.is_active {
            qb.push(" and is_active = ").push_bind(a);
        }
        qb.push(" order by source_status, scene_code limit 500");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_view).collect())
    }
}

fn row_to_view(row: PgRow) -> StatusFlowView {
    StatusFlowView {
        id: row.get("id"),
        source_status: row.get("source_status"),
        target_status: row.get("target_status"),
        scene_code: row.get("scene_code"),
        need_auth_flag: row.get("need_auth_flag"),
        is_active: row.get("is_active"),
        remark: row.get("remark"),
        created_at: row.get("created_at"),
    }
}
