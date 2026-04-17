//! preissue 领域层

use cuba_shared::error::{AppError, ErrorCode};

// 33xxx 段
pub const PRE_GENERIC: ErrorCode = ErrorCode::custom(33000);
pub const PRE_REASON_REQUIRED: ErrorCode = ErrorCode::custom(33101);
pub const PRE_MATERIAL_NOT_ALLOWED: ErrorCode = ErrorCode::custom(33102);
pub const PRE_EMPTY_LINES: ErrorCode = ErrorCode::custom(33103);
pub const PRE_ALREADY_CLOSED: ErrorCode = ErrorCode::custom(33104);
pub const PRE_OVERFILL: ErrorCode = ErrorCode::custom(33105);
pub const PRE_STATUS_MISMATCH: ErrorCode = ErrorCode::custom(33106);

pub struct PreissueError;

impl PreissueError {
    #[must_use]
    pub fn reason_required() -> AppError {
        AppError::business(PRE_REASON_REQUIRED, "异常先发必须填写原因")
    }

    #[must_use]
    pub fn material_not_allowed(material_code: &str) -> AppError {
        AppError::business(
            PRE_MATERIAL_NOT_ALLOWED,
            format!("物料 {material_code} 未开启异常先发(allow_preissue_flag=false)"),
        )
    }

    #[must_use]
    pub fn empty_lines() -> AppError {
        AppError::business(PRE_EMPTY_LINES, "异常先发行不能为空")
    }

    #[must_use]
    pub fn already_closed(no: &str) -> AppError {
        AppError::business(PRE_ALREADY_CLOSED, format!("异常单 {no} 已闭环"))
    }

    #[must_use]
    pub fn overfill(line_id: i64) -> AppError {
        AppError::business(
            PRE_OVERFILL,
            format!("补入库数量超过待补数量(preissue_line={line_id})"),
        )
    }

    #[must_use]
    pub fn status_mismatch(status: &str, action: &str) -> AppError {
        AppError::business(
            PRE_STATUS_MISMATCH,
            format!("异常单当前状态 {status} 不允许 {action}"),
        )
    }
}
