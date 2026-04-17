//! 盘点 service

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

use crate::repo::{PgStocktakeRepository, StocktakeRepository};

pub const ST_EMPTY: ErrorCode = ErrorCode::custom(47101);
pub const ST_INVALID_TRANSITION: ErrorCode = ErrorCode::custom(47102);
pub const ST_NOT_COUNTED: ErrorCode = ErrorCode::custom(47103);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StocktakeHeadView {
    pub id: i64,
    pub stocktake_no: String,
    pub wh_id: i64,
    pub wh_code: Option<String>,
    pub loc_id: Option<i64>,
    pub loc_code: Option<String>,
    pub stocktake_date: Date,
    pub operator_id: Option<i64>,
    pub doc_status: String,
    pub remark: Option<String>,
    pub lines: Vec<StocktakeLineView>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StocktakeLineView {
    pub id: i64,
    pub line_no: i32,
    pub material_id: i64,
    pub material_code: Option<String>,
    pub batch_no: String,
    pub stock_status: String,
    pub book_qty: Decimal,
    pub actual_qty: Option<Decimal>,
    pub diff_qty: Option<Decimal>,
    pub unit: String,
    pub counted: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateStocktakeCommand {
    pub wh_id: i64,
    #[serde(default)]
    pub loc_id: Option<i64>,
    pub stocktake_date: Date,
    #[serde(default)]
    pub remark: Option<String>,
    /// true = 从 inv.balance 快照;false = 使用给定 lines
    #[serde(default = "default_true")]
    pub snapshot_from_balance: bool,
    #[serde(default)]
    pub lines: Vec<CreateStocktakeLine>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateStocktakeLine {
    pub line_no: i32,
    pub material_id: i64,
    #[serde(default)]
    pub batch_no: String,
    pub stock_status: String,
    pub book_qty: Decimal,
    #[validate(length(min = 1, max = 30))]
    pub unit: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecordCountCommand {
    pub lines: Vec<RecordCountLine>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecordCountLine {
    pub line_id: i64,
    pub actual_qty: Decimal,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryStocktakes {
    pub stocktake_no: Option<String>,
    pub wh_id: Option<i64>,
    pub doc_status: Option<String>,
    pub date_from: Option<Date>,
    pub date_to: Option<Date>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubmitStocktakeResult {
    pub stocktake_id: i64,
    pub stocktake_no: String,
    pub gain_txn_no: Option<String>,
    pub loss_txn_no: Option<String>,
    pub doc_status: String,
    pub gain_line_count: usize,
    pub loss_line_count: usize,
    pub zero_line_count: usize,
}

#[derive(Clone)]
pub struct StocktakeService {
    repo: Arc<dyn StocktakeRepository>,
    inventory: InventoryService,
}

impl StocktakeService {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: Arc::new(PgStocktakeRepository::new(pool.clone())),
            inventory: InventoryService::new(pool),
        }
    }

    pub async fn create(
        &self,
        ctx: &AuditContext,
        cmd: CreateStocktakeCommand,
    ) -> Result<StocktakeHeadView, AppError> {
        // snapshot 模式下 lines 可空;explicit 模式 lines 必填
        if !cmd.snapshot_from_balance && cmd.lines.is_empty() {
            return Err(AppError::business(ST_EMPTY, "显式模式下盘点行不能为空"));
        }
        for l in &cmd.lines {
            l.validate().map_err(|e| AppError::validation(e.to_string()))?;
        }
        self.repo.create(ctx, &cmd).await
    }

    pub async fn record_counts(
        &self,
        _ctx: &AuditContext,
        id: i64,
        cmd: RecordCountCommand,
    ) -> Result<(), AppError> {
        self.repo.update_counts(id, &cmd.lines).await
    }

    /// 提交盘点:按差异产生 CONVERT 事务
    pub async fn submit(
        &self,
        ctx: &AuditContext,
        id: i64,
    ) -> Result<SubmitStocktakeResult, AppError> {
        let head = self.repo.get(id).await?;
        let status = DocStatus::try_from(head.doc_status.as_str())?;
        if !matches!(status, DocStatus::Draft | DocStatus::Submitted) {
            return Err(AppError::business(
                ST_INVALID_TRANSITION,
                format!("当前状态 {} 不允许 submit", head.doc_status),
            ));
        }

        // 所有行必须已点
        if head.lines.iter().any(|l| !l.counted) {
            return Err(AppError::business(
                ST_NOT_COUNTED,
                "存在未录入实盘数量的行",
            ));
        }

        let loc_id = head.loc_id.ok_or_else(|| {
            AppError::validation("盘点单缺少 loc_id,无法调整")
        })?;

        // 按差异分成 gain / loss 两组,每组一笔 IN / OUT
        let mut gain_lines: Vec<TxnLineInput> = Vec::new();
        let mut loss_lines: Vec<TxnLineInput> = Vec::new();
        let mut zero = 0usize;

        for l in &head.lines {
            let actual = l.actual_qty.unwrap_or_default();
            let diff = actual - l.book_qty;
            if diff == Decimal::ZERO {
                zero += 1;
                continue;
            }
            let stat = StockStatus::try_from(l.stock_status.as_str())?;
            if diff > Decimal::ZERO {
                gain_lines.push(TxnLineInput {
                    line_no: (gain_lines.len() as i32) + 1,
                    material_id: l.material_id,
                    batch_no: l.batch_no.clone(),
                    qty: diff,
                    unit: l.unit.clone(),
                    io_flag: IoFlag::In,
                    source_material_id: None,
                    target_material_id: None,
                    stock_status: Some(stat),
                    status_change_flag: false,
                    location_change_flag: false,
                    item_change_flag: false,
                    recoverable_flag: false,
                    scrap_flag: false,
                    note: l.note.clone(),
                });
            } else {
                // diff < 0 — 亏(OUT 绝对值)
                loss_lines.push(TxnLineInput {
                    line_no: (loss_lines.len() as i32) + 1,
                    material_id: l.material_id,
                    batch_no: l.batch_no.clone(),
                    qty: -diff,
                    unit: l.unit.clone(),
                    io_flag: IoFlag::Out,
                    source_material_id: None,
                    target_material_id: None,
                    stock_status: Some(stat),
                    status_change_flag: false,
                    location_change_flag: false,
                    item_change_flag: false,
                    recoverable_flag: false,
                    scrap_flag: false,
                    note: l.note.clone(),
                });
            }
        }

        let gain_count = gain_lines.len();
        let loss_count = loss_lines.len();

        // 盈(gain):IN scene=STOCKTAKE_GAIN
        let gain_txn_no = if !gain_lines.is_empty() {
            let cmd = CommitTxnCommand {
                txn_type: TxnType::In,
                scene_code: "STOCKTAKE_GAIN".into(),
                scene_name: Some("盘盈调整".into()),
                doc_type: "STOCKTAKE".into(),
                doc_no: head.stocktake_no.clone(),
                source_object_type: None,
                source_object_id: None,
                target_object_type: None,
                target_object_id: None,
                source: None,
                target: Some(TxnSideInput {
                    wh_id: head.wh_id,
                    loc_id,
                    status: Some(StockStatus::Qualified),
                }),
                lines: gain_lines,
                is_exception: true,
                exception_type: Some("STOCKTAKE_GAIN".into()),
                related_doc_no: None,
                snapshot_json: None,
                remark: head.remark.clone(),
            };
            Some(self.inventory.commit(ctx, cmd).await?.txn_no)
        } else {
            None
        };

        // 亏(loss):OUT scene=STOCKTAKE_LOSS
        let loss_txn_no = if !loss_lines.is_empty() {
            let cmd = CommitTxnCommand {
                txn_type: TxnType::Out,
                scene_code: "STOCKTAKE_LOSS".into(),
                scene_name: Some("盘亏调整".into()),
                doc_type: "STOCKTAKE".into(),
                doc_no: head.stocktake_no.clone(),
                source_object_type: None,
                source_object_id: None,
                target_object_type: None,
                target_object_id: None,
                source: Some(TxnSideInput {
                    wh_id: head.wh_id,
                    loc_id,
                    status: Some(StockStatus::Qualified),
                }),
                target: None,
                lines: loss_lines,
                is_exception: true,
                exception_type: Some("STOCKTAKE_LOSS".into()),
                related_doc_no: None,
                snapshot_json: None,
                remark: head.remark.clone(),
            };
            Some(self.inventory.commit(ctx, cmd).await?.txn_no)
        } else {
            None
        };

        self.repo.update_status(id, DocStatus::Completed).await?;

        Ok(SubmitStocktakeResult {
            stocktake_id: id,
            stocktake_no: head.stocktake_no,
            gain_txn_no,
            loss_txn_no,
            doc_status: DocStatus::Completed.as_str().to_string(),
            gain_line_count: gain_count,
            loss_line_count: loss_count,
            zero_line_count: zero,
        })
    }

    pub async fn void(&self, _ctx: &AuditContext, id: i64) -> Result<(), AppError> {
        let head = self.repo.get(id).await?;
        let status = DocStatus::try_from(head.doc_status.as_str())?;
        if !status.can_void() {
            return Err(AppError::business(
                ST_INVALID_TRANSITION,
                format!("当前状态 {} 不允许作废", head.doc_status),
            ));
        }
        self.repo.update_status(id, DocStatus::Voided).await
    }

    pub async fn get(&self, id: i64) -> Result<StocktakeHeadView, AppError> {
        self.repo.get(id).await
    }

    pub async fn list(&self, q: &QueryStocktakes) -> Result<Vec<StocktakeHeadView>, AppError> {
        self.repo.list(q).await
    }
}
