//! 认证相关
//!
//! - [`Claims`] JWT payload 定义
//! - [`password`] 密码哈希与校验

use serde::{Deserialize, Serialize};

/// JWT Claims
///
/// ## 字段
/// - `sub` 主题:user_id(字符串化,兼容 JWT 标准)
/// - `login_name` 登录名
/// - `exp` 过期时间(unix seconds)
/// - `iat` 签发时间
/// - `roles` 角色码列表
/// - `permissions` 权限码列表(一次性摊平,减少每次请求查库)
///
/// 权限点变更后,需要用户重新登录才能刷新;对低频变更可以接受,
/// 若需要即时失效,另外做 token 黑名单。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub login_name: String,
    pub exp: i64,
    pub iat: i64,
    /// JWT ID,登出/吊销时按此值写黑名单
    #[serde(default)]
    pub jti: String,
    /// 租户 id(多租户隔离)
    #[serde(default = "default_tenant")]
    pub tenant_id: i64,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
}

const fn default_tenant() -> i64 {
    1
}

impl Claims {
    #[must_use]
    pub fn user_id(&self) -> Option<i64> {
        self.sub.parse().ok()
    }
}

pub mod password {
    //! 密码哈希与校验(bcrypt,cost=10)
    use crate::error::AppError;

    /// 哈希密码
    pub fn hash(plain: &str) -> Result<String, AppError> {
        bcrypt::hash(plain, 10)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("bcrypt hash failed: {e}")))
    }

    /// 校验密码
    pub fn verify(plain: &str, hashed: &str) -> Result<bool, AppError> {
        bcrypt::verify(plain, hashed)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("bcrypt verify failed: {e}")))
    }
}

pub mod jwt {
    //! JWT 编解码
    use jsonwebtoken::{
        decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
    };

    use super::Claims;
    use crate::error::AppError;

    /// 编码
    pub fn encode_token(claims: &Claims, secret: &[u8]) -> Result<String, AppError> {
        encode(
            &Header::new(Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(secret),
        )
        .map_err(|e| AppError::Internal(anyhow::anyhow!("jwt encode failed: {e}")))
    }

    /// 解码并校验
    pub fn decode_token(token: &str, secret: &[u8]) -> Result<Claims, AppError> {
        let validation = Validation::new(Algorithm::HS256);
        let data: TokenData<Claims> =
            decode(token, &DecodingKey::from_secret(secret), &validation)
                .map_err(|_| AppError::Unauthenticated)?;
        Ok(data.claims)
    }
}
