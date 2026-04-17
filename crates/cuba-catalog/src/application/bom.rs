//! BOM 头 + 行
//!
//! 头 1 : 多行,插入时在同一事务里写。

use std::sync::Arc;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::PrimitiveDateTime;
use validator::Validate;

use cuba_shared::{audit::AuditContext, error::AppError};

use crate::domain::CatalogError;
use crate::infrastructure::bom_repo::{BomRepository, PgBomRepository};

// -- View --------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomHeadView {
    pub id: i64,
    pub bom_code: String,
    pub bom_version: String,
    pub product_material_id: i64,
    pub product_material_code: Option<String>,
    pub route_id: Option<i64>,
    pub is_active: bool,
    pub remark: Option<String>,
    pub lines: Vec<BomLineView>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomLineView {
    pub id: i64,
    pub line_no: i32,
    pub material_id: i64,
    pub material_code: Option<String>,
    pub usage_qty: Decimal,
    pub loss_rate: Decimal,
    pub public_material_flag: bool,
    pub remark: Option<String>,
}

// -- Commands ----------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateBomCommand {
    #[validate(length(min = 1, max = 50))]
    pub bom_code: String,
    #[validate(length(min = 1, max = 30))]
    pub bom_version: String,
    pub product_material_id: i64,
    #[serde(default)]
    pub route_id: Option<i64>,
    #[serde(default)]
    pub remark: Option<String>,
    pub lines: Vec<CreateBomLine>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateBomLine {
    pub line_no: i32,
    pub material_id: i64,
    pub usage_qty: Decimal,
    #[serde(default)]
    pub loss_rate: Decimal,
    #[serde(default)]
    pub public_material_flag: bool,
    #[serde(default)]
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryBoms {
    pub bom_code: Option<String>,
    pub product_material_id: Option<i64>,
    pub is_active: Option<bool>,
}

// -- Service -----------------------------------------------------------------

#[derive(Clone)]
pub struct BomService {
    repo: Arc<dyn BomRepository>,
}

impl BomService {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { repo: Arc::new(PgBomRepository::new(pool)) }
    }

    pub async fn create(
        &self,
        _ctx: &AuditContext,
        cmd: CreateBomCommand,
    ) -> Result<BomHeadView, AppError> {
        cmd.validate().map_err(|e| AppError::validation(e.to_string()))?;
        if cmd.lines.is_empty() {
            return Err(CatalogError::bom_empty());
        }
        for l in &cmd.lines {
            l.validate().map_err(|e| AppError::validation(e.to_string()))?;
        }
        self.repo.create(&cmd).await
    }

    pub async fn get(&self, id: i64) -> Result<BomHeadView, AppError> {
        self.repo.get(id).await
    }

    pub async fn list(&self, q: &QueryBoms) -> Result<Vec<BomHeadView>, AppError> {
        self.repo.list(q).await
    }
}
