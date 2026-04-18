//! defect 领域层

use cuba_shared::error::{AppError, ErrorCode};

// 40xxx 段
pub const DEF_INVALID_SOURCE: ErrorCode = ErrorCode::custom(40101);
pub const DEF_INVALID_METHOD: ErrorCode = ErrorCode::custom(40102);
pub const DEF_INVALID_STAGE: ErrorCode = ErrorCode::custom(40103);
pub const DEF_EMPTY_LINES: ErrorCode = ErrorCode::custom(40104);
pub const DEF_INVALID_TRANSITION: ErrorCode = ErrorCode::custom(40105);

pub const DEFECT_SOURCES: &[&str] = &["IQC", "PROD", "CUSTOMER_RETURN", "OUTSOURCE"];
pub const PRODUCT_STAGES: &[&str] = &["RAW", "FOG", "TP", "ASM"];
pub const PROCESS_METHODS: &[&str] = &["TO_BAD_STOCK", "TO_DISMANTLE", "TO_SCRAP", "TO_REWORK"];

#[must_use]
pub fn is_valid_source(v: &str) -> bool {
    DEFECT_SOURCES.contains(&v)
}
#[must_use]
pub fn is_valid_stage(v: &str) -> bool {
    PRODUCT_STAGES.contains(&v)
}
#[must_use]
pub fn is_valid_method(v: &str) -> bool {
    PROCESS_METHODS.contains(&v)
}

pub struct DefectError;

impl DefectError {
    #[must_use]
    pub fn invalid_source(v: &str) -> AppError {
        AppError::business(DEF_INVALID_SOURCE, format!("未知不良来源: {v}"))
    }
    #[must_use]
    pub fn invalid_method(v: &str) -> AppError {
        AppError::business(DEF_INVALID_METHOD, format!("未知不良处理方式: {v}"))
    }
    #[must_use]
    pub fn invalid_stage(v: &str) -> AppError {
        AppError::business(DEF_INVALID_STAGE, format!("未知产品阶段: {v}"))
    }
    #[must_use]
    pub fn empty_lines() -> AppError {
        AppError::business(DEF_EMPTY_LINES, "不良行不能为空")
    }
    #[must_use]
    pub fn invalid_transition(from: &str, action: &str) -> AppError {
        AppError::business(
            DEF_INVALID_TRANSITION,
            format!("不良单当前状态 {from} 不允许 {action}"),
        )
    }
}
