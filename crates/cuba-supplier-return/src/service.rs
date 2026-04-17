//! 供应商退货 service

use std::sync::Arc;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::{Date, PrimitiveDateTime};
use validator::Validate;

use cuba_inventory::{CommitTxnCommand, InventoryService, TxnLineInput, TxnSideInput};
use cuba_shared::{
    audit::AuditContext,
    error::{AppError, ErrorCode},
    types::{DocStatus, IoFlag, StockStatus, TxnType},
};

use crate::repo::{PgSupplierReturnRepository, SupplierReturnRepository};

pub const SR_EMPTY_LINES: ErrorCode = ErrorCode::custom(45101);
pub const SR_INVALID_TRANSITION: ErrorCode = ErrorCode::custom(45102);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplierReturnHeadView {
    pub id: i64,
    pub return_no: String,
    pub supplier_id: i64,
    pub supplier_name: Option<String>,
    pub return_date: Date,
    pub original_doc_no: Option<String>,
    pub operator_id: Option<i64>,
    pub doc_status: String,
    pub remark: Option<String>,
    pub lines: Vec<SupplierReturnLineView>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplierReturnLineView {
    pub id: i64,
    pub line_no: i32,
    pub material_id: i64,
    pub material_code: Option<String>,
    pub batch_no: String,
    pub qty: Decimal,
    pub unit: String,
    pub source_status: String,
    pub return_reason: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateSupplierReturnCommand {
    pub supplier_id: i64,
    pub return_date: Date,
    #[serde(default)]
    pub original_doc_no: Option<String>,
    #[serde(default)]
    pub remark: Option<String>,
    /// 发出物料的源仓位
    pub source_wh_id: i64,
    pub source_loc_id: i64,
    pub lines: Vec<CreateSupplierReturnLine>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateSupplierReturnLine {
    pub line_no: i32,
    pub material_id: i64,
    #[serde(default)]
    pub batch_no: String,
    pub qty: Decimal,
    #[validate(length(min = 1, max = 30))]
    pub unit: String,
    /// 通常 "BAD",来料不良才退供
    #[serde(default = "default_bad")]
    pub source_status: String,
    #[validate(length(min = 1, max = 200))]
    pub return_reason: String,
    #[serde(default)]
    pub note: Option<String>,
}

fn default_bad() -> String {
    "BAD".into()
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QuerySupplierReturns {
    pub return_no: Option<String>,
    pub supplier_id: Option<i64>,
    pub doc_status: Option<String>,
    pub date_from: Option<Date>,
    pub date_to: Option<Date>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubmitSupplierReturnResult {
    pub return_id: i64,
    pub return_no: String,
    pub txn_no: String,
    pub doc_status: String,
}

#[derive(Clone)]
pub struct SupplierReturnService {
    repo: Arc<dyn SupplierReturnRepository>,
    inventory: InventoryService,
}

impl SupplierReturnService {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: Arc::new(PgSupplierReturnRepository::new(pool.clone())),
            inventory: InventoryService::new(pool),
        }
    }

    pub async fn create(
        &self,
        ctx: &AuditContext,
        cmd: CreateSupplierReturnCommand,
    ) -> Result<SupplierReturnHeadView, AppError> {
        if cmd.lines.is_empty() {
            return Err(AppError::business(SR_EMPTY_LINES, "退供单行不能为空"));
        }
        for l in &cmd.lines {
            l.validate().map_err(|e| AppError::validation(e.to_string()))?;
            if l.qty <= Decimal::ZERO {
                return Err(AppError::validation("数量必须 > 0"));
            }
        }
        self.repo.create(ctx, &cmd).await
    }

    pub async fn submit(
        &self,
        ctx: &AuditContext,
        id: i64,
    ) -> Result<SubmitSupplierReturnResult, AppError> {
        let head = self.repo.get(id).await?;
        let status = DocStatus::try_from(head.doc_status.as_str())?;
        if !matches!(status, DocStatus::Draft | DocStatus::Submitted) {
            return Err(AppError::business(
                SR_INVALID_TRANSITION,
                format!("当前状态 {} 不允许 submit", head.doc_status),
            ));
        }

        let (src_wh, src_loc) = self.repo.get_source_location(id).await?;

        // 按源状态分组(多数情况所有行同状态)
        use std::collections::HashMap;
        let mut groups: HashMap<String, Vec<&SupplierReturnLineView>> = HashMap::new();
        for l in &head.lines {
            groups.entry(l.source_status.clone()).or_default().push(l);
        }

        let mut last_txn_no = String::new();
        for (source_status_key, lines) in groups {
            let src_stat = StockStatus::try_from(source_status_key.as_str())?;
            let mut txn_lines = Vec::with_capacity(lines.len());
            for (i, l) in lines.iter().enumerate() {
                txn_lines.push(TxnLineInput {
                    line_no: (i as i32) + 1,
                    material_id: l.material_id,
                    batch_no: l.batch_no.clone(),
                    qty: l.qty,
                    unit: l.unit.clone(),
                    io_flag: IoFlag::Out,
                    source_material_id: None,
                    target_material_id: None,
                    stock_status: Some(src_stat),
                    status_change_flag: false,
                    location_change_flag: false,
                    item_change_flag: false,
                    recoverable_flag: false,
                    scrap_flag: false,
                    note: Some(l.return_reason.clone()),
                });
            }

            let tcmd = CommitTxnCommand {
                txn_type: TxnType::Out,
                scene_code: "SUPPLIER_RETURN_OUT".into(),
                scene_name: Some("退供出库".into()),
                doc_type: "SUPPLIER_RETURN".into(),
                doc_no: head.return_no.clone(),
                source_object_type: Some("SUPPLIER".into()),
                source_object_id: Some(head.supplier_id),
                target_object_type: None,
                target_object_id: None,
                source: Some(TxnSideInput {
                    wh_id: src_wh,
                    loc_id: src_loc,
                    status: Some(src_stat),
                }),
                target: None,
                lines: txn_lines,
                is_exception: true,
                exception_type: Some("SUPPLIER_RETURN".into()),
                related_doc_no: head.original_doc_no.clone(),
                snapshot_json: None,
                remark: head.remark.clone(),
            };
            let committed = self.inventory.commit(ctx, tcmd).await?;
            last_txn_no = committed.txn_no;
        }

        self.repo.update_status(id, DocStatus::Completed).await?;
        Ok(SubmitSupplierReturnResult {
            return_id: id,
            return_no: head.return_no,
            txn_no: last_txn_no,
            doc_status: DocStatus::Completed.as_str().to_string(),
        })
    }

    pub async fn void(&self, _ctx: &AuditContext, id: i64) -> Result<(), AppError> {
        let head = self.repo.get(id).await?;
        let status = DocStatus::try_from(head.doc_status.as_str())?;
        if !status.can_void() {
            return Err(AppError::business(
                SR_INVALID_TRANSITION,
                format!("当前状态 {} 不允许作废", head.doc_status),
            ));
        }
        self.repo.update_status(id, DocStatus::Voided).await
    }

    pub async fn get(&self, id: i64) -> Result<SupplierReturnHeadView, AppError> {
        self.repo.get(id).await
    }

    pub async fn list(
        &self,
        q: &QuerySupplierReturns,
    ) -> Result<Vec<SupplierReturnHeadView>, AppError> {
        self.repo.list(q).await
    }
}
