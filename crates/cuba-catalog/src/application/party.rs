//! 供应商 + 客户
//!
//! 两个实体字段近乎相同,用同一个 `PartyService` 避免重复

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::PrimitiveDateTime;
use validator::Validate;

use cuba_shared::{audit::AuditContext, error::AppError};

use crate::infrastructure::party_repo::{PartyRepository, PgPartyRepository};

// -- View --------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplierView {
    pub id: i64,
    pub supplier_code: String,
    pub supplier_name: String,
    pub contact_name: Option<String>,
    pub contact_phone: Option<String>,
    pub address: Option<String>,
    pub is_active: bool,
    pub remark: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerView {
    pub id: i64,
    pub customer_code: String,
    pub customer_name: String,
    pub contact_name: Option<String>,
    pub contact_phone: Option<String>,
    pub address: Option<String>,
    pub is_active: bool,
    pub remark: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

// -- Commands ----------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateSupplierCommand {
    #[validate(length(min = 1, max = 50))]
    pub supplier_code: String,
    #[validate(length(min = 1, max = 200))]
    pub supplier_name: String,
    #[serde(default)]
    pub contact_name: Option<String>,
    #[serde(default)]
    pub contact_phone: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateSupplierCommand {
    #[validate(length(min = 1, max = 200))]
    pub supplier_name: String,
    #[serde(default)]
    pub contact_name: Option<String>,
    #[serde(default)]
    pub contact_phone: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    pub is_active: bool,
    #[serde(default)]
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QuerySuppliers {
    pub keyword: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateCustomerCommand {
    #[validate(length(min = 1, max = 50))]
    pub customer_code: String,
    #[validate(length(min = 1, max = 200))]
    pub customer_name: String,
    #[serde(default)]
    pub contact_name: Option<String>,
    #[serde(default)]
    pub contact_phone: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateCustomerCommand {
    #[validate(length(min = 1, max = 200))]
    pub customer_name: String,
    #[serde(default)]
    pub contact_name: Option<String>,
    #[serde(default)]
    pub contact_phone: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    pub is_active: bool,
    #[serde(default)]
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryCustomers {
    pub keyword: Option<String>,
    pub is_active: Option<bool>,
}

// -- Service -----------------------------------------------------------------

#[derive(Clone)]
pub struct PartyService {
    repo: Arc<dyn PartyRepository>,
}

impl PartyService {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { repo: Arc::new(PgPartyRepository::new(pool)) }
    }

    // -- supplier --

    pub async fn create_supplier(
        &self,
        _ctx: &AuditContext,
        cmd: CreateSupplierCommand,
    ) -> Result<SupplierView, AppError> {
        cmd.validate().map_err(|e| AppError::validation(e.to_string()))?;
        self.repo.create_supplier(&cmd).await
    }

    pub async fn update_supplier(
        &self,
        _ctx: &AuditContext,
        id: i64,
        cmd: UpdateSupplierCommand,
    ) -> Result<SupplierView, AppError> {
        cmd.validate().map_err(|e| AppError::validation(e.to_string()))?;
        self.repo.update_supplier(id, &cmd).await
    }

    pub async fn get_supplier(&self, id: i64) -> Result<SupplierView, AppError> {
        self.repo.get_supplier(id).await
    }

    pub async fn list_suppliers(
        &self,
        q: &QuerySuppliers,
    ) -> Result<Vec<SupplierView>, AppError> {
        self.repo.list_suppliers(q).await
    }

    // -- customer --

    pub async fn create_customer(
        &self,
        _ctx: &AuditContext,
        cmd: CreateCustomerCommand,
    ) -> Result<CustomerView, AppError> {
        cmd.validate().map_err(|e| AppError::validation(e.to_string()))?;
        self.repo.create_customer(&cmd).await
    }

    pub async fn update_customer(
        &self,
        _ctx: &AuditContext,
        id: i64,
        cmd: UpdateCustomerCommand,
    ) -> Result<CustomerView, AppError> {
        cmd.validate().map_err(|e| AppError::validation(e.to_string()))?;
        self.repo.update_customer(id, &cmd).await
    }

    pub async fn get_customer(&self, id: i64) -> Result<CustomerView, AppError> {
        self.repo.get_customer(id).await
    }

    pub async fn list_customers(
        &self,
        q: &QueryCustomers,
    ) -> Result<Vec<CustomerView>, AppError> {
        self.repo.list_customers(q).await
    }
}
