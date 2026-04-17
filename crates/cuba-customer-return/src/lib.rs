//! cuba-customer-return
//!
//! 客户退货单 (`wms_customer_return_h/d`)。
//!
//! ## 流程
//! 1. `create`:DRAFT,登记退货物料、数量、退货原因
//! 2. `judge`(可选):判定每行处理方式(RETURN_TO_STOCK / TO_DEFECT / TO_SCRAP / TO_SUPPLIER_RETURN)
//! 3. `submit`:
//!    - 对判定为 RETURN_TO_STOCK 的行 → IN QUALIFIED
//!    - 对判定为 TO_DEFECT 的行 → IN BAD(到不良仓)
//!    - 对判定为 TO_SCRAP 的行 → IN SCRAPPED(到报废仓)
//!    - 对判定为 TO_SUPPLIER_RETURN 的行 → IN BAD(到不良仓,等后续走退供单)
//!    若某行没判定,默认作 TO_CHECK(待检,在头的 return_wh/loc)

#![deny(unsafe_code)]

pub mod repo;
pub mod service;

pub use repo::{CustomerReturnRepository, PgCustomerReturnRepository};
pub use service::{
    CreateCustomerReturnCommand, CreateCustomerReturnLine, CustomerReturnHeadView,
    CustomerReturnLineView, CustomerReturnService, JudgeLineCommand, QueryCustomerReturns,
    SubmitCustomerReturnResult,
};
