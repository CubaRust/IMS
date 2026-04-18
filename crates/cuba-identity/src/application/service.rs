//! identity 应用服务
//!
//! 内部调用链:
//! - `login(cmd)`:
//!   1. repo 按 login_name 查 user
//!   2. 校验 is_active + bcrypt
//!   3. repo 查 user 的 roles + permissions
//!   4. 签 JWT,返回
//! - `change_password(ctx, cmd)`:
//!   1. 取当前 user_id 的记录
//!   2. 校验旧密码
//!   3. 校验新密码强度
//!   4. bcrypt 哈希并 update
//! - `me(ctx)` / `list_users(q)` / `list_roles()` / `list_permissions()`:直接委托 repo

use std::sync::Arc;

use sqlx::PgPool;
use time::OffsetDateTime;
use validator::Validate;

use cuba_shared::{
    audit::AuditContext,
    auth::{jwt, password, Claims},
    error::AppError,
};

use super::{
    commands::{ChangePasswordCommand, LoginCommand, LoginResult},
    dto::{PermissionView, RoleView, UserView},
    queries::QueryUsers,
};
use crate::domain::{errors::IdentityError, model::is_strong_password};
use crate::infrastructure::repository::{IdentityRepository, PgIdentityRepository};

#[derive(Clone)]
pub struct IdentityService {
    repo: Arc<dyn IdentityRepository>,
    jwt_secret: Vec<u8>,
    jwt_ttl_seconds: i64,
}

impl IdentityService {
    /// 生产路径:从连接池 + 配置里的 jwt 参数构造
    #[must_use]
    pub fn new(pool: PgPool, jwt_secret: &str, jwt_ttl_seconds: i64) -> Self {
        Self {
            repo: Arc::new(PgIdentityRepository::new(pool)),
            jwt_secret: jwt_secret.as_bytes().to_vec(),
            jwt_ttl_seconds,
        }
    }

    /// 测试路径:注入自定义 repo
    #[must_use]
    pub fn with_repo(
        repo: Arc<dyn IdentityRepository>,
        jwt_secret: &str,
        jwt_ttl_seconds: i64,
    ) -> Self {
        Self {
            repo,
            jwt_secret: jwt_secret.as_bytes().to_vec(),
            jwt_ttl_seconds,
        }
    }

    /// 登录,颁 JWT
    pub async fn login(&self, cmd: LoginCommand) -> Result<LoginResult, AppError> {
        cmd.validate()
            .map_err(|e| AppError::validation(e.to_string()))?;

        let user = self
            .repo
            .find_user_by_login_name(&cmd.login_name)
            .await?
            .ok_or_else(IdentityError::invalid_credentials)?;

        if !user.is_active {
            return Err(IdentityError::user_disabled());
        }

        let ok = password::verify(&cmd.password, &user.password_hash)?;
        if !ok {
            return Err(IdentityError::invalid_credentials());
        }

        let roles = self.repo.list_user_roles(user.id).await?;
        let permissions = self.repo.list_user_permissions(user.id).await?;

        let now = OffsetDateTime::now_utc().unix_timestamp();
        let exp = now + self.jwt_ttl_seconds;
        let jti = uuid::Uuid::new_v4().to_string();

        let claims = Claims {
            sub: user.id.to_string(),
            login_name: user.login_name.clone(),
            exp,
            iat: now,
            jti,
            tenant_id: user.tenant_id,
            roles: roles.clone(),
            permissions: permissions.clone(),
        };
        let token = jwt::encode_token(&claims, &self.jwt_secret)?;

        Ok(LoginResult {
            token,
            expires_at: exp,
            user_id: user.id,
            user_code: user.user_code,
            login_name: user.login_name,
            user_name: user.user_name,
            roles,
            permissions,
        })
    }

