//! 回收拆解模板 service (head + detail)

use std::sync::Arc;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::PrimitiveDateTime;
use validator::Validate;

use cuba_shared::{audit::AuditContext, error::AppError};

use crate::domain::CatalogError;
use crate::infrastructure::recovery_tpl_repo::{PgRecoveryTplRepository, RecoveryTplRepository};

// -- View --------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryTplHeadView {
    pub id: i64,
    pub tpl_code: String,
    pub tpl_name: String,
    pub source_material_id: i64,
    pub source_material_code: Option<String>,
    pub is_active: bool,
    pub remark: Option<String>,
    pub lines: Vec<RecoveryTplLineView>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryTplLineView {
    pub id: i64,
    pub line_no: i32,
    pub target_material_id: Option<i64>,
    pub target_material_code: Option<String>,
    pub default_recovery_qty: Decimal,
    pub target_default_status: Option<String>,
    pub scrap_flag: bool,
    pub remark: Option<String>,
}

// -- Commands ----------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateRecoveryTplCommand {
    #[validate(length(min = 1, max = 50))]
    pub tpl_code: String,
    #[validate(length(min = 1, max = 200))]
    pub tpl_name: String,
    pub source_material_id: i64,
    #[serde(default)]
    pub remark: Option<String>,
    pub lines: Vec<CreateRecoveryTplLine>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateRecoveryTplLine {
    pub line_no: i32,
    pub target_material_id: Option<i64>,
    pub default_recovery_qty: Decimal,
    #[serde(default)]
    pub target_default_status: Option<String>,
    #[serde(default)]
    pub scrap_flag: bool,
    #[serde(default)]
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryRecoveryTpls {
    pub tpl_code: Option<String>,
    pub source_material_id: Option<i64>,
    pub is_active: Option<bool>,
}

// -- Service -----------------------------------------------------------------

#[derive(Clone)]
pub struct RecoveryTplService {
    repo: Arc<dyn RecoveryTplRepository>,
}

impl RecoveryTplService {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: Arc::new(PgRecoveryTplRepository::new(pool)),
        }
    }

    pub async fn create(
        &self,
        _ctx: &AuditContext,
        cmd: CreateRecoveryTplCommand,
    ) -> Result<RecoveryTplHeadView, AppError> {
        cmd.validate()
            .map_err(|e| AppError::validation(e.to_string()))?;
        if cmd.lines.is_empty() {
            return Err(CatalogError::recovery_tpl_empty());
        }
        for l in &cmd.lines {
            l.validate()
                .map_err(|e| AppError::validation(e.to_string()))?;
        }
        self.repo.create(&cmd).await
    }

    pub async fn get(&self, id: i64) -> Result<RecoveryTplHeadView, AppError> {
        self.repo.get(id).await
    }

    pub async fn list(&self, q: &QueryRecoveryTpls) -> Result<Vec<RecoveryTplHeadView>, AppError> {
        self.repo.list(q).await
    }
}
