//! warehouse 应用层(CRUD + 简单查询)

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::PrimitiveDateTime;
use validator::Validate;

use cuba_shared::{audit::AuditContext, error::AppError};

use crate::domain::{is_valid_loc_type, is_valid_wh_type, WarehouseError};
use crate::infrastructure::repository::{PgWarehouseRepository, WarehouseRepository};

// -- DTO ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarehouseView {
    pub id: i64,
    pub wh_code: String,
    pub wh_name: String,
    pub wh_type: String,
    pub is_active: bool,
    pub remark: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationView {
    pub id: i64,
    pub wh_id: i64,
    pub wh_code: String,
    pub loc_code: String,
    pub loc_name: String,
    pub loc_type: String,
    pub is_active: bool,
    pub remark: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

// -- Commands / Queries ------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateWarehouseCommand {
    #[validate(length(min = 1, max = 50))]
    pub wh_code: String,
    #[validate(length(min = 1, max = 100))]
    pub wh_name: String,
    pub wh_type: String,
    #[serde(default)]
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateWarehouseCommand {
    #[validate(length(min = 1, max = 100))]
    pub wh_name: String,
    pub wh_type: String,
    pub is_active: bool,
    #[serde(default)]
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryWarehouses {
    pub wh_code: Option<String>,
    pub wh_type: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateLocationCommand {
    pub wh_id: i64,
    #[validate(length(min = 1, max = 50))]
    pub loc_code: String,
    #[validate(length(min = 1, max = 100))]
    pub loc_name: String,
    #[serde(default = "default_loc_type")]
    pub loc_type: String,
    #[serde(default)]
    pub remark: Option<String>,
}

fn default_loc_type() -> String {
    "NORMAL".into()
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateLocationCommand {
    #[validate(length(min = 1, max = 100))]
    pub loc_name: String,
    pub loc_type: String,
    pub is_active: bool,
    #[serde(default)]
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryLocations {
    pub wh_id: Option<i64>,
    pub loc_type: Option<String>,
    pub is_active: Option<bool>,
}

// -- Service -----------------------------------------------------------------

#[derive(Clone)]
pub struct WarehouseService {
    repo: Arc<dyn WarehouseRepository>,
}

impl WarehouseService {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: Arc::new(PgWarehouseRepository::new(pool)),
        }
    }

    #[must_use]
    pub fn with_repo(repo: Arc<dyn WarehouseRepository>) -> Self {
        Self { repo }
    }

    pub async fn create_warehouse(
        &self,
        _ctx: &AuditContext,
        cmd: CreateWarehouseCommand,
    ) -> Result<WarehouseView, AppError> {
        cmd.validate()
            .map_err(|e| AppError::validation(e.to_string()))?;
        if !is_valid_wh_type(&cmd.wh_type) {
            return Err(WarehouseError::invalid_wh_type(&cmd.wh_type));
        }
        self.repo.create_warehouse(&cmd).await
    }

    pub async fn update_warehouse(
        &self,
        _ctx: &AuditContext,
        id: i64,
        cmd: UpdateWarehouseCommand,
    ) -> Result<WarehouseView, AppError> {
        cmd.validate()
            .map_err(|e| AppError::validation(e.to_string()))?;
        if !is_valid_wh_type(&cmd.wh_type) {
            return Err(WarehouseError::invalid_wh_type(&cmd.wh_type));
        }
        self.repo.update_warehouse(id, &cmd).await
    }

    pub async fn list_warehouses(
        &self,
        q: &QueryWarehouses,
    ) -> Result<Vec<WarehouseView>, AppError> {
        self.repo.list_warehouses(q).await
    }

    pub async fn get_warehouse(&self, id: i64) -> Result<WarehouseView, AppError> {
        self.repo.get_warehouse(id).await
    }

    pub async fn create_location(
        &self,
        _ctx: &AuditContext,
        cmd: CreateLocationCommand,
    ) -> Result<LocationView, AppError> {
        cmd.validate()
            .map_err(|e| AppError::validation(e.to_string()))?;
        if !is_valid_loc_type(&cmd.loc_type) {
            return Err(AppError::validation(format!(
                "未知的仓位类型: {}",
                cmd.loc_type
            )));
        }
        self.repo.create_location(&cmd).await
    }

    pub async fn update_location(
        &self,
        _ctx: &AuditContext,
        id: i64,
        cmd: UpdateLocationCommand,
    ) -> Result<LocationView, AppError> {
        cmd.validate()
            .map_err(|e| AppError::validation(e.to_string()))?;
        if !is_valid_loc_type(&cmd.loc_type) {
            return Err(AppError::validation(format!(
                "未知的仓位类型: {}",
                cmd.loc_type
            )));
        }
        self.repo.update_location(id, &cmd).await
    }

    pub async fn list_locations(&self, q: &QueryLocations) -> Result<Vec<LocationView>, AppError> {
        self.repo.list_locations(q).await
    }

    pub async fn get_location(&self, id: i64) -> Result<LocationView, AppError> {
        self.repo.get_location(id).await
    }
}
