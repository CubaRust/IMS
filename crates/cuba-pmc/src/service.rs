//! PMC 委外 service

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

use crate::repo::{PgPmcRepository, PmcRepository};

pub const PMC_EMPTY: ErrorCode = ErrorCode::custom(46101);
pub const PMC_INVALID_TRANSITION: ErrorCode = ErrorCode::custom(46102);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutsourceHeadView {
    pub id: i64,
    pub outsource_no: String,
    pub supplier_id: i64,
    pub supplier_name: Option<String>,
    pub issue_date: Date,
    pub operator_id: Option<i64>,
    pub doc_status: String,
    pub send_status: String, // DRAFT / SENT
    pub back_status: String, // DRAFT / PARTIAL / COMPLETED
    pub remark: Option<String>,
    pub send_lines: Vec<OutsourceLineView>,
    pub back_lines: Vec<OutsourceLineView>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutsourceLineView {
    pub id: i64,
    pub line_no: i32,
    pub material_id: i64,
    pub material_code: Option<String>,
    pub batch_no: String,
    pub qty: Decimal,
    pub actual_qty: Decimal,
    pub unit: String,
    pub note: Option<String>,
}

// -- Commands ----------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateOutsourceCommand {
    pub supplier_id: i64,
    pub issue_date: Date,
    #[serde(default)]
    pub remark: Option<String>,

    /// 送料源仓位
    pub send_wh_id: i64,
    pub send_loc_id: i64,
    /// 回料目的仓位(委外回料待检)
    pub back_wh_id: i64,
    pub back_loc_id: i64,

    pub send_lines: Vec<CreateOutsourceLine>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateOutsourceLine {
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

#[derive(Debug, Clone, Deserialize)]
pub struct SubmitBackCommand {
    pub back_lines: Vec<SubmitBackLine>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubmitBackLine {
    pub material_id: i64,
    #[serde(default)]
    pub batch_no: String,
    pub qty: Decimal,
    pub unit: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryOutsources {
    pub outsource_no: Option<String>,
    pub supplier_id: Option<i64>,
    pub doc_status: Option<String>,
    pub date_from: Option<Date>,
    pub date_to: Option<Date>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubmitResult {
    pub outsource_id: i64,
    pub outsource_no: String,
    pub txn_no: String,
    pub doc_status: String,
}

// -- Service -----------------------------------------------------------------

#[derive(Clone)]
pub struct PmcService {
    repo: Arc<dyn PmcRepository>,
    inventory: InventoryService,
}

impl PmcService {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: Arc::new(PgPmcRepository::new(pool.clone())),
            inventory: InventoryService::new(pool),
        }
    }

    pub async fn create(
        &self,
        ctx: &AuditContext,
        cmd: CreateOutsourceCommand,
    ) -> Result<OutsourceHeadView, AppError> {
        if cmd.send_lines.is_empty() {
            return Err(AppError::business(PMC_EMPTY, "委外单送料行不能为空"));
        }
        for l in &cmd.send_lines {
            l.validate()
                .map_err(|e| AppError::validation(e.to_string()))?;
            if l.qty <= Decimal::ZERO {
                return Err(AppError::validation("数量必须 > 0"));
            }
        }
        self.repo.create(ctx, &cmd).await
    }

    /// 送料:OUT 从送料源仓位
    pub async fn submit_send(&self, ctx: &AuditContext, id: i64) -> Result<SubmitResult, AppError> {
        let head = self.repo.get(id).await?;
        if head.send_status == "SENT" {
            return Err(AppError::business(
                PMC_INVALID_TRANSITION,
                "委外送料已完成,不能重复送",
            ));
        }
        let (sw, sl, _bw, _bl) = self.repo.get_locations(id).await?;

        let mut lines = Vec::with_capacity(head.send_lines.len());
        for (i, l) in head.send_lines.iter().enumerate() {
            lines.push(TxnLineInput {
                line_no: (i as i32) + 1,
                material_id: l.material_id,
                batch_no: l.batch_no.clone(),
                qty: l.qty,
                unit: l.unit.clone(),
                io_flag: IoFlag::Out,
                source_material_id: None,
                target_material_id: None,
                stock_status: Some(StockStatus::Qualified),
                status_change_flag: false,
                location_change_flag: false,
                item_change_flag: false,
                recoverable_flag: false,
                scrap_flag: false,
                note: l.note.clone(),
            });
        }

        let tcmd = CommitTxnCommand {
            txn_type: TxnType::Out,
            scene_code: "OUTSOURCE_SEND".into(),
            scene_name: Some("委外送料".into()),
            doc_type: "OUTSOURCE".into(),
            doc_no: head.outsource_no.clone(),
            source_object_type: Some("SUPPLIER".into()),
            source_object_id: Some(head.supplier_id),
            target_object_type: None,
            target_object_id: None,
            source: Some(TxnSideInput {
                wh_id: sw,
                loc_id: sl,
                status: Some(StockStatus::Qualified),
            }),
            target: None,
            lines,
            is_exception: false,
            exception_type: None,
            related_doc_no: None,
            snapshot_json: None,
            remark: head.remark.clone(),
        };
        let committed = self.inventory.commit(ctx, tcmd).await?;

        self.repo.mark_sent(id).await?;

        Ok(SubmitResult {
            outsource_id: id,
            outsource_no: head.outsource_no,
            txn_no: committed.txn_no,
            doc_status: DocStatus::Submitted.as_str().to_string(),
        })
    }

    /// 回料:IN 到委外回料待检仓,多批次累加
    pub async fn submit_back(
        &self,
        ctx: &AuditContext,
        id: i64,
        cmd: SubmitBackCommand,
    ) -> Result<SubmitResult, AppError> {
        let head = self.repo.get(id).await?;
        if head.send_status != "SENT" {
            return Err(AppError::business(
                PMC_INVALID_TRANSITION,
                "未送料,无法回料",
            ));
        }
        if cmd.back_lines.is_empty() {
            return Err(AppError::validation("回料行不能为空"));
        }
        let (_sw, _sl, bw, bl) = self.repo.get_locations(id).await?;

        let mut txn_lines = Vec::with_capacity(cmd.back_lines.len());
        for (i, l) in cmd.back_lines.iter().enumerate() {
            if l.qty <= Decimal::ZERO {
                return Err(AppError::validation("数量必须 > 0"));
            }
            txn_lines.push(TxnLineInput {
                line_no: (i as i32) + 1,
                material_id: l.material_id,
                batch_no: l.batch_no.clone(),
                qty: l.qty,
                unit: l.unit.clone(),
                io_flag: IoFlag::In,
                source_material_id: None,
                target_material_id: None,
                stock_status: Some(StockStatus::ToCheck),
                status_change_flag: false,
                location_change_flag: false,
                item_change_flag: false,
                recoverable_flag: false,
                scrap_flag: false,
                note: l.note.clone(),
            });
        }

        let tcmd = CommitTxnCommand {
            txn_type: TxnType::In,
            scene_code: "OUTSOURCE_BACK_IN".into(),
            scene_name: Some("委外回料".into()),
            doc_type: "OUTSOURCE".into(),
            doc_no: head.outsource_no.clone(),
            source_object_type: Some("SUPPLIER".into()),
            source_object_id: Some(head.supplier_id),
            target_object_type: None,
            target_object_id: None,
            source: None,
            target: Some(TxnSideInput {
                wh_id: bw,
                loc_id: bl,
                status: Some(StockStatus::ToCheck),
            }),
            lines: txn_lines,
            is_exception: false,
            exception_type: None,
            related_doc_no: None,
            snapshot_json: None,
            remark: head.remark.clone(),
        };
        let committed = self.inventory.commit(ctx, tcmd).await?;

        // 累计到 back_d 表,并判定整单 back 是否 COMPLETED
        self.repo.append_back(id, &cmd.back_lines).await?;

        Ok(SubmitResult {
            outsource_id: id,
            outsource_no: head.outsource_no,
            txn_no: committed.txn_no,
            doc_status: DocStatus::Submitted.as_str().to_string(),
        })
    }

    pub async fn void(&self, _ctx: &AuditContext, id: i64) -> Result<(), AppError> {
        let head = self.repo.get(id).await?;
        if head.send_status != "DRAFT" {
            return Err(AppError::business(
                PMC_INVALID_TRANSITION,
                "已送料,无法作废",
            ));
        }
        self.repo.update_status(id, DocStatus::Voided).await
    }

    pub async fn get(&self, id: i64) -> Result<OutsourceHeadView, AppError> {
        self.repo.get(id).await
    }

    pub async fn list(&self, q: &QueryOutsources) -> Result<Vec<OutsourceHeadView>, AppError> {
        self.repo.list(q).await
    }
}
