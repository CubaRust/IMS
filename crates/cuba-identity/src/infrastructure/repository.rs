//! identity 仓储
//!
//! 关键查询:
//! - `find_user_by_login_name`:登录校验必用
//! - `list_user_permissions`:join `sys_user_role` + `sys_role_permission` + `sys_permission`
//!   对 `sys_user_role.user_id` 过滤 + distinct `perm_code` 摊平
//! - `list_user_roles`:同理,返回 role_code 列表

use async_trait::async_trait;
use sqlx::{postgres::PgRow, PgPool, Postgres, Row};

use cuba_shared::error::AppError;

use crate::application::{
    dto::{PermissionView, RoleView, UserView},
    queries::QueryUsers,
};
use crate::domain::model::User;

/// identity 仓储接口
#[async_trait]
pub trait IdentityRepository: Send + Sync {
    async fn find_user_by_login_name(&self, login_name: &str) -> Result<Option<User>, AppError>;
    async fn find_user_by_id(&self, id: i64) -> Result<Option<User>, AppError>;

    async fn list_user_roles(&self, user_id: i64) -> Result<Vec<String>, AppError>;
    async fn list_user_permissions(&self, user_id: i64) -> Result<Vec<String>, AppError>;

    async fn update_password(&self, user_id: i64, new_hash: &str) -> Result<(), AppError>;

    async fn me(&self, user_id: i64) -> Result<UserView, AppError>;

    async fn list_users(
        &self,
        tenant_id: i64,
        q: &QueryUsers,
    ) -> Result<Vec<UserView>, AppError>;
    async fn list_roles(&self) -> Result<Vec<RoleView>, AppError>;
    async fn list_permissions(&self) -> Result<Vec<PermissionView>, AppError>;

    // --- JWT 吊销 ---
    async fn revoke_jwt(
        &self,
        jti: &str,
        user_id: Option<i64>,
        login_name: &str,
        reason: &str,
        exp: i64,
    ) -> Result<(), AppError>;
    async fn revoke_all_for_user(&self, user_id: i64) -> Result<u64, AppError>;
    async fn is_jwt_revoked(&self, jti: &str) -> Result<bool, AppError>;
}

pub struct PgIdentityRepository {
    pool: PgPool,
}

