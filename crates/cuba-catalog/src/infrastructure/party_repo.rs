//! supplier + customer repo

use async_trait::async_trait;
use sqlx::{postgres::PgRow, PgPool, Postgres, Row};

use cuba_shared::error::AppError;

use crate::application::party::{
    CreateCustomerCommand, CreateSupplierCommand, CustomerView, QueryCustomers, QuerySuppliers,
    SupplierView, UpdateCustomerCommand, UpdateSupplierCommand,
};

#[async_trait]
pub trait PartyRepository: Send + Sync {
    // supplier
    async fn create_supplier(&self, cmd: &CreateSupplierCommand) -> Result<SupplierView, AppError>;
    async fn update_supplier(
        &self,
        id: i64,
        cmd: &UpdateSupplierCommand,
    ) -> Result<SupplierView, AppError>;
    async fn get_supplier(&self, id: i64) -> Result<SupplierView, AppError>;
    async fn list_suppliers(&self, q: &QuerySuppliers) -> Result<Vec<SupplierView>, AppError>;

    // customer
    async fn create_customer(&self, cmd: &CreateCustomerCommand) -> Result<CustomerView, AppError>;
    async fn update_customer(
        &self,
        id: i64,
        cmd: &UpdateCustomerCommand,
    ) -> Result<CustomerView, AppError>;
    async fn get_customer(&self, id: i64) -> Result<CustomerView, AppError>;
    async fn list_customers(&self, q: &QueryCustomers) -> Result<Vec<CustomerView>, AppError>;
}

pub struct PgPartyRepository {
    pool: PgPool,
}

impl PgPartyRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PartyRepository for PgPartyRepository {
    async fn create_supplier(&self, cmd: &CreateSupplierCommand) -> Result<SupplierView, AppError> {
        let id: i64 = sqlx::query_scalar(
            r#"
            insert into mdm.mdm_supplier
                (supplier_code, supplier_name, contact_name, contact_phone, address, remark)
            values ($1,$2,$3,$4,$5,$6)
            returning id
            "#,
        )
        .bind(&cmd.supplier_code)
        .bind(&cmd.supplier_name)
        .bind(&cmd.contact_name)
        .bind(&cmd.contact_phone)
        .bind(&cmd.address)
        .bind(&cmd.remark)
        .fetch_one(&self.pool)
        .await
        .map_err(super::map_unique_err)?;
        self.get_supplier(id).await
    }

    async fn update_supplier(
        &self,
        id: i64,
        cmd: &UpdateSupplierCommand,
    ) -> Result<SupplierView, AppError> {
        let rows = sqlx::query(
            r#"
            update mdm.mdm_supplier
               set supplier_name = $2, contact_name = $3, contact_phone = $4,
                   address = $5, is_active = $6, remark = $7
             where id = $1
            "#,
        )
        .bind(id)
        .bind(&cmd.supplier_name)
        .bind(&cmd.contact_name)
        .bind(&cmd.contact_phone)
        .bind(&cmd.address)
        .bind(cmd.is_active)
        .bind(&cmd.remark)
        .execute(&self.pool)
        .await?
        .rows_affected();
        if rows == 0 {
            return Err(AppError::not_found(format!("供应商 id={id} 不存在")));
        }
        self.get_supplier(id).await
    }

    async fn get_supplier(&self, id: i64) -> Result<SupplierView, AppError> {
        let row = sqlx::query(
            r#"
            select id, supplier_code, supplier_name, contact_name, contact_phone,
                   address, is_active, remark, created_at, updated_at
              from mdm.mdm_supplier where id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found(format!("供应商 id={id} 不存在")))?;
        Ok(row_to_supplier(row))
    }