    /// 改自己的密码
    pub async fn change_password(
        &self,
        ctx: &AuditContext,
        cmd: ChangePasswordCommand,
    ) -> Result<(), AppError> {
        cmd.validate()
            .map_err(|e| AppError::validation(e.to_string()))?;

        let user = self
            .repo
            .find_user_by_id(ctx.user_id)
            .await?
            .ok_or_else(|| IdentityError::user_not_found(&ctx.login_name))?;

        let ok = password::verify(&cmd.old_password, &user.password_hash)?;
        if !ok {
            return Err(IdentityError::old_password_mismatch());
        }

        if !is_strong_password(&cmd.new_password) {
            return Err(IdentityError::weak_password());
        }

        let new_hash = password::hash(&cmd.new_password)?;
        self.repo.update_password(ctx.user_id, &new_hash).await?;
        Ok(())
    }

    /// 查当前用户
    pub async fn me(&self, ctx: &AuditContext) -> Result<UserView, AppError> {
        self.repo.me(ctx.user_id).await
    }

    pub async fn list_users(
        &self,
        ctx: &AuditContext,
        q: &QueryUsers,
    ) -> Result<Vec<UserView>, AppError> {
        self.repo.list_users(ctx.tenant_id, q).await
    }

    pub async fn list_roles(&self) -> Result<Vec<RoleView>, AppError> {
        self.repo.list_roles().await
    }

    pub async fn list_permissions(&self) -> Result<Vec<PermissionView>, AppError> {
        self.repo.list_permissions().await
    }

    /// 登出 - 把当前 jti 写黑名单
    pub async fn logout(&self, ctx: &AuditContext, jti: &str, exp: i64) -> Result<(), AppError> {
        self.repo
            .revoke_jwt(jti, Some(ctx.user_id), &ctx.login_name, "LOGOUT", exp)
            .await
    }

    /// 强制下线指定用户(管理员用)
    pub async fn force_logout(&self, _ctx: &AuditContext, user_id: i64) -> Result<u64, AppError> {
        // 本期策略:清理该用户所有未过期 jti 不现实(没存)。
        // 所以给出"强制改密"的等价路径:admin 调 revoke_all_for_user 会给一个哨兵行。
        // 真要精确只吊销某 jti,让用户自己 logout 即可。
        self.repo.revoke_all_for_user(user_id).await
    }

    /// 续 token:给旧 token 签一个新 jti,旧 jti 写黑名单
    ///
    /// 安全:旧 token 必须未过期、未在黑名单里,auth_guard 已保证。
    /// 这里直接查 user,重新 claim。
    pub async fn refresh(
        &self,
        ctx: &AuditContext,
        old_jti: &str,
        old_exp: i64,
    ) -> Result<LoginResult, AppError> {
        let user = self
            .repo
            .find_user_by_id(ctx.user_id)
            .await?
            .ok_or_else(|| IdentityError::user_not_found(&ctx.login_name))?;
        if !user.is_active {
            return Err(IdentityError::user_disabled());
        }

        // 旧 jti 黑名单化
        self.repo
            .revoke_jwt(old_jti, Some(user.id), &user.login_name, "REFRESH", old_exp)
            .await?;

        let roles = self.repo.list_user_roles(user.id).await?;
        let permissions = self.repo.list_user_permissions(user.id).await?;

        let now = OffsetDateTime::now_utc().unix_timestamp();
        let exp = now + self.jwt_ttl_seconds;
        let new_jti = uuid::Uuid::new_v4().to_string();
        let claims = Claims {
            sub: user.id.to_string(),
            login_name: user.login_name.clone(),
            exp,
            iat: now,
            jti: new_jti,
            tenant_id: user.tenant_id,
            roles: roles.clone(),
            permissions: permissions.clone(),
        };
        let token = jwt::encode_token(&claims, &self.jwt_secret)?;
        Ok(LoginResult {
            token,
            expires_at: exp,
            user_id: user.id,
            user_code: user.user_code.clone(),
            user_name: user.user_name.clone(),
            login_name: user.login_name.clone(),
            roles,
            permissions,
        })
    }

    /// auth_guard 调用 — 查 jti 是否已在黑名单
    pub async fn is_jwt_revoked(&self, jti: &str) -> Result<bool, AppError> {
        self.repo.is_jwt_revoked(jti).await
    }
}
