//! outbound 领域层

use cuba_shared::error::{AppError, ErrorCode};

// 31xxx
pub const OUT_INVALID_TYPE: ErrorCode = ErrorCode::custom(31101);
pub const OUT_EMPTY_LINES: ErrorCode = ErrorCode::custom(31102);
pub const OUT_INVALID_STATUS_TRANSITION: ErrorCode = ErrorCode::custom(31103);
pub const OUT_WORKORDER_REQUIRED: ErrorCode = ErrorCode::custom(31104);

pub const OUTBOUND_TYPES: &[&str] = &[
    "PROD_ISSUE",
    "PROCESS_ISSUE",
    "PUBLIC_ISSUE",
    "OUTSOURCE_SEND",
    "SUPPLIER_RETURN",
    "SALES_SEND",
    "SCRAP_OUT",
    "OTHER",
];

#[must_use]
pub fn is_valid_outbound_type(v: &str) -> bool {
    OUTBOUND_TYPES.contains(&v)
}

pub struct OutboundError;

impl OutboundError {
    #[must_use]
    pub fn invalid_type(v: &str) -> AppError {
        AppError::business(OUT_INVALID_TYPE, format!("未知的出库类型: {v}"))
    }

    #[must_use]
    pub fn empty_lines() -> AppError {
        AppError::business(OUT_EMPTY_LINES, "出库单行不能为空")
    }

    #[must_use]
    pub fn invalid_transition(from: &str, action: &str) -> AppError {
        AppError::business(
            OUT_INVALID_STATUS_TRANSITION,
            format!("出库单当前状态 {from} 不允许 {action}"),
        )
    }

    #[must_use]
    pub fn workorder_required() -> AppError {
        AppError::business(
            OUT_WORKORDER_REQUIRED,
            "生产发料/工序发料必须绑定工单号",
        )
    }
}

/// 根据出库类型推 scene_code
#[must_use]
pub fn scene_code_for(outbound_type: &str) -> &'static str {
    match outbound_type {
        "PROD_ISSUE" => "PROD_ISSUE",
        "PROCESS_ISSUE" => "PROCESS_ISSUE",
        "PUBLIC_ISSUE" => "PUBLIC_ISSUE",
        "OUTSOURCE_SEND" => "OUTSOURCE_SEND",
        "SUPPLIER_RETURN" => "SUPPLIER_RETURN_OUT",
        "SALES_SEND" => "SALES_SEND",
        "SCRAP_OUT" => "SCRAP_OUT",
        _ => "OTHER_OUT",
    }
}

/// PROD_ISSUE / PROCESS_ISSUE 必须绑 work_order
#[must_use]
pub fn requires_work_order(outbound_type: &str) -> bool {
    matches!(outbound_type, "PROD_ISSUE" | "PROCESS_ISSUE")
}
