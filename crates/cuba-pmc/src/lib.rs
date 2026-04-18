//! cuba-pmc
//!
//! PMC 委外单 (`wms_outsource_h/send_d/back_d/scrap_d`)。
//!
//! ## 流程
//! 1. `create`:建立委外单(只登记,先记 send 行和预期 back 行)
//! 2. `submit_send`:送出 OUT,scene=`OUTSOURCE_SEND`
//! 3. `submit_back`:回料 IN,scene=`OUTSOURCE_BACK_IN`,默认状态 TO_CHECK
//! 4. `void`:仅 DRAFT 时
//!
//! 本期简化:一张单独立两个入口 submit_send + submit_back。
//! 支持多次 back(分批回料),每次 back 会累计到 back_d 的 actual_qty。

#![deny(unsafe_code)]

pub mod repo;
pub mod service;

pub use repo::{PgPmcRepository, PmcRepository};
pub use service::{
    CreateOutsourceCommand, CreateOutsourceLine, OutsourceHeadView, OutsourceLineView, PmcService,
    QueryOutsources, SubmitBackCommand, SubmitBackLine, SubmitResult,
};
