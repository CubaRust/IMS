//! inbound 应用层

use std::sync::Arc;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::{Date, PrimitiveDateTime};
use validator::Validate;

use cuba_inventory::{CommitTxnCommand, InventoryService, TxnLineInput, TxnSideInput};
use cuba_preissue::PreissueService;
use cuba_shared::{
    audit::AuditContext,
    error::AppError,
    types::{DocStatus, IoFlag, StockStatus, TxnType},
};

use crate::domain::{default_target_status, is_valid_inbound_type, scene_code_for, InboundError};
use crate::infrastructure::repository::{InboundRepository, PgInboundRepository};

// -- View --------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundHeadView {
    pub id: i64,
    pub inbound_no: String,
    pub inbound_type: String,
    pub supplier_id: Option<i64>,
    pub supplier_name: Option<String>,
    pub wh_id: i64,
    pub wh_code: Option<String>,
    pub loc_id: Option<i64>,
    pub loc_code: Option<String>,
    pub inbound_date: Date,
    pub operator_id: Option<i64>,
    pub doc_status: String,
    pub remark: Option<String>,
    pub lines: Vec<InboundLineView>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundLineView {
    pub id: i64,
    pub line_no: i32,
    pub material_id: i64,
    pub material_code: Option<String>,
    pub batch_no: String,
    pub qty: Decimal,
    pub unit: String,
    pub stock_status: String,
    pub work_order_no: Option<String>,
    pub process_name: Option<String>,
    pub outsource_no: Option<String>,
    pub related_preissue_line_id: Option<i64>,
    pub note: Option<String>,
}

