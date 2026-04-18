//! scrap application

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

use crate::domain::{is_valid_source, ScrapError};
use crate::infrastructure::repository::{PgScrapRepository, ScrapRepository};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapHeadView {
    pub id: i64,
    pub scrap_no: String,
    pub scrap_source: String,
    pub source_doc_type: Option<String>,
    pub source_doc_no: Option<String>,
    pub scrap_date: Date,
    pub operator_id: Option<i64>,
    pub doc_status: String,
    pub remark: Option<String>,
    pub lines: Vec<ScrapLineView>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapLineView {
    pub id: i64,
    pub line_no: i32,
    pub material_id: Option<i64>,
    pub material_code: Option<String>,
    pub batch_no: String,
    pub qty: Decimal,
    pub unit: String,
    pub stock_status: Option<String>,
    pub scrap_reason: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateScrapCommand {
    pub scrap_source: String,
    #[serde(default)]
    pub source_doc_type: Option<String>,
    #[serde(default)]
    pub source_doc_no: Option<String>,
    pub scrap_date: Date,
    #[serde(default)]
    pub remark: Option<String>,

    /// 物料所在源仓位
    pub source_wh_id: i64,
    pub source_loc_id: i64,
    /// 报废仓(通常 SCRAP01)
    pub scrap_wh_id: i64,
    pub scrap_loc_id: i64,

    pub lines: Vec<CreateScrapLine>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateScrapLine {
    pub line_no: i32,
    pub material_id: i64,
    #[serde(default)]
    pub batch_no: String,
    pub qty: Decimal,
    #[validate(length(min = 1, max = 30))]
    pub unit: String,
    /// 物料当前所在库存状态,通常 BAD
    pub stock_status: String,
    #[validate(length(min = 1, max = 50))]
    pub scrap_reason: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryScraps {
    pub scrap_no: Option<String>,
    pub scrap_source: Option<String>,
    pub doc_status: Option<String>,
    pub date_from: Option<Date>,
    pub date_to: Option<Date>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubmitScrapResult {
    pub scrap_id: i64,
    pub scrap_no: String,
    pub txn_no: String,
    pub doc_status: String,
}

#[derive(Clone)]
pub struct ScrapService {
    repo: Arc<dyn ScrapRepository>,
    inventory: InventoryService,
}

impl ScrapService {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: Arc::new(PgScrapRepository::new(pool.clone())),
            inventory: InventoryService::new(pool),
        }
    }

    pub async fn create(
        &self,
        ctx: &AuditContext,
        cmd: CreateScrapCommand,
    ) -> Result<ScrapHeadView, AppError> {
        if !is_valid_source(&cmd.scrap_source) {
            return Err(AppError::validation(format!(
                "未知报废来源: {}",
                cmd.scrap_source
            )));
        }
        if cmd.lines.is_empty() {
            return Err(ScrapError::empty_lines());
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

    /// submit:TRANSFER 从源仓位(BAD)→ 报废仓(SCRAPPED)
    pub async fn submit(&self, ctx: &AuditContext, id: i64) -> Result<SubmitScrapResult, AppError> {
        let head = self.repo.get(id).await?;
        let status = DocStatus::try_from(head.doc_status.as_str())?;
        if !matches!(status, DocStatus::Draft | DocStatus::Submitted) {
            return Err(ScrapError::invalid_transition(&head.doc_status, "submit"));
        }

        let (src_wh, src_loc, scrap_wh, scrap_loc) = self.repo.get_locations(id).await?;

        // 按源状态分组(通常所有行同状态,这里按行单独写成 transfer 对)
        let mut lines = Vec::with_capacity(head.lines.len() * 2);
        let mut line_no = 1;
        let mut src_status_first = StockStatus::Bad;
        for (i, l) in head.lines.iter().enumerate() {
            let src_stat = l
                .stock_status
                .as_deref()
                .map(StockStatus::try_from)
                .transpose()?
                .unwrap_or(StockStatus::Bad);
            if i == 0 {
                src_status_first = src_stat;
            }
            let mid = l.material_id.ok_or_else(|| {
                AppError::validation(format!("报废单行 {} 缺失 material_id", l.line_no))
            })?;
            lines.push(TxnLineInput {
                line_no,
                material_id: mid,
                batch_no: l.batch_no.clone(),
                qty: l.qty,
                unit: l.unit.clone(),
                io_flag: IoFlag::Out,
                source_material_id: None,
                target_material_id: None,
                stock_status: Some(src_stat),
                status_change_flag: true,
                location_change_flag: true,
                item_change_flag: false,
                recoverable_flag: false,
                scrap_flag: true,
                note: Some(l.scrap_reason.clone()),
            });
            line_no += 1;
            lines.push(TxnLineInput {
                line_no,
                material_id: mid,
                batch_no: l.batch_no.clone(),
                qty: l.qty,
                unit: l.unit.clone(),
                io_flag: IoFlag::In,
                source_material_id: None,
                target_material_id: None,
                stock_status: Some(StockStatus::Scrapped),
                status_change_flag: true,
                location_change_flag: true,
                item_change_flag: false,
                recoverable_flag: false,
                scrap_flag: true,
                note: Some(l.scrap_reason.clone()),
            });
            line_no += 1;
        }

        let tcmd = CommitTxnCommand {
            txn_type: TxnType::Transfer,
            scene_code: "SCRAP".into(),
            scene_name: Some("报废".into()),
            doc_type: "SCRAP".into(),
            doc_no: head.scrap_no.clone(),
            source_object_type: head.source_doc_type.clone(),
            source_object_id: None,
            target_object_type: None,
            target_object_id: None,
            source: Some(TxnSideInput {
                wh_id: src_wh,
                loc_id: src_loc,
                status: Some(src_status_first),
            }),
            target: Some(TxnSideInput {
                wh_id: scrap_wh,
                loc_id: scrap_loc,
                status: Some(StockStatus::Scrapped),
            }),
            lines,
            is_exception: true,
            exception_type: Some("SCRAP".into()),
            related_doc_no: head.source_doc_no.clone(),
            snapshot_json: None,
            remark: head.remark.clone(),
        };
        let committed = self.inventory.commit(ctx, tcmd).await?;

        self.repo
            .update_status(head.id, DocStatus::Completed)
            .await?;

        Ok(SubmitScrapResult {
            scrap_id: head.id,
            scrap_no: head.scrap_no,
            txn_no: committed.txn_no,
            doc_status: DocStatus::Completed.as_str().to_string(),
        })
    }

    pub async fn void(&self, _ctx: &AuditContext, id: i64) -> Result<(), AppError> {
        let head = self.repo.get(id).await?;
        let status = DocStatus::try_from(head.doc_status.as_str())?;
        if !status.can_void() {
            return Err(ScrapError::invalid_transition(&head.doc_status, "void"));
        }
        self.repo.update_status(head.id, DocStatus::Voided).await
    }

    pub async fn get(&self, id: i64) -> Result<ScrapHeadView, AppError> {
        self.repo.get(id).await
    }

    pub async fn list(&self, q: &QueryScraps) -> Result<Vec<ScrapHeadView>, AppError> {
        self.repo.list(q).await
    }
}
