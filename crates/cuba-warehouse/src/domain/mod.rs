//! warehouse 领域层(本模块以 CRUD 为主,领域规则较少)

use cuba_shared::error::{AppError, ErrorCode};

pub const WH_GENERIC: ErrorCode = ErrorCode::custom(21000);
pub const WH_INVALID_TYPE: ErrorCode = ErrorCode::custom(21101);
pub const WH_IN_USE: ErrorCode = ErrorCode::custom(21102);

pub struct WarehouseError;

impl WarehouseError {
    #[must_use]
    pub fn invalid_wh_type(v: &str) -> AppError {
        AppError::business(
            WH_INVALID_TYPE,
            format!("未知的仓库类型: {v}"),
        )
    }

    #[must_use]
    pub fn in_use() -> AppError {
        AppError::business(WH_IN_USE, "仓库/仓位被业务单据引用,无法删除,请先禁用")
    }
}

pub const WH_TYPES: &[&str] = &[
    "RAW_WH",
    "SEMI_WH",
    "FG_WH",
    "BAD_WH",
    "SCRAP_WH",
    "TRANSIT_WH",
    "RETURN_WH",
    "CHECK_WH",
];

pub const LOC_TYPES: &[&str] = &["NORMAL", "IQC", "BAD", "SCRAP", "TRANSIT", "RETURN", "HOLD"];

#[must_use]
pub fn is_valid_wh_type(v: &str) -> bool {
    WH_TYPES.contains(&v)
}

#[must_use]
pub fn is_valid_loc_type(v: &str) -> bool {
    LOC_TYPES.contains(&v)
}
