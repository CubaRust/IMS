//! inbound 领域层(校验 + 错误码)

use cuba_shared::error::{AppError, ErrorCode};

// 30xxx 段
pub const INB_GENERIC: ErrorCode = ErrorCode::custom(30000);
pub const INB_INVALID_TYPE: ErrorCode = ErrorCode::custom(30101);
pub const INB_EMPTY_LINES: ErrorCode = ErrorCode::custom(30102);
pub const INB_INVALID_STATUS_TRANSITION: ErrorCode = ErrorCode::custom(30103);
pub const INB_ALREADY_COMPLETED: ErrorCode = ErrorCode::custom(30104);

pub const INBOUND_TYPES: &[&str] = &[
    "PURCHASE",
    "PROD",
    "RETURN",
    "OUTSOURCE_BACK",
    "CUSTOMER_RETURN",
    "RECOVERY_IN",
    "OTHER",
];

#[must_use]
pub fn is_valid_inbound_type(v: &str) -> bool {
    INBOUND_TYPES.contains(&v)
}

pub struct InboundError;

impl InboundError {
    #[must_use]
    pub fn invalid_type(v: &str) -> AppError {
        AppError::business(INB_INVALID_TYPE, format!("未知的入库类型: {v}"))
    }

    #[must_use]
    pub fn empty_lines() -> AppError {
        AppError::business(INB_EMPTY_LINES, "入库单行不能为空")
    }

    #[must_use]
    pub fn invalid_transition(from: &str, action: &str) -> AppError {
        AppError::business(
            INB_INVALID_STATUS_TRANSITION,
            format!("入库单当前状态 {from} 不允许 {action}"),
        )
    }

    #[must_use]
    pub fn already_completed() -> AppError {
        AppError::business(INB_ALREADY_COMPLETED, "入库单已完成,无法修改")
    }
}

/// 根据入库类型,推断目标库存状态:
/// - PURCHASE → TO_CHECK (来料待检)
/// - PROD → QUALIFIED (产成品合格)
/// - OUTSOURCE_BACK → TO_CHECK (委外回料要检)
/// - 其他 → QUALIFIED (可以被上层 override)
#[must_use]
pub fn default_target_status(inbound_type: &str) -> &'static str {
    match inbound_type {
        "PURCHASE" | "OUTSOURCE_BACK" => "TO_CHECK",
        _ => "QUALIFIED",
    }
}

/// 根据入库类型推出 scene_code(写进库存事务流水)
#[must_use]
pub fn scene_code_for(inbound_type: &str) -> &'static str {
    match inbound_type {
        "PURCHASE" => "PURCHASE_IN",
        "PROD" => "PROD_IN",
        "RETURN" => "PROD_RETURN_IN",
        "OUTSOURCE_BACK" => "OUTSOURCE_BACK_IN",
        "CUSTOMER_RETURN" => "CUSTOMER_RETURN_IN",
        "RECOVERY_IN" => "RECOVERY_IN",
        _ => "OTHER_IN",
    }
}
