//! cuba-preissue
//!
//! 异常先发 / 待补入库 (`wms_preissue_h/d`)。
//!
//! ## 关键规则(需求 11.5 / 文档第 9、10 节)
//! 1. 只有开启 `allow_preissue_flag` 的物料允许走此流程(校验在 service 层)
//! 2. 创建时强制填写 `reason`
//! 3. 库存走 PREISSUE_PENDING 状态:book_qty 允许负,pending_qty 累加
//! 4. 正式入库时 `wms_inbound_d.related_preissue_line_id` 指回 `wms_preissue_d` 自动冲销
//! 5. 异常状态:PENDING / PARTIAL / CLOSED / OVERTIME / VOIDED
//! 6. 超期规则:由后台定时任务刷 `timeout_flag`(本期先不做,仅占位字段)
//!
//! ## 冲销逻辑
//! 入库 submit 时(cuba-inbound 做):若 `related_preissue_line_id` 有值,
//! 则本模块提供 `close_preissue_line(line_id, qty)`:
//! 1. `wms_preissue_d.filled_qty += qty`,`unfilled_qty -= qty`
//! 2. 若 unfilled 清零,line_status → CLOSED,head 重新聚合(全 closed → CLOSED;否则 PARTIAL)
//! 3. 在 inventory 里写一笔 CONVERT:OUT PREISSUE_PENDING, IN 正常状态

#![deny(unsafe_code)]

pub mod application;
pub mod domain;
pub mod infrastructure;

pub use application::{
    CreatePreissueCommand, CreatePreissueLine, PreissueHeadView, PreissueLineView,
    PreissueService, QueryPreissues, SubmitPreissueResult,
};
pub use infrastructure::repository::{PgPreissueRepository, PreissueRepository};
