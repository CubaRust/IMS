//! identity 领域模型

use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

/// 用户实体(数据库行投影)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub user_code: String,
    pub user_name: String,
    pub login_name: String,
    pub password_hash: String,
    pub mobile: Option<String>,
    pub is_active: bool,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

/// 密码强度校验
///
/// 规则:长度 >= 8,包含字母和数字(本期简化)。
/// 返回 `true` 表示通过。
#[must_use]
pub fn is_strong_password(plain: &str) -> bool {
    if plain.len() < 8 {
        return false;
    }
    let has_alpha = plain.chars().any(|c| c.is_ascii_alphabetic());
    let has_digit = plain.chars().any(|c| c.is_ascii_digit());
    has_alpha && has_digit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strong_password_accepts_reasonable() {
        assert!(is_strong_password("Admin@123"));
        assert!(is_strong_password("abc12345"));
    }

    #[test]
    fn strong_password_rejects_short() {
        assert!(!is_strong_password("a1"));
    }

    #[test]
    fn strong_password_rejects_no_digit() {
        assert!(!is_strong_password("abcdefgh"));
    }

    #[test]
    fn strong_password_rejects_no_alpha() {
        assert!(!is_strong_password("12345678"));
    }
}
