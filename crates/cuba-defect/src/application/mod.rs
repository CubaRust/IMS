//! defect 应用层

use std::sync::Arc;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::{Date, PrimitiveDateTime};
use validator::Validate;

use cuba_inventory::{CommitTxnCommand, InventoryService, TxnLineInput, TxnSideInput};
use cuba_shared::{
    audit::AuditContext,
    error::AppError,
    types::{DocStatus, IoFlag, StockStatus, TxnType},
};

use crate::domain::{is_valid_method, is_valid_source, is_valid_stage, DefectError};
use crate::infrastructure::repository::{DefectRepository, PgDefectRepository};

// -- View --------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefectHeadView {
    pub id: i64,
    pub defect_no: String,
    pub defect_source: String,
    pub work_order_no: Option<String>,
    pub process_name: Option<String>,
    pub product_stage: String,
    pub found_date: Date,
    pub finder_name: Option<String>,
    pub process_method: String,
    pub operator_id: Option<i64>,
    pub doc_status: String,
    pub remark: Option<String>,
    pub lines: Vec<DefectLineView>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefectLineView {
    pub id: i64,
    pub line_no: i32,
    pub material_id: i64,
    pub material_code: Option<String>,
    pub batch_no: String,
    pub qty: Decimal,
    pub unit: String,
    pub defect_reason: String,
    pub defect_desc: Option<String>,
    pub source_doc_type: Option<String>,
    pub source_doc_no: Option<String>,
    pub target_status: Option<String>,
    pub note: Option<String>,
}

