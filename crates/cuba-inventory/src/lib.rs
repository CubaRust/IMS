//! cuba-inventory
//!
//! 库存引擎。工作区的所有库存变化必须经过本 crate。
//!
//! ## 分层
//! - `domain`          : 值对象、领域服务(五类事务动作的业务规则)
//! - `application`     : command(写)、query(读)
//! - `infrastructure`  : sqlx 实现的 `InventoryRepository`
//!
//! ## 对外使用方式
//! ```ignore
//! use cuba_inventory::{InventoryService, CommitTxnCommand};
//! let svc = InventoryService::new(pool);
//! let committed = svc.commit(&ctx, cmd).await?;
//! ```
//!
//! ## 不变量
//! 1. 所有库存变化必须通过 `CommitTxnCommand`,禁止业务侧直写余额表
//! 2. TRANSFER 在事务行里必然表现为 1 行 OUT + 1 行 IN
//! 3. 不做真负库存。异常先发走 `pending_qty` + `PREISSUE_PENDING` 状态
//! 4. 事务头/行/余额更新必须在同一 DB 事务里提交

#![deny(unsafe_code)]

pub mod application;
pub mod domain;
pub mod infrastructure;

// 门面
pub use application::{
    commands::{CommitTxnCommand, CommitTxnResult, TxnLineInput, TxnSideInput},
    dto::{BalanceView, TxnHeadView, TxnLineView},
    queries::{QueryBalance, QueryTxns},
    service::InventoryService,
};
pub use domain::errors::{self, InventoryError};
pub use infrastructure::repository::{InventoryRepository, PgInventoryRepository};
