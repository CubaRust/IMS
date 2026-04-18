//! warehouse repository

use async_trait::async_trait;
use sqlx::{postgres::PgRow, PgPool, Postgres, Row};

use cuba_shared::error::AppError;

use crate::application::{
    CreateLocationCommand, CreateWarehouseCommand, LocationView, QueryLocations, QueryWarehouses,
    UpdateLocationCommand, UpdateWarehouseCommand, WarehouseView,
};

#[async_trait]
pub trait WarehouseRepository: Send + Sync {
    async fn create_warehouse(
        &self,
        cmd: &CreateWarehouseCommand,
    ) -> Result<WarehouseView, AppError>;
    async fn update_warehouse(
        &self,
        id: i64,
        cmd: &UpdateWarehouseCommand,
    ) -> Result<WarehouseView, AppError>;
    async fn get_warehouse(&self, id: i64) -> Result<WarehouseView, AppError>;
    async fn list_warehouses(&self, q: &QueryWarehouses) -> Result<Vec<WarehouseView>, AppError>;

    async fn create_location(&self, cmd: &CreateLocationCommand) -> Result<LocationView, AppError>;
    async fn update_location(
        &self,
        id: i64,
        cmd: &UpdateLocationCommand,
    ) -> Result<LocationView, AppError>;
    async fn get_location(&self, id: i64) -> Result<LocationView, AppError>;
    async fn list_locations(&self, q: &QueryLocations) -> Result<Vec<LocationView>, AppError>;
}

pub struct PgWarehouseRepository {
    pool: PgPool,
}

impl PgWarehouseRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WarehouseRepository for PgWarehouseRepository {
    async fn create_warehouse(
        &self,
        cmd: &CreateWarehouseCommand,
    ) -> Result<WarehouseView, AppError> {
        let id: i64 = sqlx::query_scalar(
            r#"
            insert into mdm.mdm_warehouse (wh_code, wh_name, wh_type, remark)
            values ($1, $2, $3, $4)
            returning id
            "#,
        )
        .bind(&cmd.wh_code)
        .bind(&cmd.wh_name)
        .bind(&cmd.wh_type)
        .bind(&cmd.remark)
        .fetch_one(&self.pool)
        .await
        .map_err(map_unique_err)?;

        self.get_warehouse(id).await
    }

    async fn update_warehouse(
        &self,
        id: i64,
        cmd: &UpdateWarehouseCommand,
    ) -> Result<WarehouseView, AppError> {
        let rows = sqlx::query(
            r#"
            update mdm.mdm_warehouse
               set wh_name = $2, wh_type = $3, is_active = $4, remark = $5
             where id = $1
            "#,
        )
        .bind(id)
        .bind(&cmd.wh_name)
        .bind(&cmd.wh_type)
        .bind(cmd.is_active)
        .bind(&cmd.remark)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows == 0 {
            return Err(AppError::not_found(format!("仓库 id={id} 不存在")));
        }
        self.get_warehouse(id).await
    }

    async fn get_warehouse(&self, id: i64) -> Result<WarehouseView, AppError> {
        let row = sqlx::query(
            r#"
            select id, wh_code, wh_name, wh_type, is_active, remark,
                   created_at, updated_at
              from mdm.mdm_warehouse
             where id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found(format!("仓库 id={id} 不存在")))?;
        Ok(row_to_warehouse(row))
    }

