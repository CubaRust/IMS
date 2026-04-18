//! recovery application

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

use crate::domain::RecoveryError;
use crate::infrastructure::repository::{PgRecoveryRepository, RecoveryRepository};

// -- View --------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryHeadView {
    pub id: i64,
    pub recovery_no: String,
    pub source_defect_id: i64,
    pub source_defect_no: Option<String>,
    pub tpl_id: Option<i64>,
    pub recovery_date: Date,
    pub operator_id: Option<i64>,
    pub doc_status: String,
    pub remark: Option<String>,
    pub inputs: Vec<RecoveryInView>,
    pub outputs: Vec<RecoveryOutView>,
    pub scraps: Vec<RecoveryScrapView>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryInView {
    pub id: i64,
    pub line_no: i32,
    pub material_id: i64,
    pub material_code: Option<String>,
    pub batch_no: String,
    pub qty: Decimal,
    pub unit: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryOutView {
    pub id: i64,
    pub line_no: i32,
    pub material_id: i64,
    pub material_code: Option<String>,
    pub qty: Decimal,
    pub unit: String,
    pub target_wh_id: Option<i64>,
    pub target_loc_id: Option<i64>,
    pub target_status: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryScrapView {
    pub id: i64,
    pub line_no: i32,
    pub material_id: Option<i64>,
    pub material_code: Option<String>,
    pub qty: Decimal,
    pub unit: String,
    pub scrap_reason: Option<String>,
    pub note: Option<String>,
}

// -- Commands ----------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateRecoveryCommand {
    pub source_defect_id: i64,
    #[serde(default)]
    pub tpl_id: Option<i64>,
    pub recovery_date: Date,
    #[serde(default)]
    pub remark: Option<String>,

    /// 拆解发生的源仓位(NG 品所在位置,通常是不良仓 BAD01)
    pub source_wh_id: i64,
    pub source_loc_id: i64,
    /// 报废碎料的目的仓位(SCRAP01)
    pub scrap_wh_id: i64,
    pub scrap_loc_id: i64,

    pub inputs: Vec<CreateRecoveryIn>,
    #[serde(default)]
    pub outputs: Vec<CreateRecoveryOut>,
    #[serde(default)]
    pub scraps: Vec<CreateRecoveryScrap>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateRecoveryIn {
    pub line_no: i32,
    pub material_id: i64,
    #[serde(default)]
    pub batch_no: String,
    pub qty: Decimal,
    #[validate(length(min = 1, max = 30))]
    pub unit: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateRecoveryOut {
    pub line_no: i32,
    pub material_id: i64,
    pub qty: Decimal,
    #[validate(length(min = 1, max = 30))]
    pub unit: String,
    pub target_wh_id: i64,
    pub target_loc_id: i64,
    #[serde(default = "default_qualified")]
    pub target_status: String,
    #[serde(default)]
    pub note: Option<String>,
}

fn default_qualified() -> String {
    "QUALIFIED".into()
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateRecoveryScrap {
    pub line_no: i32,
    #[serde(default)]
    pub material_id: Option<i64>,
    pub qty: Decimal,
    #[validate(length(min = 1, max = 30))]
    pub unit: String,
    #[serde(default)]
    pub scrap_reason: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryRecoveries {
    pub recovery_no: Option<String>,
    pub source_defect_id: Option<i64>,
    pub doc_status: Option<String>,
    pub date_from: Option<Date>,
    pub date_to: Option<Date>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubmitRecoveryResult {
    pub recovery_id: i64,
    pub recovery_no: String,
    pub txn_no: String,
    pub doc_status: String,
}

// -- Service -----------------------------------------------------------------

#[derive(Clone)]
pub struct RecoveryService {
    repo: Arc<dyn RecoveryRepository>,
    inventory: InventoryService,
}

impl RecoveryService {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: Arc::new(PgRecoveryRepository::new(pool.clone())),
            inventory: InventoryService::new(pool),
        }
    }

    pub async fn create(
        &self,
        ctx: &AuditContext,
        cmd: CreateRecoveryCommand,
    ) -> Result<RecoveryHeadView, AppError> {
        if cmd.inputs.is_empty() {
            return Err(RecoveryError::empty_in());
        }
        for l in &cmd.inputs {
            l.validate().map_err(|e| AppError::validation(e.to_string()))?;
        }
        for l in &cmd.outputs {
            l.validate().map_err(|e| AppError::validation(e.to_string()))?;
        }
        for l in &cmd.scraps {
            l.validate().map_err(|e| AppError::validation(e.to_string()))?;
        }
        self.repo.create(ctx, &cmd).await
    }

    /// 提交 → CONVERT:输入 OUT BAD,输出 IN QUALIFIED,报废 IN SCRAPPED
    pub async fn submit(
        &self,
        ctx: &AuditContext,
        id: i64,
    ) -> Result<SubmitRecoveryResult, AppError> {
        let head = self.repo.get(ctx.tenant_id, id).await?;
        let status = DocStatus::try_from(head.doc_status.as_str())?;
        if !matches!(status, DocStatus::Draft | DocStatus::Submitted) {
            return Err(RecoveryError::invalid_transition(&head.doc_status, "submit"));
        }

        let (src_wh, src_loc, scrap_wh, scrap_loc) = self.repo.get_locations(ctx.tenant_id, id).await?;

        // CONVERT 里要求:至少一条 OUT + 至少一条 IN;这里我们批量构造
        let mut lines: Vec<TxnLineInput> = Vec::new();
        let mut line_no: i32 = 1;

        // 1. 输入 OUT (源物料 BAD)
        for l in &head.inputs {
            lines.push(TxnLineInput {
                line_no,
                material_id: l.material_id,
                batch_no: l.batch_no.clone(),
                qty: l.qty,
                unit: l.unit.clone(),
                io_flag: IoFlag::Out,
                source_material_id: Some(l.material_id),
                target_material_id: None,
                stock_status: Some(StockStatus::Bad),
                status_change_flag: true,
                location_change_flag: false,
                item_change_flag: true,
                recoverable_flag: true,
                scrap_flag: false,
                note: l.note.clone(),
            });
            line_no += 1;
        }

        // 2. 可回收件 IN 到目标仓位(每行自己的 target_wh/loc,
        //    但 inventory.compute_deltas 会按 head.target 算仓位,这里我们用 CONVERT 走
        //    head.source / head.target 都指向源,各输出行的定位**会按 head.target 决定**
        //    -> 所以我们这里简化:如果 outputs 的 target_wh/loc 跟 head 不同,会走不到正确仓位
        //    推荐 UI 侧收敛:所有 outputs 用同一个目标仓位
        //
        //    TODO(后续):允许按行覆盖仓位。当前实现按第一条 output 作为统一 target。
        let (out_wh, out_loc, out_status) = head
            .outputs
            .first()
            .map(|o| {
                (
                    o.target_wh_id.unwrap_or(src_wh),
                    o.target_loc_id.unwrap_or(src_loc),
                    StockStatus::try_from(o.target_status.as_str()).unwrap_or(StockStatus::Qualified),
                )
            })
            .unwrap_or((src_wh, src_loc, StockStatus::Qualified));

        for l in &head.outputs {
            if l.qty <= Decimal::ZERO {
                continue;
            }
            let stat = StockStatus::try_from(l.target_status.as_str())
                .unwrap_or(StockStatus::Qualified);
            lines.push(TxnLineInput {
                line_no,
                material_id: l.material_id,
                batch_no: String::new(),
                qty: l.qty,
                unit: l.unit.clone(),
                io_flag: IoFlag::In,
                source_material_id: None,
                target_material_id: Some(l.material_id),
                stock_status: Some(stat),
                status_change_flag: true,
                location_change_flag: false,
                item_change_flag: true,
                recoverable_flag: true,
                scrap_flag: false,
                note: l.note.clone(),
            });
            line_no += 1;
        }

        // 3. 报废碎料 IN 到报废仓
        //    这一组用独立 CONVERT 走更好,但 inventory 事务模型是一个 head,我们先塞一起
        //    (对 book 是:BAD-1;RECOVERY+1;SCRAPPED+余料。余额表按 locator 聚合,正常)
        for l in &head.scraps {
            if l.qty <= Decimal::ZERO {
                continue;
            }
            let Some(mid) = l.material_id else { continue };
            lines.push(TxnLineInput {
                line_no,
                material_id: mid,
                batch_no: String::new(),
                qty: l.qty,
                unit: l.unit.clone(),
                io_flag: IoFlag::In,
                source_material_id: None,
                target_material_id: Some(mid),
                stock_status: Some(StockStatus::Scrapped),
                status_change_flag: true,
                location_change_flag: true,
                item_change_flag: true,
                recoverable_flag: false,
                scrap_flag: true,
                note: l.note.clone(),
            });
            line_no += 1;
        }

        // CONVERT 需要 source + target(rules 要求),简单起见都指向第一个可回收件的位置
        let source = TxnSideInput {
            wh_id: src_wh,
            loc_id: src_loc,
            status: Some(StockStatus::Bad),
        };
        let target = TxnSideInput {
            wh_id: out_wh,
            loc_id: out_loc,
            status: Some(out_status),
        };

        let tcmd = CommitTxnCommand {
            txn_type: TxnType::Convert,
            scene_code: "DISMANTLE".into(),
            scene_name: Some("拆解回收".into()),
            doc_type: "RECOVERY".into(),
            doc_no: head.recovery_no.clone(),
            source_object_type: Some("DEFECT".into()),
            source_object_id: Some(head.source_defect_id),
            target_object_type: None,
            target_object_id: None,
            source: Some(source),
            target: Some(target),
            lines,
            is_exception: true,
            exception_type: Some("RECOVERY".into()),
            related_doc_no: head.source_defect_no.clone(),
            snapshot_json: Some(serde_json::json!({
                "scrap_wh_id": scrap_wh,
                "scrap_loc_id": scrap_loc,
            })),
            remark: head.remark.clone(),
        };
        let committed = self.inventory.commit(ctx, tcmd).await?;

        self.repo.update_status(ctx.tenant_id, head.id, DocStatus::Completed).await?;

        Ok(SubmitRecoveryResult {
            recovery_id: head.id,
            recovery_no: head.recovery_no,
            txn_no: committed.txn_no,
            doc_status: DocStatus::Completed.as_str().to_string(),
        })
    }

    pub async fn void(&self, ctx: &AuditContext, id: i64) -> Result<(), AppError> {
        let head = self.repo.get(ctx.tenant_id, id).await?;
        let status = DocStatus::try_from(head.doc_status.as_str())?;
        if !status.can_void() {
            return Err(RecoveryError::invalid_transition(&head.doc_status, "void"));
        }
        self.repo.update_status(ctx.tenant_id, head.id, DocStatus::Voided).await
    }

    pub async fn get(&self, ctx: &AuditContext, id: i64) -> Result<RecoveryHeadView, AppError> {
        self.repo.get(ctx.tenant_id, id).await
    }

    pub async fn list(
        &self,
        ctx: &AuditContext,
        q: &QueryRecoveries,
    ) -> Result<Vec<RecoveryHeadView>, AppError> {
        self.repo.list(ctx.tenant_id, q).await
    }
}
