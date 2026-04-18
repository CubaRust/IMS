//! Recovery template repo: head + detail, same pattern as BOM

use async_trait::async_trait;
use sqlx::{postgres::PgRow, PgPool, Postgres, Row};

use cuba_shared::error::AppError;

use crate::application::recovery_tpl::{
    CreateRecoveryTplCommand, QueryRecoveryTpls, RecoveryTplHeadView, RecoveryTplLineView,
};

#[async_trait]
pub trait RecoveryTplRepository: Send + Sync {
    async fn create(&self, cmd: &CreateRecoveryTplCommand)
        -> Result<RecoveryTplHeadView, AppError>;
    async fn get(&self, id: i64) -> Result<RecoveryTplHeadView, AppError>;
    async fn list(&self, q: &QueryRecoveryTpls) -> Result<Vec<RecoveryTplHeadView>, AppError>;
}

pub struct PgRecoveryTplRepository {
    pool: PgPool,
}

impl PgRecoveryTplRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RecoveryTplRepository for PgRecoveryTplRepository {
    async fn create(
        &self,
        cmd: &CreateRecoveryTplCommand,
    ) -> Result<RecoveryTplHeadView, AppError> {
        let mut tx = self.pool.begin().await?;

        let id: i64 = sqlx::query_scalar(
            r#"
            insert into mdm.mdm_recovery_tpl_h (tpl_code, tpl_name, source_material_id, remark)
            values ($1,$2,$3,$4)
            returning id
            "#,
        )
        .bind(&cmd.tpl_code)
        .bind(&cmd.tpl_name)
        .bind(cmd.source_material_id)
        .bind(&cmd.remark)
        .fetch_one(&mut *tx)
        .await
        .map_err(super::map_unique_err)?;

        for l in &cmd.lines {
            sqlx::query(
                r#"
                insert into mdm.mdm_recovery_tpl_d
                    (tpl_id, line_no, target_material_id, default_recovery_qty,
                     target_default_status, scrap_flag, remark)
                values ($1,$2,$3,$4,$5,$6,$7)
                "#,
            )
            .bind(id)
            .bind(l.line_no)
            .bind(l.target_material_id)
            .bind(l.default_recovery_qty)
            .bind(&l.target_default_status)
            .bind(l.scrap_flag)
            .bind(&l.remark)
            .execute(&mut *tx)
            .await
            .map_err(super::map_unique_err)?;
        }

        tx.commit().await?;
        self.get(id).await
    }

    async fn get(&self, id: i64) -> Result<RecoveryTplHeadView, AppError> {
        let head = sqlx::query(
            r#"
            select h.id, h.tpl_code, h.tpl_name, h.source_material_id,
                   m.material_code as source_material_code,
                   h.is_active, h.remark, h.created_at, h.updated_at
              from mdm.mdm_recovery_tpl_h h
              left join mdm.mdm_material m on m.id = h.source_material_id
             where h.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found(format!("回收模板 id={id} 不存在")))?;

        let lines = sqlx::query(
            r#"
            select d.id, d.line_no, d.target_material_id, m.material_code as target_material_code,
                   d.default_recovery_qty, d.target_default_status, d.scrap_flag, d.remark
              from mdm.mdm_recovery_tpl_d d
              left join mdm.mdm_material m on m.id = d.target_material_id
             where d.tpl_id = $1
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

    async fn list(&self, q: &QueryRecoveryTpls) -> Result<Vec<RecoveryTplHeadView>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select h.id, h.tpl_code, h.tpl_name, h.source_material_id,
                   m.material_code as source_material_code,
                   h.is_active, h.remark, h.created_at, h.updated_at
              from mdm.mdm_recovery_tpl_h h
              left join mdm.mdm_material m on m.id = h.source_material_id
             where 1 = 1
            "#,
        );
        if let Some(c) = &q.tpl_code {
            qb.push(" and h.tpl_code = ").push_bind(c.clone());
        }
        if let Some(mid) = q.source_material_id {
            qb.push(" and h.source_material_id = ").push_bind(mid);
        }
        if let Some(a) = q.is_active {
            qb.push(" and h.is_active = ").push_bind(a);
        }
        qb.push(" order by h.tpl_code limit 500");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| row_to_head(r, vec![])).collect())
    }
}

fn row_to_head(row: PgRow, lines: Vec<RecoveryTplLineView>) -> RecoveryTplHeadView {
    RecoveryTplHeadView {
        id: row.get("id"),
        tpl_code: row.get("tpl_code"),
        tpl_name: row.get("tpl_name"),
        source_material_id: row.get("source_material_id"),
        source_material_code: row.get("source_material_code"),
        is_active: row.get("is_active"),
        remark: row.get("remark"),
        lines,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_line(row: PgRow) -> RecoveryTplLineView {
    RecoveryTplLineView {
        id: row.get("id"),
        line_no: row.get("line_no"),
        target_material_id: row.get("target_material_id"),
        target_material_code: row.get("target_material_code"),
        default_recovery_qty: row.get("default_recovery_qty"),
        target_default_status: row.get("target_default_status"),
        scrap_flag: row.get("scrap_flag"),
        remark: row.get("remark"),
    }
}