// -- Commands ----------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateInboundCommand {
    pub inbound_type: String,
    #[serde(default)]
    pub supplier_id: Option<i64>,
    #[serde(default)]
    pub source_object_type: Option<String>,
    #[serde(default)]
    pub source_object_id: Option<i64>,
    pub wh_id: i64,
    #[serde(default)]
    pub loc_id: Option<i64>,
    pub inbound_date: Date,
    #[serde(default)]
    pub remark: Option<String>,
    pub lines: Vec<CreateInboundLine>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateInboundLine {
    pub line_no: i32,
    pub material_id: i64,
    #[serde(default)]
    pub batch_no: String,
    pub qty: Decimal,
    #[validate(length(min = 1, max = 30))]
    pub unit: String,
    /// 可选:覆盖默认状态
    #[serde(default)]
    pub stock_status: Option<String>,
    #[serde(default)]
    pub work_order_no: Option<String>,
    #[serde(default)]
    pub process_name: Option<String>,
    #[serde(default)]
    pub outsource_no: Option<String>,
    #[serde(default)]
    pub related_preissue_line_id: Option<i64>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryInbounds {
    pub inbound_no: Option<String>,
    pub inbound_type: Option<String>,
    pub supplier_id: Option<i64>,
    pub doc_status: Option<String>,
    pub date_from: Option<Date>,
    pub date_to: Option<Date>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubmitInboundResult {
    pub inbound_id: i64,
    pub inbound_no: String,
    pub txn_no: String,
    pub doc_status: String,
}

// -- Service -----------------------------------------------------------------

#[derive(Clone)]
pub struct InboundService {
    repo: Arc<dyn InboundRepository>,
    inventory: InventoryService,
    preissue: PreissueService,
    pool_for_events: PgPool,
}

impl InboundService {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: Arc::new(PgInboundRepository::new(pool.clone())),
            inventory: InventoryService::new(pool.clone()),
            preissue: PreissueService::new(pool.clone()),
            pool_for_events: pool,
        }
    }

    /// 建 DRAFT 单
    pub async fn create(
        &self,
        ctx: &AuditContext,
        cmd: CreateInboundCommand,
    ) -> Result<InboundHeadView, AppError> {
        if !is_valid_inbound_type(&cmd.inbound_type) {
            return Err(InboundError::invalid_type(&cmd.inbound_type));
        }
        if cmd.lines.is_empty() {
            return Err(InboundError::empty_lines());
        }
        for l in &cmd.lines {
            l.validate()
                .map_err(|e| AppError::validation(e.to_string()))?;
            if l.qty <= Decimal::ZERO {
                return Err(AppError::validation("数量必须 > 0"));
            }
        }
        self.repo.create(ctx, &cmd).await
    }

    /// 提交 → 库存 + 状态转 COMPLETED
    pub async fn submit(
        &self,
        ctx: &AuditContext,
        id: i64,
    ) -> Result<SubmitInboundResult, AppError> {
        let head = self.repo.get(ctx.tenant_id, id).await?;
        let status = DocStatus::try_from(head.doc_status.as_str())?;
        if !matches!(status, DocStatus::Draft | DocStatus::Submitted) {
            return Err(InboundError::invalid_transition(&head.doc_status, "submit"));
        }

        // 构造库存 IN 事务
        let scene = scene_code_for(&head.inbound_type);
        let target_status_default = default_target_status(&head.inbound_type);

        let mut lines = Vec::with_capacity(head.lines.len());
        for (idx, l) in head.lines.iter().enumerate() {
            let status_str = if l.stock_status.is_empty() {
                target_status_default.to_string()
            } else {
                l.stock_status.clone()
            };
            let stock_status = StockStatus::try_from(status_str.as_str())?;
            lines.push(TxnLineInput {
                line_no: (idx as i32) + 1,
                material_id: l.material_id,
                batch_no: l.batch_no.clone(),
                qty: l.qty,
                unit: l.unit.clone(),
                io_flag: IoFlag::In,
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

        // 推断 target 仓位
        let target = TxnSideInput {
            wh_id: head.wh_id,
            loc_id: head
                .loc_id
                .ok_or_else(|| AppError::validation("入库单未指定 loc_id,无法提交到库存"))?,
            status: Some(StockStatus::try_from(target_status_default)?),
        };

        let cmd = CommitTxnCommand {
            txn_type: TxnType::In,
            scene_code: scene.to_string(),
            scene_name: None,
            doc_type: "INBOUND".to_string(),
            doc_no: head.inbound_no.clone(),
            source_object_type: None,
            source_object_id: None,
            target_object_type: None,
            target_object_id: None,
            source: None,
            target: Some(target),
            lines,
            is_exception: false,
            exception_type: None,
            related_doc_no: None,
            snapshot_json: None,
            remark: head.remark.clone(),
        };

        let committed = self.inventory.commit(ctx, cmd).await?;

        // 异常先发自动冲销:对每行带 related_preissue_line_id 的,调 preissue.close_line
        let target_loc_for_close = head.loc_id.unwrap_or_default();
        for l in &head.lines {
            if let Some(preissue_line_id) = l.related_preissue_line_id {
                self.preissue
                    .close_line(
                        ctx,
                        preissue_line_id,
                        l.qty,
                        head.wh_id,
                        target_loc_for_close,
                    )
                    .await?;
            }
        }

        // 单据状态推进
        self.repo
            .update_status(ctx.tenant_id, head.id, DocStatus::Completed)
            .await?;

        // 写领域事件(聚合根层事件,outbox publisher 可用于对外推送)
        let ev_ctx = cuba_events::WriteEventCtx::from(ctx);
        let _ = cuba_events::write_event(
            &self.pool_for_events,
            &ev_ctx,
            &cuba_events::DomainEvent::InboundSubmitted {
                inbound_id: head.id,
                inbound_no: head.inbound_no.clone(),
                inbound_type: head.inbound_type.clone(),
                wh_id: head.wh_id,
                txn_no: committed.txn_no.clone(),
            },
        )
        .await;

        Ok(SubmitInboundResult {
            inbound_id: head.id,
            inbound_no: head.inbound_no,
            txn_no: committed.txn_no,
            doc_status: DocStatus::Completed.as_str().to_string(),
        })
    }

    /// 作废(仅 DRAFT / SUBMITTED)
    pub async fn void(&self, ctx: &AuditContext, id: i64) -> Result<(), AppError> {
        let head = self.repo.get(ctx.tenant_id, id).await?;
        let status = DocStatus::try_from(head.doc_status.as_str())?;
        if !status.can_void() {
            return Err(InboundError::invalid_transition(&head.doc_status, "void"));
        }
        self.repo
            .update_status(ctx.tenant_id, head.id, DocStatus::Voided)
            .await
    }

    pub async fn get(&self, ctx: &AuditContext, id: i64) -> Result<InboundHeadView, AppError> {
        self.repo.get(ctx.tenant_id, id).await
    }

    pub async fn list(
        &self,
        ctx: &AuditContext,
        q: &QueryInbounds,
    ) -> Result<Vec<InboundHeadView>, AppError> {
        self.repo.list(ctx.tenant_id, q).await
    }
}
