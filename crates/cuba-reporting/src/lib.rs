//! cuba-reporting
//!
//! 只读报表,基于 0009 建立的 `rpt.*` 视图:
//! - `rpt.v_stock_aging` 库龄
//! - `rpt.v_stock_dormant` 呆滞
//! - `rpt.v_exception_summary` 异常统计
//! - `rpt.v_txn_flow` 收发存流水

#![deny(unsafe_code)]

pub mod service;

pub use service::{
    AgingBucketRow, AnomalyTodoRow, DashboardData, DefectStats30dRow, DormantRow,
    ExceptionSummaryRow, InventoryByLocationRow, InventoryByMaterialRow, LowStockWarningRow,
    OutsourceInTransitRow, QueryAging, QueryAnomalyTodo, QueryDefectStats30d, QueryDormant,
    QueryExceptionSummary, QueryInventoryByLocation, QueryInventoryByMaterial,
    QueryLowStockWarning, QueryOutsourceInTransit, QueryTodayIo, QueryTxnFlow, ReportingService,
    TodayIoRow, TxnFlowRow,
};