    async fn list_suppliers(&self, q: &QuerySuppliers) -> Result<Vec<SupplierView>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select id, supplier_code, supplier_name, contact_name, contact_phone,
                   address, is_active, remark, created_at, updated_at
              from mdm.mdm_supplier where 1 = 1
            "#,
        );
        if let Some(kw) = &q.keyword {
            qb.push(" and (supplier_code ilike ")
                .push_bind(format!("%{kw}%"))
                .push(" or supplier_name ilike ")
                .push_bind(format!("%{kw}%"))
                .push(")");
        }
        if let Some(active) = q.is_active {
            qb.push(" and is_active = ").push_bind(active);
        }
        qb.push(" order by supplier_code limit 500");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_supplier).collect())
    }

    async fn create_customer(&self, cmd: &CreateCustomerCommand) -> Result<CustomerView, AppError> {
        let id: i64 = sqlx::query_scalar(
            r#"
            insert into mdm.mdm_customer
                (customer_code, customer_name, contact_name, contact_phone, address, remark)
            values ($1,$2,$3,$4,$5,$6)
            returning id
            "#,
        )
        .bind(&cmd.customer_code)
        .bind(&cmd.customer_name)
        .bind(&cmd.contact_name)
        .bind(&cmd.contact_phone)
        .bind(&cmd.address)
        .bind(&cmd.remark)
        .fetch_one(&self.pool)
        .await
        .map_err(super::map_unique_err)?;
        self.get_customer(id).await
    }

    async fn update_customer(
        &self,
        id: i64,
        cmd: &UpdateCustomerCommand,
    ) -> Result<CustomerView, AppError> {
        let rows = sqlx::query(
            r#"
            update mdm.mdm_customer
               set customer_name = $2, contact_name = $3, contact_phone = $4,
                   address = $5, is_active = $6, remark = $7
             where id = $1
            "#,
        )
        .bind(id)
        .bind(&cmd.customer_name)
        .bind(&cmd.contact_name)
        .bind(&cmd.contact_phone)
        .bind(&cmd.address)
        .bind(cmd.is_active)
        .bind(&cmd.remark)
        .execute(&self.pool)
        .await?
        .rows_affected();
        if rows == 0 {
            return Err(AppError::not_found(format!("客户 id={id} 不存在")));
        }
        self.get_customer(id).await
    }

    async fn get_customer(&self, id: i64) -> Result<CustomerView, AppError> {
        let row = sqlx::query(
            r#"
            select id, customer_code, customer_name, contact_name, contact_phone,
                   address, is_active, remark, created_at, updated_at
              from mdm.mdm_customer where id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found(format!("客户 id={id} 不存在")))?;
        Ok(row_to_customer(row))
    }

    async fn list_customers(&self, q: &QueryCustomers) -> Result<Vec<CustomerView>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select id, customer_code, customer_name, contact_name, contact_phone,
                   address, is_active, remark, created_at, updated_at
              from mdm.mdm_customer where 1 = 1
            "#,
        );
        if let Some(kw) = &q.keyword {
            qb.push(" and (customer_code ilike ")
                .push_bind(format!("%{kw}%"))
                .push(" or customer_name ilike ")
                .push_bind(format!("%{kw}%"))
                .push(")");
        }
        if let Some(active) = q.is_active {
            qb.push(" and is_active = ").push_bind(active);
        }
        qb.push(" order by customer_code limit 500");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_customer).collect())
    }
}

fn row_to_supplier(row: PgRow) -> SupplierView {
    SupplierView {
        id: row.get("id"),
        supplier_code: row.get("supplier_code"),
        supplier_name: row.get("supplier_name"),
        contact_name: row.get("contact_name"),
        contact_phone: row.get("contact_phone"),
        address: row.get("address"),
        is_active: row.get("is_active"),
        remark: row.get("remark"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_customer(row: PgRow) -> CustomerView {
    CustomerView {
        id: row.get("id"),
        customer_code: row.get("customer_code"),
        customer_name: row.get("customer_name"),
        contact_name: row.get("contact_name"),
        contact_phone: row.get("contact_phone"),
        address: row.get("address"),
        is_active: row.get("is_active"),
        remark: row.get("remark"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}
