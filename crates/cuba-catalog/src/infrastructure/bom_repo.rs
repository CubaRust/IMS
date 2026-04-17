//! BOM repo:头+行,创建走同一 DB 事务

use async_trait::async_trait;
use sqlx::{postgres::PgRow, PgPool, Postgres, Row};

use cuba_shared::error::AppError;

use crate::application::bom::{BomHeadView, BomLineView, CreateBomCommand, QueryBoms};

#[async_trait]
pub trait BomRepository: Send + Sync {
    async fn create(&self, cmd: &CreateBomCommand) -> Result<BomHeadView, AppError>;
    async fn get(&self, id: i64) -> Result<BomHeadView, AppError>;
    async fn list(&self, q: &QueryBoms) -> Result<Vec<BomHeadView>, AppError>;
}

pub struct PgBomRepository {
    pool: PgPool,
}

impl PgBomRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BomRepository for PgBomRepository {
    async fn create(&self, cmd: &CreateBomCommand) -> Result<BomHeadView, AppError> {
        let mut tx = self.pool.begin().await?;

        let id: i64 = sqlx::query_scalar(
            r#"
            insert into mdm.mdm_bom_h (bom_code, bom_version, product_material_id, route_id, remark)
            values ($1,$2,$3,$4,$5)
            returning id
            "#,
        )
        .bind(&cmd.bom_code)
        .bind(&cmd.bom_version)
        .bind(cmd.product_material_id)
        .bind(cmd.route_id)
        .bind(&cmd.remark)
        .fetch_one(&mut *tx)
        .await
        .map_err(super::map_unique_err)?;

        for l in &cmd.lines {
            sqlx::query(
                r#"
                insert into mdm.mdm_bom_d
                    (bom_id, line_no, material_id, usage_qty, loss_rate,
                     public_material_flag, remark)
                values ($1,$2,$3,$4,$5,$6,$7)
                "#,
            )
            .bind(id)
            .bind(l.line_no)
            .bind(l.material_id)
            .bind(l.usage_qty)
            .bind(l.loss_rate)
            .bind(l.public_material_flag)
            .bind(&l.remark)
            .execute(&mut *tx)
            .await
            .map_err(super::map_unique_err)?;
        }

        tx.commit().await?;
        self.get(id).await
    }

    async fn get(&self, id: i64) -> Result<BomHeadView, AppError> {
        let head = sqlx::query(
            r#"
            select h.id, h.bom_code, h.bom_version, h.product_material_id,
                   m.material_code as product_material_code,
                   h.route_id, h.is_active, h.remark, h.created_at, h.updated_at
              from mdm.mdm_bom_h h
              left join mdm.mdm_material m on m.id = h.product_material_id
             where h.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found(format!("BOM id={id} 不存在")))?;

        let lines = sqlx::query(
            r#"
            select d.id, d.line_no, d.material_id, m.material_code,
                   d.usage_qty, d.loss_rate, d.public_material_flag, d.remark
              from mdm.mdm_bom_d d
              left join mdm.mdm_material m on m.id = d.material_id
             where d.bom_id = $1
             order by d.line_no
            "#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(row_to_line)
        .collect();

        Ok(row_to_head(head, lines))
    }

    async fn list(&self, q: &QueryBoms) -> Result<Vec<BomHeadView>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select h.id, h.bom_code, h.bom_version, h.product_material_id,
                   m.material_code as product_material_code,
                   h.route_id, h.is_active, h.remark, h.created_at, h.updated_at
              from mdm.mdm_bom_h h
              left join mdm.mdm_material m on m.id = h.product_material_id
             where 1 = 1
            "#,
        );
        if let Some(c) = &q.bom_code {
            qb.push(" and h.bom_code = ").push_bind(c.clone());
        }
        if let Some(mid) = q.product_material_id {
            qb.push(" and h.product_material_id = ").push_bind(mid);
        }
        if let Some(a) = q.is_active {
            qb.push(" and h.is_active = ").push_bind(a);
        }
        qb.push(" order by h.bom_code limit 500");
        let rows = qb.build().fetch_all(&self.pool).await?;

        // 列表不带行明细,表格页另请求详情;避免 N+1 查询
        let items = rows.into_iter().map(|r| row_to_head(r, vec![])).collect();
        Ok(items)
    }
}

fn row_to_head(row: PgRow, lines: Vec<BomLineView>) -> BomHeadView {
    BomHeadView {
        id: row.get("id"),
        bom_code: row.get("bom_code"),
        bom_version: row.get("bom_version"),
        product_material_id: row.get("product_material_id"),
        product_material_code: row.get("product_material_code"),
        route_id: row.get("route_id"),
        is_active: row.get("is_active"),
        remark: row.get("remark"),
        lines,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_line(row: PgRow) -> BomLineView {
    BomLineView {
        id: row.get("id"),
        line_no: row.get("line_no"),
        material_id: row.get("material_id"),
        material_code: row.get("material_code"),
        usage_qty: row.get("usage_qty"),
        loss_rate: row.get("loss_rate"),
        public_material_flag: row.get("public_material_flag"),
        remark: row.get("remark"),
    }
}