    async fn list_warehouses(&self, q: &QueryWarehouses) -> Result<Vec<WarehouseView>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select id, wh_code, wh_name, wh_type, is_active, remark,
                   created_at, updated_at
              from mdm.mdm_warehouse
             where 1 = 1
            "#,
        );
        if let Some(code) = &q.wh_code {
            qb.push(" and wh_code = ").push_bind(code.clone());
        }
        if let Some(t) = &q.wh_type {
            qb.push(" and wh_type = ").push_bind(t.clone());
        }
        if let Some(active) = q.is_active {
            qb.push(" and is_active = ").push_bind(active);
        }
        qb.push(" order by wh_code");

        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_warehouse).collect())
    }

    async fn create_location(&self, cmd: &CreateLocationCommand) -> Result<LocationView, AppError> {
        let id: i64 = sqlx::query_scalar(
            r#"
            insert into mdm.mdm_location (wh_id, loc_code, loc_name, loc_type, remark)
            values ($1, $2, $3, $4, $5)
            returning id
            "#,
        )
        .bind(cmd.wh_id)
        .bind(&cmd.loc_code)
        .bind(&cmd.loc_name)
        .bind(&cmd.loc_type)
        .bind(&cmd.remark)
        .fetch_one(&self.pool)
        .await
        .map_err(map_unique_err)?;

        self.get_location(id).await
    }

    async fn update_location(
        &self,
        id: i64,
        cmd: &UpdateLocationCommand,
    ) -> Result<LocationView, AppError> {
        let rows = sqlx::query(
            r#"
            update mdm.mdm_location
               set loc_name = $2, loc_type = $3, is_active = $4, remark = $5
             where id = $1
            "#,
        )
        .bind(id)
        .bind(&cmd.loc_name)
        .bind(&cmd.loc_type)
        .bind(cmd.is_active)
        .bind(&cmd.remark)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows == 0 {
            return Err(AppError::not_found(format!("仓位 id={id} 不存在")));
        }
        self.get_location(id).await
    }

    async fn get_location(&self, id: i64) -> Result<LocationView, AppError> {
        let row = sqlx::query(
            r#"
            select l.id, l.wh_id, w.wh_code, l.loc_code, l.loc_name,
                   l.loc_type, l.is_active, l.remark,
                   l.created_at, l.updated_at
              from mdm.mdm_location l
              join mdm.mdm_warehouse w on w.id = l.wh_id
             where l.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found(format!("仓位 id={id} 不存在")))?;
        Ok(row_to_location(row))
    }

    async fn list_locations(&self, q: &QueryLocations) -> Result<Vec<LocationView>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select l.id, l.wh_id, w.wh_code, l.loc_code, l.loc_name,
                   l.loc_type, l.is_active, l.remark,
                   l.created_at, l.updated_at
              from mdm.mdm_location l
              join mdm.mdm_warehouse w on w.id = l.wh_id
             where 1 = 1
            "#,
        );
        if let Some(wh) = q.wh_id {
            qb.push(" and l.wh_id = ").push_bind(wh);
        }
        if let Some(t) = &q.loc_type {
            qb.push(" and l.loc_type = ").push_bind(t.clone());
        }
        if let Some(active) = q.is_active {
            qb.push(" and l.is_active = ").push_bind(active);
        }
        qb.push(" order by w.wh_code, l.loc_code");

        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_location).collect())
    }
}

// -- helpers -----------------------------------------------------------------

fn map_unique_err(e: sqlx::Error) -> AppError {
    match &e {
        sqlx::Error::Database(db) if db.code().as_deref() == Some("23505") => {
            AppError::conflict(format!("唯一约束冲突: {}", db.message()))
        }
        _ => e.into(),
    }
}

fn row_to_warehouse(row: PgRow) -> WarehouseView {
    WarehouseView {
        id: row.get("id"),
        wh_code: row.get("wh_code"),
        wh_name: row.get("wh_name"),
        wh_type: row.get("wh_type"),
        is_active: row.get("is_active"),
        remark: row.get("remark"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_location(row: PgRow) -> LocationView {
    LocationView {
        id: row.get("id"),
        wh_id: row.get("wh_id"),
        wh_code: row.get("wh_code"),
        loc_code: row.get("loc_code"),
        loc_name: row.get("loc_name"),
        loc_type: row.get("loc_type"),
        is_active: row.get("is_active"),
        remark: row.get("remark"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}
