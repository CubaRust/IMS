//! 审计上下文
//!
//! 贯穿每次请求:由中间件从 JWT / 请求头解析,挂进请求扩展,
//! application 层用 `FromRequestParts` 或显式参数拿到。
//!
//! 所有业务写操作必须接受 `AuditContext`,以便:
//! - 记录 `operator_id` 到单据
//! - 记录 `trace_id` 到流水,方便排错
//! - 记录 `ip / user_agent` 到审计日志(本期先预留字段)

use serde::{Deserialize, Serialize};

/// 审计上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditContext {
    pub user_id: i64,
    pub login_name: String,
    pub trace_id: String,
    #[serde(default)]
    pub ip: Option<String>,
    #[serde(default)]
    pub user_agent: Option<String>,
    /// 用户所拥有的权限码(从 JWT 解出,已在中间件校验)
    #[serde(default)]
    pub permissions: Vec<String>,
    /// 用户所属角色码
    #[serde(default)]
    pub roles: Vec<String>,
    /// 当前 JWT 的 jti(logout/refresh 用)
    #[serde(default)]
    pub jti: Option<String>,
    /// 当前 JWT 的 exp(unix seconds)
    #[serde(default)]
    pub jwt_exp: Option<i64>,
}

impl AuditContext {
    /// 仅用于 **测试和内部后台任务**,不允许从 HTTP 层构造
    #[must_use]
    pub fn system(trace_id: impl Into<String>) -> Self {
        Self {
            user_id: 0,
            login_name: "system".to_string(),
            trace_id: trace_id.into(),
            ip: None,
            user_agent: None,
            permissions: vec!["*".to_string()],
            roles: vec!["SYSTEM".to_string()],
            jti: None,
            jwt_exp: None,
        }
    }

    #[must_use]
    pub fn has_permission(&self, perm: &str) -> bool {
        self.permissions.iter().any(|p| p == "*" || p == perm)
    }

    #[must_use]
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// 在 service 层做权限判断的快捷方法
    pub fn require_permission(&self, perm: &str) -> Result<(), crate::error::AppError> {
        if self.has_permission(perm) {
            Ok(())
        } else {
            Err(crate::error::AppError::forbidden(perm.to_string()))
        }
    }
}
