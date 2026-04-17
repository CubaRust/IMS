//! 错误码枚举
//!
//! 业务 crate 定义自己模块段内的错误码时,用 `ErrorCode::custom(20101)` 构造,
//! 或者在 crate 内部定义常量:
//!
//! ```ignore
//! use cuba_shared::error::ErrorCode;
//! pub const INV_INSUFFICIENT: ErrorCode = ErrorCode::custom(20101);
//! ```

use serde::{Deserialize, Serialize};

/// 错误码。内部是 `u32`,便于 JSON 序列化和跨模块传递。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ErrorCode(u32);

impl ErrorCode {
    /// 自定义错误码
    #[must_use]
    pub const fn custom(code: u32) -> Self {
        Self(code)
    }

    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    // -- 通用段(10xxx) ----------------------------------------------------

    /// 未分类的业务错误
    pub const GENERIC: Self = Self(10000);
    /// 未登录
    pub const UNAUTHENTICATED: Self = Self(10401);
    /// 无权限
    pub const FORBIDDEN: Self = Self(10403);
    /// 参数校验失败
    pub const VALIDATION: Self = Self(10002);
    /// 资源不存在
    pub const NOT_FOUND: Self = Self(10404);
    /// 资源冲突(唯一约束、并发更新等)
    pub const CONFLICT: Self = Self(10409);
    /// 内部错误(数据库 / IO / 未分类)
    pub const INTERNAL: Self = Self(10500);
    /// 功能未实现
    pub const NOT_IMPLEMENTED: Self = Self(10501);
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<ErrorCode> for u32 {
    fn from(c: ErrorCode) -> Self {
        c.0
    }
}
