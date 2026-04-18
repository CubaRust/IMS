//! cuba-identity
//!
//! 认证与用户/角色/权限管理。
//!
//! ## 对外 API(通过 `cuba-api` 暴露 HTTP)
//! - 登录颁 JWT
//! - 改密
//! - 当前用户 + 权限摊平列表
//! - 用户 / 角色 / 权限的基础查询(admin)
//!
//! ## 架构
//! - `domain` : 值对象、登录凭据验证规则
//! - `application` : `LoginCommand` / `ChangePasswordCommand` / `QueryUsers`
//! - `infrastructure` : `PgIdentityRepository` 实现
//!
//! JWT 签名秘钥从 `AppConfig::jwt_secret` 读,本 crate 不关心如何配置。

#![deny(unsafe_code)]

pub mod application;
pub mod domain;
pub mod infrastructure;

pub use application::system_config::{
    CreateDictCommand, DictView, DocNoRuleView, QueryDicts, SystemConfigService, UpdateDictCommand,
    UpdateDocNoRuleCommand,
};
pub use application::{
    ChangePasswordCommand, IdentityService, LoginCommand, LoginResult, UserView,
};
pub use domain::errors::{self, IdentityError};
pub use infrastructure::repository::{IdentityRepository, PgIdentityRepository};
