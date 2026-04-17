//! preissue 应用层

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
    types::{IoFlag, StockStatus, TxnType},
};

use crate::domain::PreissueError;
use crate::infrastructure::repository::{PgPreissueRepository, PreissueRepository};

// -- View --------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreissueHeadView {
    pub id: i64,
    pub preissue_no: String,
    pub exception_type: String,
    pub supplier_id: Option<i64>,
    pub supplier_name: Option<String>,
    pub work_order_no: Option<String>,
    pub process_name: Option<String>,
    pub workshop_name: Option<String>,
    pub issue_date: Date,
    pub operator_id: Option<i64>,
    pub reason: String,
    pub exception_status: String,
    pub timeout_flag: bool,
    pub expected_close_date: Option<Date>,
    pub remark: Option<String>,
    pub lines: Vec<PreissueLineView>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreissueLineView {
    pub id: i64,
    pub line_no: i32,
    pub material_id: i64,
    pub material_code: Option<String>,
    pub qty: Decimal,
    pub filled_qty: Decimal,
    pub unfilled_qty: Decimal,
    pub expected_batch_no: Option<String>,
    pub target_desc: Option<String>,
    pub line_status: String,
    pub closed_by_inbound_line_id: Option<i64>,
    pub note: Option<String>,
}

// -- Commands ----------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreatePreissueCommand {
    #[serde(default)]
    pub exception_type: Option<String>,
    #[serde(default)]
    pub supplier_id: Option<i64>,
    #[serde(default)]
    pub work_order_no: Option<String>,
    #[serde(default)]
    pub process_name: Option<String>,
    #[serde(default)]
    pub workshop_name: Option<String>,
    pub issue_date: Date,
    #[validate(length(min = 1, max = 4000))]
    pub reason: String,
    #[serde(default)]
    pub expected_close_date: Option<Date>,
    #[serde(default)]
    pub remark: Option<String>,

    /// 发料目的仓位(PREISSUE_PENDING 的定位仓位)
    pub wh_id: i64,
    pub loc_id: i64,

    pub lines: Vec<CreatePreissueLine>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreatePreissueLine {
    pub line_no: i32,
    pub material_id: i64,
    pub qty: Decimal,
    #[validate(length(min = 1, max = 30))]
    pub unit: String,
    #[serde(default)]
    pub expected_batch_no: Option<String>,
    #[serde(default)]
    pub target_desc: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryPreissues {
    pub preissue_no: Option<String>,
    pub supplier_id: Option<i64>,
    pub work_order_no: Option<String>,
    pub exception_status: Option<String>,
    pub timeout_flag: Option<bool>,
    pub date_from: Option<Date>,
    pub date_to: Option<Date>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubmitPreissueResult {
    pub preissue_id: i64,
    pub preissue_no: String,
    pub txn_no: String,
    pub exception_status: String,
}

// -- Service -----------------------------------------------------------------

#[derive(Clone)]
pub struct PreissueService {
    repo: Arc<dyn PreissueRepository>,
    inventory: InventoryService,
    pool: PgPool,
}

impl PreissueService {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: Arc::new(PgPreissueRepository::new(pool.clone())),
            inventory: InventoryService::new(pool.clone()),
            pool,
        }
    }

    /// 创建异常单并立即产生 PREISSUE_PENDING 库存动作
    ///
    /// 注意:preissue 不走"先 DRAFT 再 submit"的两步流程 —— 业务上**先发料已经是动作**,
    /// 所以创建即提交,立即写库存事务,状态直接 PENDING。
    pub async fn create_and_issue(
        &self,
        ctx: &AuditContext,
        cmd: CreatePreissueCommand,
    ) -> Result<SubmitPreissueResult, AppError> {
        cmd.validate().map_err(|e| AppError::validation(e.to_string()))?;
        if cmd.reason.trim().is_empty() {
            return Err(PreissueError::reason_required());
        }
        if cmd.lines.is_empty() {
            return Err(PreissueError::empty_lines());
        }
        for l in &cmd.lines {
            l.validate().map_err(|e| AppError::validation(e.to_string()))?;
            if l.qty <= Decimal::ZERO {
                return Err(AppError::validation("数量必须 > 0"));
            }
        }

        // 校验每个物料开启了 allow_preissue_flag
        for l in &cmd.lines {
            let allow: Option<bool> = sqlx::query_scalar(
                "select allow_preissue_flag from mdm.mdm_material where id = $1",
            )
            .bind(l.material_id)
            .fetch_optional(&self.pool)
            .await?;
            match allow {
                Some(true) => {}
                Some(false) => {
                    let code: Option<String> = sqlx::query_scalar(
                        "select material_code from mdm.mdm_material where id = $1",
                    )
                    .bind(l.material_id)
                    .fetch_optional(&self.pool)
                    .await?;
                    return Err(PreissueError::material_not_allowed(
                        &code.unwrap_or_else(|| format!("id={}", l.material_id)),
                    ));
                }
                None => return Err(AppError::not_found(format!("物料 id={} 不存在", l.material_id))),
            }
        }

        // 1. 写 preissue_h/d(DB 事务内)
        let head = self.repo.create(ctx, &cmd).await?;

        // 2. 产生 PREISSUE_PENDING 库存动作
        //    五类事务里这个是 OUT + 状态 PREISSUE_PENDING
        let mut lines = Vec::with_capacity(head.lines.len());
        for (idx, l) in head.lines.iter().enumerate() {
            lines.push(TxnLineInput {
                line_no: (idx as i32) + 1,
                material_id: l.material_id,
                batch_no: l.expected_batch_no.clone().unwrap_or_default(),
                qty: l.qty,
                unit: cmd.lines[idx].unit.clone(),
                io_flag: IoFlag::Out,
                source_material_id: None,
                target_material_id: None,
                stock_status: Some(StockStatus::PreissuePending),
                status_change_flag: false,
                location_change_flag: false,
                item_change_flag: false,
                recoverable_flag: false,
                scrap_flag: false,
                note: l.note.clone(),
            });
        }

        let source = TxnSideInput {
            wh_id: cmd.wh_id,
            loc_id: cmd.loc_id,
            status: Some(StockStatus::PreissuePending),
        };

        let tcmd = CommitTxnCommand {
            txn_type: TxnType::Out,
            scene_code: "PREISSUE_OUT".into(),
            scene_name: Some("异常先发".into()),
            doc_type: "PREISSUE".into(),
            doc_no: head.preissue_no.clone(),
            source_object_type: None,
            source_object_id: None,
            target_object_type: None,
            target_object_id: None,
            source: Some(source),
            target: None,
            lines,
            is_exception: true,
            exception_type: Some("PREISSUE".into()),
            related_doc_no: None,
            snapshot_json: Some(serde_json::json!({
                "reason": head.reason.clone(),
                "work_order_no": head.work_order_no,
                "process_name": head.process_name,
            })),
            remark: Some(format!("异常先发:{}", head.reason)),
        };

        let committed = self.inventory.commit(ctx, tcmd).await?;

        Ok(SubmitPreissueResult {
            preissue_id: head.id,
            preissue_no: head.preissue_no,
            txn_no: committed.txn_no,
            exception_status: head.exception_status,
        })
    }

    /// 冲销一行:正式入库时由 cuba-inbound 调用
    ///
    /// `filled_now` 本次入库要冲销的数量
    pub async fn close_line(
        &self,
        ctx: &AuditContext,
        preissue_line_id: i64,
        filled_now: Decimal,
        wh_id: i64,
        loc_id: i64,
    ) -> Result<(), AppError> {
        if filled_now <= Decimal::ZERO {
            return Err(AppError::validation("冲销数量必须 > 0"));
        }
        // repo 里在事务里更新 filled/unfilled + line_status + head status
        let (preissue_no, line_status, material_id, batch_no) = self
            .repo
            .apply_fill(preissue_line_id, filled_now)
            .await?;

        // 产生一笔 CONVERT:OUT PREISSUE_PENDING → IN QUALIFIED(回正)
        // 注意:inbound 本身已经入正常库存了,这里只是冲销掉 PREISSUE_PENDING 的占位
        let lines = vec![
            TxnLineInput {
                line_no: 1,
                material_id,
                batch_no: batch_no.clone(),
                qty: filled_now,
                unit: "PCS".into(), // 占位,infra 实际不校验
                io_flag: IoFlag::In,
                source_material_id: None,
                target_material_id: None,
                stock_status: Some(StockStatus::PreissuePending),
                status_change_flag: false,
                location_change_flag: false,
                item_change_flag: false,
                recoverable_flag: false,
                scrap_flag: false,
                note: Some(format!("preissue 冲销 line={preissue_line_id}")),
            },
        ];
        let side = TxnSideInput {
            wh_id,
            loc_id,
            status: Some(StockStatus::PreissuePending),
        };
        let tcmd = CommitTxnCommand {
            txn_type: TxnType::In,
            scene_code: "PREISSUE_CLOSE".into(),
            scene_name: Some("异常先发冲销".into()),
            doc_type: "PREISSUE".into(),
            doc_no: preissue_no,
            source_object_type: None,
            source_object_id: None,
            target_object_type: None,
            target_object_id: None,
            source: None,
            target: Some(side),
            lines,
            is_exception: true,
            exception_type: Some("PREISSUE_CLOSE".into()),
            related_doc_no: None,
            snapshot_json: None,
            remark: Some(format!("冲销 line={preissue_line_id} qty={filled_now}")),
        };
        self.inventory.commit(ctx, tcmd).await?;

        tracing::info!(
            preissue_line_id,
            new_line_status = %line_status,
            "preissue line closed"
        );
        Ok(())
    }

    pub async fn get(&self, id: i64) -> Result<PreissueHeadView, AppError> {
        self.repo.get(id).await
    }

    pub async fn list(&self, q: &QueryPreissues) -> Result<Vec<PreissueHeadView>, AppError> {
        self.repo.list(q).await
    }

    /// 作废(仅 PENDING 状态)
    pub async fn void(&self, _ctx: &AuditContext, id: i64) -> Result<(), AppError> {
        let head = self.repo.get(id).await?;
        if head.exception_status != "PENDING" {
            return Err(PreissueError::status_mismatch(&head.exception_status, "void"));
        }
        self.repo.update_head_status(id, "VOIDED").await
    }
}
