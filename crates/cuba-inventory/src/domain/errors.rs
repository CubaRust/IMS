//! 库存模块错误码(20xxx 段)
//!
//! 错误码规范见 docs/architecture/error-code-convention.md

use cuba_shared::error::{AppError, ErrorCode};

// -- 20000-20099: 通用 ------------------------------------------------------
pub const INV_GENERIC: ErrorCode = ErrorCode::custom(20000);
pub const INV_INVALID_PARAM: ErrorCode = ErrorCode::custom(20001);

// -- 20100-20499: 业务规则 --------------------------------------------------
/// 库存不足
pub const INV_INSUFFICIENT: ErrorCode = ErrorCode::custom(20101);
/// 事务结构非法(IN/OUT/TRANSFER 的两侧定义冲突等)
pub const INV_INVALID_TXN: ErrorCode = ErrorCode::custom(20102);
/// 物料启用了批次管理,但入参没给批次
pub const INV_BATCH_REQUIRED: ErrorCode = ErrorCode::custom(20103);
/// 该物料没启用批次管理,但入参带了批次
pub const INV_BATCH_FORBIDDEN: ErrorCode = ErrorCode::custom(20104);
/// 负库存只允许 PREISSUE_PENDING 状态
pub const INV_NEGATIVE_FORBIDDEN: ErrorCode = ErrorCode::custom(20105);
/// 状态流转未配置或不被允许
pub const INV_STATUS_FLOW_DENIED: ErrorCode = ErrorCode::custom(20106);
/// TRANSFER 要求同一批次同一物料
pub const INV_TRANSFER_MISMATCH: ErrorCode = ErrorCode::custom(20107);
/// 事务行数量 <= 0
pub const INV_NONPOSITIVE_QTY: ErrorCode = ErrorCode::custom(20108);

// -- 20500-20799: 基础设施 --------------------------------------------------
pub const INV_DB_ERROR: ErrorCode = ErrorCode::custom(20500);

/// 便于 service 层快速构造错误
pub struct InventoryError;

impl InventoryError {
    #[must_use]
    pub fn insufficient(detail: impl Into<std::borrow::Cow<'static, str>>) -> AppError {
        AppError::business(INV_INSUFFICIENT, detail)
    }

    #[must_use]
    pub fn invalid_txn(reason: impl Into<std::borrow::Cow<'static, str>>) -> AppError {
        AppError::business(INV_INVALID_TXN, reason)
    }

    #[must_use]
    pub fn batch_required(material_code: &str) -> AppError {
        AppError::business(
            INV_BATCH_REQUIRED,
            format!("物料 {material_code} 启用了批次管理,必须提供批次号"),
        )
    }

    #[must_use]
    pub fn nonpositive_qty() -> AppError {
        AppError::business(INV_NONPOSITIVE_QTY, "数量必须大于 0")
    }

    #[must_use]
    pub fn transfer_mismatch() -> AppError {
        AppError::business(
            INV_TRANSFER_MISMATCH,
            "TRANSFER 的两端必须是同一物料/批次",
        )
    }
}
