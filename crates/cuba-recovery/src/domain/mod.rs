//! recovery domain

use cuba_shared::error::{AppError, ErrorCode};

pub const REC_EMPTY_IN: ErrorCode = ErrorCode::custom(41101);
pub const REC_BALANCE_MISMATCH: ErrorCode = ErrorCode::custom(41102);
pub const REC_INVALID_TRANSITION: ErrorCode = ErrorCode::custom(41103);

pub struct RecoveryError;

impl RecoveryError {
    #[must_use]
    pub fn empty_in() -> AppError {
        AppError::business(REC_EMPTY_IN, "拆解输入(NG 品)不能为空")
    }
    #[must_use]
    pub fn invalid_transition(from: &str, action: &str) -> AppError {
        AppError::business(
            REC_INVALID_TRANSITION,
            format!("拆解回收单当前状态 {from} 不允许 {action}"),
        )
    }
}
