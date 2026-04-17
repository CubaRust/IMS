//! 库存领域层
//!
//! 只做两件事:
//! 1. 定义领域内的值对象(`StockLocator`、`StockChange`、`ChangeKind` 等)
//! 2. 定义领域规则(事务校验 `validate_txn`)
//!
//! **不依赖 sqlx / axum**,确保单元测试可以离线跑。

pub mod errors;
pub mod model;
pub mod rules;
