//! 系统配置:数据字典 + 单据编码规则

use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, PgPool, Postgres, Row};
use time::PrimitiveDateTime;
use validator::Validate;

use cuba_shared::error::AppError;

// -- Dict --------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictView {
    pub id: i64,
    pub dict_type: String,
    pub dict_key: String,
    pub dict_value: String,
    pub dict_order: i32,
    pub is_active: bool,
    pub remark: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryDicts {
    pub dict_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateDictCommand {
    #[validate(length(min = 1, max = 50))]
    pub dict_type: String,
    #[validate(length(min = 1, max = 100))]
    pub dict_key: String,
    #[validate(length(min = 1, max = 200))]
    pub dict_value: String,
    #[serde(default)]
    pub dict_order: i32,
    #[serde(default)]
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateDictCommand {
    #[validate(length(min = 1, max = 200))]
    pub dict_value: Option<String>,
    pub dict_order: Option<i32>,
    pub is_active: Option<bool>,
    pub remark: Option<String>,
}

// -- DocNoRule ---------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocNoRuleView {
    pub id: i64,
    pub doc_type: String,
    pub doc_prefix: String,
    pub date_pattern: String,
    pub seq_length: i32,
    pub current_date_str: Option<String>,
    pub current_seq: i32,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateDocNoRuleCommand {
    #[validate(length(min = 1, max = 20))]
    pub doc_prefix: Option<String>,
    pub date_pattern: Option<String>,
    pub seq_length: Option<i32>,
}

// -- Service -----------------------------------------------------------------

#[derive(Clone)]
pub struct SystemConfigService {
    pool: PgPool,
}

impl SystemConfigService {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // -- dict --

    pub async fn list_dicts(&self, q: &QueryDicts) -> Result<Vec<DictView>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select id, dict_type, dict_key, dict_value, dict_order,
                   is_active, remark, created_at, updated_at
              from sys.sys_dict
             where 1 = 1
            "#,
        );
        if let Some(t) = &q.dict_type {
            qb.push(" and dict_type = ").push_bind(t.clone());
        }
        qb.push(" order by dict_type, dict_order, dict_key");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_dict).collect())
    }

    pub async fn create_dict(&self, cmd: CreateDictCommand) -> Result<DictView, AppError> {
        cmd.validate()
            .map_err(|e| AppError::validation(e.to_string()))?;
        let id: i64 = sqlx::query_scalar(
            r#"
            insert into sys.sys_dict (dict_type, dict_key, dict_value, dict_order, remark)
            values ($1,$2,$3,$4,$5)
            returning id
            "#,
        )
        .bind(&cmd.dict_type)
        .bind(&cmd.dict_key)
        .bind(&cmd.dict_value)
        .bind(cmd.dict_order)
        .bind(&cmd.remark)
        .fetch_one(&self.pool)
        .await
        .map_err(map_unique_err)?;

        self.get_dict(id).await
    }

    pub async fn update_dict(&self, id: i64, cmd: UpdateDictCommand) -> Result<DictView, AppError> {
        cmd.validate()
            .map_err(|e| AppError::validation(e.to_string()))?;

        let mut qb = sqlx::QueryBuilder::<Postgres>::new("update sys.sys_dict set ");
        let mut sep = qb.separated(", ");
        if let Some(v) = &cmd.dict_value {
            sep.push("dict_value = ").push_bind_unseparated(v.clone());
        }
        if let Some(o) = cmd.dict_order {
            sep.push("dict_order = ").push_bind_unseparated(o);
        }
        if let Some(a) = cmd.is_active {
            sep.push("is_active = ").push_bind_unseparated(a);
        }
        if let Some(r) = &cmd.remark {
            sep.push("remark = ").push_bind_unseparated(r.clone());
        }
        qb.push(" where id = ").push_bind(id);
        qb.build().execute(&self.pool).await?;
        self.get_dict(id).await
    }

    async fn get_dict(&self, id: i64) -> Result<DictView, AppError> {
        let row = sqlx::query(
            r#"
            select id, dict_type, dict_key, dict_value, dict_order,
                   is_active, remark, created_at, updated_at
              from sys.sys_dict where id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found(format!("字典 id={id} 不存在")))?;
        Ok(row_to_dict(row))
    }

    // -- doc_no_rule --

    pub async fn list_doc_no_rules(&self) -> Result<Vec<DocNoRuleView>, AppError> {
        let rows = sqlx::query(
            r#"
            select id, doc_type, doc_prefix, date_pattern, seq_length,
                   current_date_str, current_seq, updated_at
              from sys.sys_doc_no_rule
             order by doc_type
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_rule).collect())
    }

    pub async fn update_doc_no_rule(
        &self,
        id: i64,
        cmd: UpdateDocNoRuleCommand,
    ) -> Result<DocNoRuleView, AppError> {
        cmd.validate()
            .map_err(|e| AppError::validation(e.to_string()))?;

        let mut qb = sqlx::QueryBuilder::<Postgres>::new("update sys.sys_doc_no_rule set ");
        let mut sep = qb.separated(", ");
        if let Some(p) = &cmd.doc_prefix {
            sep.push("doc_prefix = ").push_bind_unseparated(p.clone());
        }
        if let Some(d) = &cmd.date_pattern {
            sep.push("date_pattern = ").push_bind_unseparated(d.clone());
        }
        if let Some(s) = cmd.seq_length {
            sep.push("seq_length = ").push_bind_unseparated(s);
        }
        qb.push(" where id = ").push_bind(id);
        qb.build().execute(&self.pool).await?;

        let row = sqlx::query(
            r#"
            select id, doc_type, doc_prefix, date_pattern, seq_length,
                   current_date_str, current_seq, updated_at
              from sys.sys_doc_no_rule where id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found(format!("编码规则 id={id} 不存在")))?;
        Ok(row_to_rule(row))
    }
}

fn row_to_dict(row: PgRow) -> DictView {
    DictView {
        id: row.get("id"),
        dict_type: row.get("dict_type"),
        dict_key: row.get("dict_key"),
        dict_value: row.get("dict_value"),
        dict_order: row.get("dict_order"),
        is_active: row.get("is_active"),
        remark: row.get("remark"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_rule(row: PgRow) -> DocNoRuleView {
    DocNoRuleView {
        id: row.get("id"),
        doc_type: row.get("doc_type"),
        doc_prefix: row.get("doc_prefix"),
        date_pattern: row.get("date_pattern"),
        seq_length: row.get("seq_length"),
        current_date_str: row.get("current_date_str"),
        current_seq: row.get("current_seq"),
        updated_at: row.get("updated_at"),
    }
}

fn map_unique_err(e: sqlx::Error) -> AppError {
    match &e {
        sqlx::Error::Database(db) if db.code().as_deref() == Some("23505") => {
            AppError::conflict(format!("唯一约束冲突: {}", db.message()))
        }
        _ => e.into(),
    }
}
