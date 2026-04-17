//! outbound 应用层

use std::sync::Arc;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::{Date, PrimitiveDateTime};
use validator::Validate;

use cuba_inventory::{
    CommitTxnCommand, InventoryService, TxnLineInput, TxnSideInput,
};
use cuba_shared::{
    audit::AuditContext,
    error::AppError,
    types::{DocStatus, IoFlag, StockStatus, TxnType},
};

use crate::domain::{
    is_valid_outbound_type, requires_work_order, scene_code_for, OutboundError,
};
use crate::infrastructure::repository::{OutboundRepository, PgOutboundRepository};

// -- View --------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundHeadView {
    pub id: i64,
    pub outbound_no: String,
    pub outbound_type: String,
    pub target_object_type: Option<String>,
    pub target_object_id: Option<i64>,
    pub work_order_no: Option<String>,
    pub process_name: Option<String>,
    pub route_id: Option<i64>,
    pub workshop_name: Option<String>,
    pub wh_id: i64,
    pub wh_code: Option<String>,
    pub outbound_date: Date,
    pub operator_id: Option<i64>,
    pub doc_status: String,
    pub remark: Option<String>,
    pub lines: Vec<OutboundLineView>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundLineView {
    pub id: i64,
    pub line_no: i32,
    pub material_id: i64,
    pub material_code: Option<String>,
    pub batch_no: String,
    pub suggest_qty: Decimal,
    pub actual_qty: Decimal,
    pub unit: String,
    pub stock_status: String,
    pub bom_recommended_flag: bool,
    pub public_material_flag: bool,
    pub preissue_flag: bool,
    pub note: Option<String>,
    /// 出库时的源仓位(由 head.wh_id + 此处 loc_id 决定)
    pub loc_id: Option<i64>,
}

// -- Commands ----------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateOutboundCommand {
    pub outbound_type: String,
    #[serde(default)]
    pub target_object_type: Option<String>,
    #[serde(default)]
    pub target_object_id: Option<i64>,
    #[serde(default)]
    pub work_order_no: Option<String>,
    #[serde(default)]
    pub process_name: Option<String>,
    #[serde(default)]
    pub route_id: Option<i64>,
    #[serde(default)]
    pub workshop_name: Option<String>,
    pub wh_id: i64,
    /// 源仓位(单据级)
    pub loc_id: i64,
    pub outbound_date: Date,
    #[serde(default)]
    pub remark: Option<String>,
    pub lines: Vec<CreateOutboundLine>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateOutboundLine {
    pub line_no: i32,
    pub material_id: i64,
    #[serde(default)]
    pub batch_no: String,
    #[serde(default)]
    pub suggest_qty: Decimal,
    pub actual_qty: Decimal,
    #[validate(length(min = 1, max = 30))]
    pub unit: String,
    #[serde(default = "default_qualified")]
    pub stock_status: String,
    #[serde(default)]
    pub bom_recommended_flag: bool,
    #[serde(default)]
    pub public_material_flag: bool,
    #[serde(default)]
    pub preissue_flag: bool,
    #[serde(default)]
    pub note: Option<String>,
}

