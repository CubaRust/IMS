//! identity 模块错误码(11xxx 段)

use cuba_shared::error::{AppError, ErrorCode};

// 11000-11099 通用
pub const ID_GENERIC: ErrorCode = ErrorCode::custom(11000);
pub const ID_INVALID_PARAM: ErrorCode = ErrorCode::custom(11001);

// 11100-11499 业务
/// 用户名或密码错误(统一一条,避免枚举用户名)
pub const ID_INVALID_CREDENTIALS: ErrorCode = ErrorCode::custom(11101);
/// 用户被禁用
pub const ID_USER_DISABLED: ErrorCode = ErrorCode::custom(11102);
/// 旧密码错
pub const ID_OLD_PASSWORD_MISMATCH: ErrorCode = ErrorCode::custom(11103);
/// 密码强度不足
pub const ID_WEAK_PASSWORD: ErrorCode = ErrorCode::custom(11104);
/// 用户不存在
pub const ID_USER_NOT_FOUND: ErrorCode = ErrorCode::custom(11105);

pub struct IdentityError;

impl IdentityError {
    #[must_use]
    pub fn invalid_credentials() -> AppError {
        AppError::business(ID_INVALID_CREDENTIALS, "用户名或密码错误")
    }

    #[must_use]
    pub fn user_disabled() -> AppError {
        AppError::business(ID_USER_DISABLED, "用户已被禁用")
    }

    #[must_use]
    pub fn old_password_mismatch() -> AppError {
        AppError::business(ID_OLD_PASSWORD_MISMATCH, "旧密码不正确")
    }

    #[must_use]
    pub fn weak_password() -> AppError {
        AppError::business(
            ID_WEAK_PASSWORD,
            "密码强度不足:至少 8 位,需含字母和数字",
        )
    }

    #[must_use]
    pub fn user_not_found(login_name: &str) -> AppError {
        AppError::business(
            ID_USER_NOT_FOUND,
            format!("用户不存在: {login_name}"),
        )
    }
}
