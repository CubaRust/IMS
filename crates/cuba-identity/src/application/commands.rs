//! identity command
//!
//! 写操作:登录、改密

use serde::{Deserialize, Serialize};
use validator::Validate;

/// 登录命令
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct LoginCommand {
    #[validate(length(min = 1, max = 100))]
    pub login_name: String,

    #[validate(length(min = 1, max = 128))]
    pub password: String,
}

/// 登录返回
#[derive(Debug, Clone, Serialize)]
pub struct LoginResult {
    pub token: String,
    pub expires_at: i64, // unix seconds
    pub user_id: i64,
    pub user_code: String,
    pub login_name: String,
    pub user_name: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

/// 改密命令
///
/// 需提供旧密码 + 新密码(即使管理员改自己的密码也一样)。
/// 管理员重置别人密码用另一个 command(本期先不做)。
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ChangePasswordCommand {
    #[validate(length(min = 1, max = 128))]
    pub old_password: String,
    #[validate(length(min = 8, max = 128))]
    pub new_password: String,
}
