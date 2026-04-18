//! 通用业务枚举
//!
//! 这些枚举的字符串值必须与数据库 CHECK 约束**严格一致**。
//! 它们在 service 层用于参数校验,在 repo 层通过 sqlx 以字符串形式序列化。

use std::fmt;

use serde::{Deserialize, Serialize};

/// 单据状态
///
/// 对应 DDL:`doc_status in ('DRAFT','SUBMITTED','COMPLETED','VOIDED')`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DocStatus {
    Draft,
    Submitted,
    Completed,
    Voided,
}

impl DocStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "DRAFT",
            Self::Submitted => "SUBMITTED",
            Self::Completed => "COMPLETED",
            Self::Voided => "VOIDED",
        }
    }

    /// 能否提交(草稿 -> 已提交)
    #[must_use]
    pub const fn can_submit(self) -> bool {
        matches!(self, Self::Draft)
    }

    /// 能否作废
    #[must_use]
    pub const fn can_void(self) -> bool {
        matches!(self, Self::Draft | Self::Submitted)
    }
}

impl fmt::Display for DocStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<&str> for DocStatus {
    type Error = crate::error::AppError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "DRAFT" => Self::Draft,
            "SUBMITTED" => Self::Submitted,
            "COMPLETED" => Self::Completed,
            "VOIDED" => Self::Voided,
            other => {
                return Err(crate::error::AppError::validation(format!(
                    "未知的单据状态: {other}"
                )))
            }
        })
    }
}

// -- 库存状态 -----------------------------------------------------------------

/// 库存状态
///
/// 对应 `sys.sys_dict` `STOCK_STATUS`,与库存余额表、事务头/行里的 `stock_status`
/// 字段保持一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StockStatus {
    ToCheck,
    Qualified,
    Bad,
    Frozen,
    InProcess,
    Outsource,
    PreissuePending,
    CustomerReturnPending,
    Scrapped,
    Recovery,
}

impl StockStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ToCheck => "TO_CHECK",
            Self::Qualified => "QUALIFIED",
            Self::Bad => "BAD",
            Self::Frozen => "FROZEN",
            Self::InProcess => "IN_PROCESS",
            Self::Outsource => "OUTSOURCE",
            Self::PreissuePending => "PREISSUE_PENDING",
            Self::CustomerReturnPending => "CUSTOMER_RETURN_PENDING",
            Self::Scrapped => "SCRAPPED",
            Self::Recovery => "RECOVERY",
        }
    }

    /// 是否计入可用库存
    #[must_use]
    pub const fn is_available(self) -> bool {
        matches!(self, Self::Qualified)
    }
}

impl fmt::Display for StockStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<&str> for StockStatus {
    type Error = crate::error::AppError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "TO_CHECK" => Self::ToCheck,
            "QUALIFIED" => Self::Qualified,
            "BAD" => Self::Bad,
            "FROZEN" => Self::Frozen,
            "IN_PROCESS" => Self::InProcess,
            "OUTSOURCE" => Self::Outsource,
            "PREISSUE_PENDING" => Self::PreissuePending,
            "CUSTOMER_RETURN_PENDING" => Self::CustomerReturnPending,
            "SCRAPPED" => Self::Scrapped,
            "RECOVERY" => Self::Recovery,
            other => {
                return Err(crate::error::AppError::validation(format!(
                    "未知的库存状态: {other}"
                )))
            }
        })
    }
}

// -- 事务类型 -----------------------------------------------------------------

/// 库存事务类型
///
/// 对应 DDL:`txn_type in ('IN','OUT','TRANSFER','CONVERT','RESERVE','RELEASE')`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TxnType {
    In,
    Out,
    Transfer,
    Convert,
    Reserve,
    Release,
}

impl TxnType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::In => "IN",
            Self::Out => "OUT",
            Self::Transfer => "TRANSFER",
            Self::Convert => "CONVERT",
            Self::Reserve => "RESERVE",
            Self::Release => "RELEASE",
        }
    }
}

impl fmt::Display for TxnType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// -- IO flag(事务行的双边标记)------------------------------------------------

/// 事务行的出入标记
///
/// 对应 DDL:`io_flag in ('IN','OUT')`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum IoFlag {
    In,
    Out,
}

impl IoFlag {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::In => "IN",
            Self::Out => "OUT",
        }
    }

    #[must_use]
    pub const fn is_in(self) -> bool {
        matches!(self, Self::In)
    }
}

impl fmt::Display for IoFlag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
