//! 物料主数据

use std::sync::Arc;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::PrimitiveDateTime;
use validator::Validate;

use cuba_shared::{audit::AuditContext, error::AppError};

use crate::domain::{is_valid_category, is_valid_process_type, CatalogError};
use crate::infrastructure::material_repo::{MaterialRepository, PgMaterialRepository};

// -- View --------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialView {
    pub id: i64,
    pub material_code: String,
    pub material_name: String,
    pub short_name: Option<String>,
    pub material_category: String,
    pub spec_model: Option<String>,
    pub brand: Option<String>,
    pub unit: String,
    pub process_type: Option<String>,
    pub has_ic_flag: bool,
    pub key_material_flag: bool,
    pub public_material_flag: bool,
    pub batch_required_flag: bool,
    pub status_required_flag: bool,
    pub allow_preissue_flag: bool,
    pub allow_outsource_flag: bool,
    pub allow_recovery_flag: bool,
    pub default_wh_id: Option<i64>,
    pub default_loc_id: Option<i64>,
    pub default_status: Option<String>,
    pub safety_stock: Decimal,
    pub min_stock: Decimal,
    pub extra_attrs: serde_json::Value,
    pub is_active: bool,
    pub remark: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

// -- Commands ----------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateMaterialCommand {
    #[validate(length(min = 1, max = 100))]
    pub material_code: String,
    #[validate(length(min = 1, max = 200))]
    pub material_name: String,
    #[serde(default)]
    pub short_name: Option<String>,
    pub material_category: String,
    #[serde(default)]
    pub spec_model: Option<String>,
    #[serde(default)]
    pub brand: Option<String>,
    #[validate(length(min = 1, max = 30))]
    pub unit: String,
    #[serde(default)]
    pub process_type: Option<String>,
    #[serde(default)]
    pub has_ic_flag: bool,
    #[serde(default)]
    pub key_material_flag: bool,
    #[serde(default)]
    pub public_material_flag: bool,
    #[serde(default = "default_true")]
    pub batch_required_flag: bool,
    #[serde(default = "default_true")]
    pub status_required_flag: bool,
    #[serde(default)]
    pub allow_preissue_flag: bool,
    #[serde(default)]
    pub allow_outsource_flag: bool,
    #[serde(default)]
    pub allow_recovery_flag: bool,
    #[serde(default)]
    pub default_wh_id: Option<i64>,
    #[serde(default)]
    pub default_loc_id: Option<i64>,
    #[serde(default)]
    pub default_status: Option<String>,
    #[serde(default)]
    pub safety_stock: Decimal,
    #[serde(default)]
    pub min_stock: Decimal,
    #[serde(default)]
    pub extra_attrs: Option<serde_json::Value>,
    #[serde(default)]
    pub remark: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateMaterialCommand {
    #[validate(length(min = 1, max = 200))]
    pub material_name: String,
    #[serde(default)]
    pub short_name: Option<String>,
    pub material_category: String,
    #[serde(default)]
    pub spec_model: Option<String>,
    #[serde(default)]
    pub brand: Option<String>,
    #[validate(length(min = 1, max = 30))]
    pub unit: String,
    #[serde(default)]
    pub process_type: Option<String>,
    pub has_ic_flag: bool,
    pub key_material_flag: bool,
    pub public_material_flag: bool,
    pub batch_required_flag: bool,
    pub status_required_flag: bool,
    pub allow_preissue_flag: bool,
    pub allow_outsource_flag: bool,
    pub allow_recovery_flag: bool,
    #[serde(default)]
    pub default_wh_id: Option<i64>,
    #[serde(default)]
    pub default_loc_id: Option<i64>,
    #[serde(default)]
    pub default_status: Option<String>,
    pub safety_stock: Decimal,
    pub min_stock: Decimal,
    #[serde(default)]
    pub extra_attrs: Option<serde_json::Value>,
    pub is_active: bool,
    #[serde(default)]
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryMaterials {
    pub keyword: Option<String>,
    pub material_category: Option<String>,
    pub process_type: Option<String>,
    pub brand: Option<String>,
    pub is_active: Option<bool>,
    pub page: u32,
    pub size: u32,
}

// -- Service -----------------------------------------------------------------

#[derive(Clone)]
pub struct MaterialService {
    repo: Arc<dyn MaterialRepository>,
}

impl MaterialService {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { repo: Arc::new(PgMaterialRepository::new(pool)) }
    }

    #[must_use]
    pub fn with_repo(repo: Arc<dyn MaterialRepository>) -> Self {
        Self { repo }
    }

    pub async fn create(
        &self,
        _ctx: &AuditContext,
        cmd: CreateMaterialCommand,
    ) -> Result<MaterialView, AppError> {
        cmd.validate().map_err(|e| AppError::validation(e.to_string()))?;
        if !is_valid_category(&cmd.material_category) {
            return Err(CatalogError::invalid_category(&cmd.material_category));
        }
        if let Some(ref pt) = cmd.process_type {
            if !is_valid_process_type(pt) {
                return Err(CatalogError::invalid_process_type(pt));
            }
        }
        self.repo.create(&cmd).await
    }

    pub async fn update(
        &self,
        _ctx: &AuditContext,
        id: i64,
        cmd: UpdateMaterialCommand,
    ) -> Result<MaterialView, AppError> {
        cmd.validate().map_err(|e| AppError::validation(e.to_string()))?;
        if !is_valid_category(&cmd.material_category) {
            return Err(CatalogError::invalid_category(&cmd.material_category));
        }
        if let Some(ref pt) = cmd.process_type {
            if !is_valid_process_type(pt) {
                return Err(CatalogError::invalid_process_type(pt));
            }
        }
        self.repo.update(id, &cmd).await
    }

    pub async fn get(&self, id: i64) -> Result<MaterialView, AppError> {
        self.repo.get(id).await
    }

    pub async fn list(
        &self,
        q: &QueryMaterials,
    ) -> Result<cuba_shared::pagination::PageResponse<MaterialView>, AppError> {
        self.repo.list(q).await
    }
}
