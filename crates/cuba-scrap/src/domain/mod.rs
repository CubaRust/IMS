//! scrap domain

use cuba_shared::error::{AppError, ErrorCode};

pub const SCR_EMPTY_LINES: ErrorCode = ErrorCode::custom(42101);
pub const SCR_INVALID_TRANSITION: ErrorCode = ErrorCode::custom(42102);

pub const SCRAP_SOURCES: &[&str] = &[
    "IQC_BAD",
    "PROD_BAD",
    "RECOVERY_LEFTOVER",
    "CUSTOMER_RETURN_BAD",
    "STOCKTAKE_DAMAGE",
    "OTHER",
];

#[must_use]
pub fn is_valid_source(v: &str) -> bool {
    SCRAP_SOURCES.contains(&v)
}

pub struct ScrapError;

impl ScrapError {
    #[must_use]
    pub fn empty_lines() -> AppError {
        AppError::business(SCR_EMPTY_LINES, "报废单行不能为空")
    }
    #[must_use]
    pub fn invalid_transition(from: &str, action: &str) -> AppError {
        AppError::business(
            SCR_INVALID_TRANSITION,
            format!("报废单当前状态 {from} 不允许 {action}"),
        )
    }
}