impl PgIdentityRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IdentityRepository for PgIdentityRepository {
    async fn find_user_by_login_name(&self, login_name: &str) -> Result<Option<User>, AppError> {
        let row = sqlx::query(
            r#"
            select id, tenant_id, user_code, user_name, login_name, password_hash,
                   mobile, is_active, created_at, updated_at
              from sys.sys_user
             where login_name = $1
            "#,
        )
        .bind(login_name)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(row_to_user))
    }

    async fn find_user_by_id(&self, id: i64) -> Result<Option<User>, AppError> {
        let row = sqlx::query(
            r#"
            select id, tenant_id, user_code, user_name, login_name, password_hash,
                   mobile, is_active, created_at, updated_at
              from sys.sys_user
             where id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(row_to_user))
    }

    async fn list_user_roles(&self, user_id: i64) -> Result<Vec<String>, AppError> {
        let rows = sqlx::query_scalar::<_, String>(
            r#"
            select distinct r.role_code
              from sys.sys_user_role ur
              join sys.sys_role r on r.id = ur.role_id
             where ur.user_id = $1
               and r.is_active = true
             order by r.role_code
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn list_user_permissions(&self, user_id: i64) -> Result<Vec<String>, AppError> {
        let rows = sqlx::query_scalar::<_, String>(
            r#"
            select distinct p.perm_code
              from sys.sys_user_role ur
              join sys.sys_role r             on r.id = ur.role_id  and r.is_active = true
              join sys.sys_role_permission rp on rp.role_id = r.id
              join sys.sys_permission p       on p.id = rp.permission_id
             where ur.user_id = $1
             order by p.perm_code
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn update_password(&self, user_id: i64, new_hash: &str) -> Result<(), AppError> {
        sqlx::query(
            "update sys.sys_user set password_hash = $1, updated_at = now() where id = $2",
        )
        .bind(new_hash)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn me(&self, user_id: i64) -> Result<UserView, AppError> {
        let row = sqlx::query(
            r#"
            select id, user_code, user_name, login_name, mobile, is_active,
                   created_at, updated_at
              from sys.sys_user
             where id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let roles = self.list_user_roles(user_id).await?;
        Ok(row_to_user_view(row, roles))
    }

    async fn list_users(
        &self,
        tenant_id: i64,
        q: &QueryUsers,
    ) -> Result<Vec<UserView>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select u.id, u.user_code, u.user_name, u.login_name,
                   u.mobile, u.is_active, u.created_at, u.updated_at
              from sys.sys_user u
            "#,
        );

        if let Some(role) = &q.role_code {
            qb.push(
                " join sys.sys_user_role ur on ur.user_id = u.id \
                  join sys.sys_role r on r.id = ur.role_id and r.role_code = ",
            )
            .push_bind(role.clone());
        }

        qb.push(" where u.tenant_id = ").push_bind(tenant_id);

        if let Some(login) = &q.login_name {
            qb.push(" and u.login_name ilike ").push_bind(format!("%{login}%"));
        }
        if let Some(name) = &q.user_name {
            qb.push(" and u.user_name ilike ").push_bind(format!("%{name}%"));
        }
        if let Some(active) = q.is_active {
            qb.push(" and u.is_active = ").push_bind(active);
        }

        qb.push(" order by u.id desc limit 500");

        let rows = qb.build().fetch_all(&self.pool).await?;

        // 逐行装配 roles(用户不多,本期简化。真要优化可以一次 IN + 分组)
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let user_id: i64 = r.get("id");
            let roles = self.list_user_roles(user_id).await?;
            out.push(row_to_user_view(r, roles));
        }
        Ok(out)
    }

    async fn list_roles(&self) -> Result<Vec<RoleView>, AppError> {
        let rows = sqlx::query(
            r#"
            select id, role_code, role_name, is_active
              from sys.sys_role
             order by role_code
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| RoleView {
                id: r.get("id"),
                role_code: r.get("role_code"),
                role_name: r.get("role_name"),
                is_active: r.get("is_active"),
            })
            .collect())
    }

    async fn list_permissions(&self) -> Result<Vec<PermissionView>, AppError> {
        let rows = sqlx::query(
            r#"
            select id, perm_code, perm_name, module_code, action_code
              from sys.sys_permission
             order by module_code, perm_code
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| PermissionView {
                id: r.get("id"),
                perm_code: r.get("perm_code"),
                perm_name: r.get("perm_name"),
                module_code: r.get("module_code"),
                action_code: r.get("action_code"),
            })
            .collect())
    }

    // --- JWT 吊销 ---

    async fn revoke_jwt(
        &self,
        jti: &str,
        user_id: Option<i64>,
        login_name: &str,
        reason: &str,
        exp: i64,
    ) -> Result<(), AppError> {
        // exp 是 unix seconds,转 Timestamp
        let exp_dt = time::OffsetDateTime::from_unix_timestamp(exp)
            .map_err(|e| AppError::validation(format!("exp 不是合法时间戳: {e}")))?;
        let exp_primitive =
            time::PrimitiveDateTime::new(exp_dt.date(), exp_dt.time());
        sqlx::query(
            r#"
            insert into sys.sys_jwt_revocation (jti, user_id, login_name, reason, expires_at)
            values ($1, $2, $3, $4, $5)
            on conflict (jti) do nothing
            "#,
        )
        .bind(jti)
        .bind(user_id)
        .bind(login_name)
        .bind(reason)
        .bind(exp_primitive)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn revoke_all_for_user(&self, user_id: i64) -> Result<u64, AppError> {
        // 本期简化:插入一条 "哨兵" 广播行,auth_guard 扩展时可按 user_id + iat 做失效判定
        // 目前 auth_guard 只查 jti,所以这里暂时**只记录**,不强制下线(下次可以加 iat 比较)。
        let rows = sqlx::query(
            r#"
            insert into sys.sys_jwt_revocation
                (jti, user_id, login_name, reason, expires_at)
            values ($1, $2,
                (select login_name from sys.sys_user where id = $2),
                'FORCE_LOGOUT',
                now() + interval '30 days')
            "#,
        )
        .bind(format!("force-{user_id}-{}", uuid::Uuid::new_v4()))
        .bind(user_id)
        .execute(&self.pool)
        .await?
        .rows_affected();
        Ok(rows)
    }

    async fn is_jwt_revoked(&self, jti: &str) -> Result<bool, AppError> {
        let cnt: Option<i64> = sqlx::query_scalar(
            "select 1 from sys.sys_jwt_revocation where jti = $1 limit 1",
        )
        .bind(jti)
        .fetch_optional(&self.pool)
        .await?;
        Ok(cnt.is_some())
    }
}

// ---------------------------------------------------------------------------
// row -> model
// ---------------------------------------------------------------------------

fn row_to_user(row: PgRow) -> User {
    User {
        id: row.get("id"),
        tenant_id: row.try_get("tenant_id").unwrap_or(1),
        user_code: row.get("user_code"),
        user_name: row.get("user_name"),
        login_name: row.get("login_name"),
        password_hash: row.get("password_hash"),
        mobile: row.get("mobile"),
        is_active: row.get("is_active"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_user_view(row: PgRow, roles: Vec<String>) -> UserView {
    UserView {
        id: row.get("id"),
        user_code: row.get("user_code"),
        user_name: row.get("user_name"),
        login_name: row.get("login_name"),
        mobile: row.get("mobile"),
        is_active: row.get("is_active"),
        roles,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}