fn default_qualified() -> String {
    "QUALIFIED".into()
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryOutbounds {
    pub outbound_no: Option<String>,
    pub outbound_type: Option<String>,
    pub work_order_no: Option<String>,
    pub doc_status: Option<String>,
    pub date_from: Option<Date>,
    pub date_to: Option<Date>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubmitOutboundResult {
    pub outbound_id: i64,
    pub outbound_no: String,
    pub txn_no: String,
    pub doc_status: String,
}

// -- Service -----------------------------------------------------------------

#[derive(Clone)]
pub struct OutboundService {
    repo: Arc<dyn OutboundRepository>,
    inventory: InventoryService,
    pool: PgPool,
}

impl OutboundService {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: Arc::new(PgOutboundRepository::new(pool.clone())),
            inventory: InventoryService::new(pool.clone()),
            pool,
        }
    }

    pub async fn create(
        &self,
        ctx: &AuditContext,
        cmd: CreateOutboundCommand,
    ) -> Result<OutboundHeadView, AppError> {
        if !is_valid_outbound_type(&cmd.outbound_type) {
            return Err(OutboundError::invalid_type(&cmd.outbound_type));
        }
        if cmd.lines.is_empty() {
            return Err(OutboundError::empty_lines());
        }
        if requires_work_order(&cmd.outbound_type) && cmd.work_order_no.is_none() {
            return Err(OutboundError::workorder_required());
        }
        for l in &cmd.lines {
            l.validate().map_err(|e| AppError::validation(e.to_string()))?;
            if l.actual_qty <= Decimal::ZERO {
                return Err(AppError::validation("实发数量必须 > 0"));
            }
        }
        self.repo.create(ctx, &cmd).await
    }

    /// 提交 → 调 inventory 扣减
    pub async fn submit(
        &self,
        ctx: &AuditContext,
        id: i64,
    ) -> Result<SubmitOutboundResult, AppError> {
        let head = self.repo.get(id).await?;
        let status = DocStatus::try_from(head.doc_status.as_str())?;
        if !matches!(status, DocStatus::Draft | DocStatus::Submitted) {
            return Err(OutboundError::invalid_transition(&head.doc_status, "submit"));
        }

        let scene = scene_code_for(&head.outbound_type);

        let mut lines = Vec::with_capacity(head.lines.len());
        for (idx, l) in head.lines.iter().enumerate() {
            let stock_status = StockStatus::try_from(l.stock_status.as_str())?;
            lines.push(TxnLineInput {
                line_no: (idx as i32) + 1,
                material_id: l.material_id,
                batch_no: l.batch_no.clone(),
                qty: l.actual_qty,
                unit: l.unit.clone(),
                io_flag: IoFlag::Out,
                source_material_id: None,
                target_material_id: None,
                stock_status: Some(stock_status),
                status_change_flag: false,
                location_change_flag: false,
                item_change_flag: false,
                recoverable_flag: false,
                scrap_flag: false,
                note: l.note.clone(),
            });
        }

        // 读头上的 wh_id + loc_id(需额外查;这里简化用同一个 loc 发所有行,
        // 进阶:每行可覆写自己的 loc_id)
        let loc_id: i64 = sqlx::query_scalar("select loc_id from wms.wms_outbound_h where id = $1")
            .bind(head.id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::not_found(format!("出库单 id={} 不存在", head.id)))?;

        let source = TxnSideInput {
            wh_id: head.wh_id,
            loc_id,
            status: Some(StockStatus::Qualified),
        };

        let cmd = CommitTxnCommand {
            txn_type: TxnType::Out,
            scene_code: scene.to_string(),
            scene_name: None,
            doc_type: "OUTBOUND".to_string(),
            doc_no: head.outbound_no.clone(),
            source_object_type: head.target_object_type.clone(),
            source_object_id: head.target_object_id,
            target_object_type: None,
            target_object_id: None,
            source: Some(source),
            target: None,
            lines,
            is_exception: false,
            exception_type: None,
            related_doc_no: None,
            snapshot_json: None,
            remark: head.remark.clone(),
        };

        let committed = self.inventory.commit(ctx, cmd).await?;

        self.repo
            .update_status(head.id, DocStatus::Completed)
            .await?;

        Ok(SubmitOutboundResult {
            outbound_id: head.id,
            outbound_no: head.outbound_no,
            txn_no: committed.txn_no,
            doc_status: DocStatus::Completed.as_str().to_string(),
        })
    }

    pub async fn void(&self, _ctx: &AuditContext, id: i64) -> Result<(), AppError> {
        let head = self.repo.get(id).await?;
        let status = DocStatus::try_from(head.doc_status.as_str())?;
        if !status.can_void() {
            return Err(OutboundError::invalid_transition(&head.doc_status, "void"));
        }
        self.repo.update_status(head.id, DocStatus::Voided).await
    }

    pub async fn get(&self, id: i64) -> Result<OutboundHeadView, AppError> {
        self.repo.get(id).await
    }

    pub async fn list(&self, q: &QueryOutbounds) -> Result<Vec<OutboundHeadView>, AppError> {
        self.repo.list(q).await
    }
}
