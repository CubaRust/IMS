//! catalog 领域层(主数据校验)

use cuba_shared::error::{AppError, ErrorCode};

// 22000-22099 通用
pub const CAT_GENERIC: ErrorCode = ErrorCode::custom(22000);
// 22100-22499 业务
pub const CAT_INVALID_CATEGORY: ErrorCode = ErrorCode::custom(22101);
pub const CAT_INVALID_PROCESS_TYPE: ErrorCode = ErrorCode::custom(22102);
pub const CAT_BOM_EMPTY: ErrorCode = ErrorCode::custom(22103);
pub const CAT_ROUTE_EMPTY: ErrorCode = ErrorCode::custom(22104);
pub const CAT_DUPLICATE_STEP: ErrorCode = ErrorCode::custom(22105);
pub const CAT_RECOVERY_TPL_EMPTY: ErrorCode = ErrorCode::custom(22106);

pub const MATERIAL_CATEGORIES: &[&str] =
    &["RAW", "SEMI", "FINISHED", "PUBLIC", "RECOVERY", "SCRAP"];

pub const PROCESS_TYPES: &[&str] = &["GG", "GF", "TP", "ASM", "FOG", "OTHER"];

#[must_use]
pub fn is_valid_category(v: &str) -> bool {
    MATERIAL_CATEGORIES.contains(&v)
}

#[must_use]
pub fn is_valid_process_type(v: &str) -> bool {
    PROCESS_TYPES.contains(&v)
}

pub struct CatalogError;

impl CatalogError {
    #[must_use]
    pub fn invalid_category(v: &str) -> AppError {
        AppError::business(CAT_INVALID_CATEGORY, format!("未知的物料类别: {v}"))
    }

    #[must_use]
    pub fn invalid_process_type(v: &str) -> AppError {
        AppError::business(CAT_INVALID_PROCESS_TYPE, format!("未知的工艺类型: {v}"))
    }

    #[must_use]
    pub fn bom_empty() -> AppError {
        AppError::business(CAT_BOM_EMPTY, "BOM 行不能为空")
    }

    #[must_use]
    pub fn route_empty() -> AppError {
        AppError::business(CAT_ROUTE_EMPTY, "工艺路线必须至少一步")
    }

    #[must_use]
    pub fn duplicate_step(step_no: i32) -> AppError {
        AppError::business(CAT_DUPLICATE_STEP, format!("工序步骤号 {step_no} 重复"))
    }

    #[must_use]
    pub fn recovery_tpl_empty() -> AppError {
        AppError::business(CAT_RECOVERY_TPL_EMPTY, "回收模板行不能为空")
    }
}
