//! 状态流转规则(目前只读,seed 已铺好默认规则)

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::PrimitiveDateTime;

use cuba_shared::error::AppError;

use crate::infrastructure::status_flow_repo::{PgStatusFlowRepository, StatusFlowRepository};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusFlowView {
    pub id: i64,
    pub source_status: String,
    pub target_status: String,
    pub scene_code: String,
    pub need_auth_flag: bool,
    pub is_active: bool,
    pub remark: Option<String>,
    pub created_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryStatusFlow {
    pub source_status: Option<String>,
    pub scene_code: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Clone)]
pub struct StatusFlowService {
    repo: Arc<dyn StatusFlowRepository>,
}

impl StatusFlowService {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: Arc::new(PgStatusFlowRepository::new(pool)),
        }
    }

    pub async fn list(&self, q: &QueryStatusFlow) -> Result<Vec<StatusFlowView>, AppError> {
        self.repo.list(q).await
    }
}
