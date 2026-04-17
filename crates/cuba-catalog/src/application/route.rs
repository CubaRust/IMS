//! 工艺路线头 + 步骤

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::PrimitiveDateTime;
use validator::Validate;

use cuba_shared::{audit::AuditContext, error::AppError};

use crate::domain::CatalogError;
use crate::infrastructure::route_repo::{PgRouteRepository, RouteRepository};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteHeadView {
    pub id: i64,
    pub route_code: String,
    pub route_name: String,
    pub product_material_id: i64,
    pub product_material_code: Option<String>,
    pub is_active: bool,
    pub remark: Option<String>,
    pub steps: Vec<RouteStepView>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteStepView {
    pub id: i64,
    pub step_no: i32,
    pub process_name: String,
    pub output_material_id: Option<i64>,
    pub output_material_code: Option<String>,
    pub semi_finished_flag: bool,
    pub rule_json: serde_json::Value,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateRouteCommand {
    #[validate(length(min = 1, max = 50))]
    pub route_code: String,
    #[validate(length(min = 1, max = 200))]
    pub route_name: String,
    pub product_material_id: i64,
    #[serde(default)]
    pub remark: Option<String>,
    pub steps: Vec<CreateRouteStep>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateRouteStep {
    pub step_no: i32,
    #[validate(length(min = 1, max = 100))]
    pub process_name: String,
    #[serde(default)]
    pub output_material_id: Option<i64>,
    #[serde(default)]
    pub semi_finished_flag: bool,
    #[serde(default)]
    pub rule_json: Option<serde_json::Value>,
    #[serde(default)]
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryRoutes {
    pub route_code: Option<String>,
    pub product_material_id: Option<i64>,
    pub is_active: Option<bool>,
}

#[derive(Clone)]
pub struct RouteService {
    repo: Arc<dyn RouteRepository>,
}

impl RouteService {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { repo: Arc::new(PgRouteRepository::new(pool)) }
    }

    pub async fn create(
        &self,
        _ctx: &AuditContext,
        cmd: CreateRouteCommand,
    ) -> Result<RouteHeadView, AppError> {
        cmd.validate().map_err(|e| AppError::validation(e.to_string()))?;
        if cmd.steps.is_empty() {
            return Err(CatalogError::route_empty());
        }
        // 步骤号不能重
        let mut seen = std::collections::HashSet::new();
        for s in &cmd.steps {
            s.validate().map_err(|e| AppError::validation(e.to_string()))?;
            if !seen.insert(s.step_no) {
                return Err(CatalogError::duplicate_step(s.step_no));
            }
        }
        self.repo.create(&cmd).await
    }

    pub async fn get(&self, id: i64) -> Result<RouteHeadView, AppError> {
        self.repo.get(id).await
    }

    pub async fn list(&self, q: &QueryRoutes) -> Result<Vec<RouteHeadView>, AppError> {
        self.repo.list(q).await
    }
}
