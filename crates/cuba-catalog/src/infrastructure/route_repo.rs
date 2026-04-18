//! route repo

use async_trait::async_trait;
use sqlx::{postgres::PgRow, PgPool, Postgres, Row};

use cuba_shared::error::AppError;

use crate::application::route::{CreateRouteCommand, QueryRoutes, RouteHeadView, RouteStepView};

#[async_trait]
pub trait RouteRepository: Send + Sync {
    async fn create(&self, cmd: &CreateRouteCommand) -> Result<RouteHeadView, AppError>;
    async fn get(&self, id: i64) -> Result<RouteHeadView, AppError>;
    async fn list(&self, q: &QueryRoutes) -> Result<Vec<RouteHeadView>, AppError>;
}

pub struct PgRouteRepository {
    pool: PgPool,
}

impl PgRouteRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RouteRepository for PgRouteRepository {
    async fn create(&self, cmd: &CreateRouteCommand) -> Result<RouteHeadView, AppError> {
        let mut tx = self.pool.begin().await?;

        let id: i64 = sqlx::query_scalar(
            r#"
            insert into mdm.mdm_route_h (route_code, route_name, product_material_id, remark)
            values ($1, $2, $3, $4)
            returning id
            "#,
        )
        .bind(&cmd.route_code)
        .bind(&cmd.route_name)
        .bind(cmd.product_material_id)
        .bind(&cmd.remark)
        .fetch_one(&mut *tx)
        .await
        .map_err(super::map_unique_err)?;

        for s in &cmd.steps {
            let rule = s.rule_json.clone().unwrap_or_else(|| serde_json::json!({}));
            sqlx::query(
                r#"
                insert into mdm.mdm_route_d
                    (route_id, step_no, process_name, output_material_id,
                     semi_finished_flag, rule_json, remark)
                values ($1,$2,$3,$4,$5,$6,$7)
                "#,
            )
            .bind(id)
            .bind(s.step_no)
            .bind(&s.process_name)
            .bind(s.output_material_id)
            .bind(s.semi_finished_flag)
            .bind(&rule)
            .bind(&s.remark)
            .execute(&mut *tx)
            .await
            .map_err(super::map_unique_err)?;
        }

        tx.commit().await?;
        self.get(id).await
    }

    async fn get(&self, id: i64) -> Result<RouteHeadView, AppError> {
        let head = sqlx::query(
            r#"
            select h.id, h.route_code, h.route_name, h.product_material_id,
                   m.material_code as product_material_code,
                   h.is_active, h.remark, h.created_at, h.updated_at
              from mdm.mdm_route_h h
              left join mdm.mdm_material m on m.id = h.product_material_id
             where h.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found(format!("工艺路线 id={id} 不存在")))?;

        let steps = sqlx::query(
            r#"
            select d.id, d.step_no, d.process_name, d.output_material_id,
                   m.material_code as output_material_code,
                   d.semi_finished_flag, d.rule_json, d.remark
              from mdm.mdm_route_d d
              left join mdm.mdm_material m on m.id = d.output_material_id
             where d.route_id = $1
             order by d.step_no
            "#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(row_to_step)
        .collect();

        Ok(row_to_head(head, steps))
    }

    async fn list(&self, q: &QueryRoutes) -> Result<Vec<RouteHeadView>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select h.id, h.route_code, h.route_name, h.product_material_id,
                   m.material_code as product_material_code,
                   h.is_active, h.remark, h.created_at, h.updated_at
              from mdm.mdm_route_h h
              left join mdm.mdm_material m on m.id = h.product_material_id
             where 1 = 1
            "#,
        );
        if let Some(c) = &q.route_code {
            qb.push(" and h.route_code = ").push_bind(c.clone());
        }
        if let Some(mid) = q.product_material_id {
            qb.push(" and h.product_material_id = ").push_bind(mid);
        }
        if let Some(a) = q.is_active {
            qb.push(" and h.is_active = ").push_bind(a);
        }
        qb.push(" order by h.route_code limit 500");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| row_to_head(r, vec![])).collect())
    }
}

fn row_to_head(row: PgRow, steps: Vec<RouteStepView>) -> RouteHeadView {
    RouteHeadView {
        id: row.get("id"),
        route_code: row.get("route_code"),
        route_name: row.get("route_name"),
        product_material_id: row.get("product_material_id"),
        product_material_code: row.get("product_material_code"),
        is_active: row.get("is_active"),
        remark: row.get("remark"),
        steps,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_step(row: PgRow) -> RouteStepView {
    RouteStepView {
        id: row.get("id"),
        step_no: row.get("step_no"),
        process_name: row.get("process_name"),
        output_material_id: row.get("output_material_id"),
        output_material_code: row.get("output_material_code"),
        semi_finished_flag: row.get("semi_finished_flag"),
        rule_json: row.get("rule_json"),
        remark: row.get("remark"),
    }
}