// -- Commands ----------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateDefectCommand {
    pub defect_source: String,
    #[serde(default)]
    pub work_order_no: Option<String>,
    #[serde(default)]
    pub process_name: Option<String>,
    pub product_stage: String,
    pub found_date: Date,
    #[serde(default)]
    pub finder_name: Option<String>,
    pub process_method: String,
    #[serde(default)]
    pub remark: Option<String>,

    /// 物料当前所在仓位(submit 时需要从此处 OUT,转 BAD 仓)
    pub source_wh_id: i64,
    pub source_loc_id: i64,
    /// 转不良库(TO_BAD_STOCK)时的目标仓位;其他方式可不填
    #[serde(default)]
    pub target_wh_id: Option<i64>,
    #[serde(default)]
    pub target_loc_id: Option<i64>,

    pub lines: Vec<CreateDefectLine>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateDefectLine {
    pub line_no: i32,
    pub material_id: i64,
    #[serde(default)]
    pub batch_no: String,
    pub qty: Decimal,
    #[validate(length(min = 1, max = 30))]
    pub unit: String,
    #[validate(length(min = 1, max = 50))]
    pub defect_reason: String,
    #[serde(default)]
    pub defect_desc: Option<String>,
    #[serde(default)]
    pub source_doc_type: Option<String>,
    #[serde(default)]
    pub source_doc_no: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryDefects {
    pub defect_no: Option<String>,
    pub defect_source: Option<String>,
    pub product_stage: Option<String>,
    pub process_method: Option<String>,
    pub work_order_no: Option<String>,
    pub doc_status: Option<String>,
    pub date_from: Option<Date>,
    pub date_to: Option<Date>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubmitDefectResult {
    pub defect_id: i64,
    pub defect_no: String,
    pub txn_no: Option<String>,
    pub doc_status: String,
    pub message: String,
}

// -- Service -----------------------------------------------------------------

#[derive(Clone)]
pub struct DefectService {
    repo: Arc<dyn DefectRepository>,
    inventory: InventoryService,
}

impl DefectService {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: Arc::new(PgDefectRepository::new(pool.clone())),
            inventory: InventoryService::new(pool),
        }
    }

    pub async fn create(
        &self,
        ctx: &AuditContext,
        cmd: CreateDefectCommand,
    ) -> Result<DefectHeadView, AppError> {
        if !is_valid_source(&cmd.defect_source) {
            return Err(DefectError::invalid_source(&cmd.defect_source));
        }
        if !is_valid_stage(&cmd.product_stage) {
            return Err(DefectError::invalid_stage(&cmd.product_stage));
        }
        if !is_valid_method(&cmd.process_method) {
            return Err(DefectError::invalid_method(&cmd.process_method));
        }
        if cmd.lines.is_empty() {
            return Err(DefectError::empty_lines());
        }
        for l in &cmd.lines {
            l.validate().map_err(|e| AppError::validation(e.to_string()))?;
            if l.qty <= Decimal::ZERO {
                return Err(AppError::validation("数量必须 > 0"));
            }
        }
        self.repo.create(ctx, &cmd).await
    }

    /// 提交不良单
    ///
    /// - TO_BAD_STOCK:产生 TRANSFER OUT QUALIFIED → IN BAD
    /// - 其他 3 种:仅状态推进,不动库存(由下游单完成)
    pub async fn submit(
        &self,
        ctx: &AuditContext,
        id: i64,
    ) -> Result<SubmitDefectResult, AppError> {
        let head = self.repo.get(ctx.tenant_id, id).await?;
        let status = DocStatus::try_from(head.doc_status.as_str())?;
        if !matches!(status, DocStatus::Draft | DocStatus::Submitted) {
            return Err(DefectError::invalid_transition(&head.doc_status, "submit"));
        }

        let txn_no = if head.process_method == "TO_BAD_STOCK" {
            let (src_wh, src_loc, tgt_wh, tgt_loc) = self.repo.get_locations(ctx.tenant_id, id).await?;
            let tgt_wh = tgt_wh.ok_or_else(|| {
                AppError::validation("TO_BAD_STOCK 必须指定 target_wh_id")
            })?;
            let tgt_loc = tgt_loc.ok_or_else(|| {
                AppError::validation("TO_BAD_STOCK 必须指定 target_loc_id")
            })?;

            // TRANSFER:一行 OUT QUALIFIED + 一行 IN BAD,同批次同数量
            let mut lines = Vec::with_capacity(head.lines.len() * 2);
            let mut line_no = 1;
            for l in &head.lines {
                lines.push(TxnLineInput {
                    line_no,
                    material_id: l.material_id,
                    batch_no: l.batch_no.clone(),
                    qty: l.qty,
                    unit: l.unit.clone(),
                    io_flag: IoFlag::Out,
                    source_material_id: None,
                    target_material_id: None,
                    stock_status: Some(StockStatus::Qualified),
                    status_change_flag: true,
                    location_change_flag: true,
                    item_change_flag: false,
                    recoverable_flag: false,
                    scrap_flag: false,
                    note: Some(l.defect_reason.clone()),
                });
                line_no += 1;
                lines.push(TxnLineInput {
                    line_no,
                    material_id: l.material_id,
                    batch_no: l.batch_no.clone(),
                    qty: l.qty,
                    unit: l.unit.clone(),
                    io_flag: IoFlag::In,
                    source_material_id: None,
                    target_material_id: None,
                    stock_status: Some(StockStatus::Bad),
                    status_change_flag: true,
                    location_change_flag: true,
                    item_change_flag: false,
                    recoverable_flag: false,
                    scrap_flag: false,
                    note: Some(l.defect_reason.clone()),
                });
                line_no += 1;
            }

            let tcmd = CommitTxnCommand {
                txn_type: TxnType::Transfer,
                scene_code: "DEFECT_TO_BAD".into(),
                scene_name: Some("不良转不良库".into()),
                doc_type: "DEFECT".into(),
                doc_no: head.defect_no.clone(),
                source_object_type: None,
                source_object_id: None,
                target_object_type: None,
                target_object_id: None,
                source: Some(TxnSideInput {
                    wh_id: src_wh,
                    loc_id: src_loc,
                    status: Some(StockStatus::Qualified),
                }),
                target: Some(TxnSideInput {
                    wh_id: tgt_wh,
                    loc_id: tgt_loc,
                    status: Some(StockStatus::Bad),
                }),
                lines,
                is_exception: true,
                exception_type: Some("DEFECT".into()),
                related_doc_no: None,
                snapshot_json: Some(serde_json::json!({
                    "defect_source": head.defect_source,
                    "product_stage": head.product_stage,
                })),
                remark: head.remark.clone(),
            };
            let committed = self.inventory.commit(ctx, tcmd).await?;
            Some(committed.txn_no)
        } else {
            None
        };

        self.repo.update_status(ctx.tenant_id, head.id, DocStatus::Completed).await?;

        Ok(SubmitDefectResult {
            defect_id: head.id,
            defect_no: head.defect_no,
            txn_no,
            doc_status: DocStatus::Completed.as_str().to_string(),
            message: match head.process_method.as_str() {
                "TO_BAD_STOCK" => "库存已转入不良仓".into(),
                "TO_DISMANTLE" => "已登记,请在拆解回收单中处理".into(),
                "TO_SCRAP" => "已登记,请在报废单中处理".into(),
                "TO_REWORK" => "已登记,物料留在原仓位返工".into(),
                _ => "已完成".into(),
            },
        })
    }

    pub async fn void(&self, ctx: &AuditContext, id: i64) -> Result<(), AppError> {
        let head = self.repo.get(ctx.tenant_id, id).await?;
        let status = DocStatus::try_from(head.doc_status.as_str())?;
        if !status.can_void() {
            return Err(DefectError::invalid_transition(&head.doc_status, "void"));
        }
        self.repo.update_status(ctx.tenant_id, head.id, DocStatus::Voided).await
    }

    pub async fn get(&self, ctx: &AuditContext, id: i64) -> Result<DefectHeadView, AppError> {
        self.repo.get(ctx.tenant_id, id).await
    }

    pub async fn list(
        &self,
        ctx: &AuditContext,
        q: &QueryDefects,
    ) -> Result<Vec<DefectHeadView>, AppError> {
        self.repo.list(ctx.tenant_id, q).await
    }
}
