//! cuba-supplier-return
//!
//! 退供商单 (`wms_supplier_return_h/d`)。本质是从源仓位(通常不良仓 BAD)OUT 掉物料,送回供应商。
//!
//! ## 流程
//! 1. `create`:DRAFT
//! 2. `submit`:OUT from (source_wh/loc, source_status) → 库存减少
//! 3. `void`:仅 DRAFT/SUBMITTED

#![deny(unsafe_code)]

pub mod repo;
pub mod service;

pub use repo::{PgSupplierReturnRepository, SupplierReturnRepository};
pub use service::{
    CreateSupplierReturnCommand, CreateSupplierReturnLine, QuerySupplierReturns,
    SubmitSupplierReturnResult, SupplierReturnHeadView, SupplierReturnLineView,
    SupplierReturnService,
};
