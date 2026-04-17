//! cuba-catalog
//!
//! 主数据管理:物料 / 供应商 / 客户 / BOM / 工艺路线 / 拆解模板 / 状态流转规则。
//!
//! 按实体拆成子模块:
//! - `material` — 物料主表 (`mdm_material`)
//! - `party` — 供应商 + 客户 (`mdm_supplier` / `mdm_customer`)
//! - `bom` — BOM 头/行 (`mdm_bom_h/d`)
//! - `route` — 工艺路线头/行 (`mdm_route_h/d`)
//! - `status_flow` — 状态流转规则只读查询
//!
//! 每个子模块的结构相同:`command / query / view / service / repo`。

#![deny(unsafe_code)]

pub mod application;
pub mod domain;
pub mod infrastructure;

// 便利导出
pub use application::material::{
    CreateMaterialCommand, MaterialService, MaterialView, QueryMaterials, UpdateMaterialCommand,
};
pub use application::party::{
    CreateCustomerCommand, CreateSupplierCommand, CustomerView, PartyService, QueryCustomers,
    QuerySuppliers, SupplierView, UpdateCustomerCommand, UpdateSupplierCommand,
};
pub use application::bom::{
    BomHeadView, BomLineView, BomService, CreateBomCommand, QueryBoms,
};
pub use application::route::{
    CreateRouteCommand, QueryRoutes, RouteHeadView, RouteService, RouteStepView,
};
pub use application::status_flow::{QueryStatusFlow, StatusFlowService, StatusFlowView};
