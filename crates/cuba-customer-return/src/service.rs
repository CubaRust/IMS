//! 客户退货 service

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

use crate::repo::{CustomerReturnRepository, PgCustomerReturnRepository};

// 44xxx
pub const CR_INVALID_JUDGE: ErrorCode = ErrorCode::custom(44101);
pub const CR_EMPTY_LINES: ErrorCode = ErrorCode::custom(44102);
pub const CR_INVALID_TRANSITION: ErrorCode = ErrorCode::custom(44103);

pub const JUDGE_METHODS: &[&str] = &[
    "RETURN_TO_STOCK",
    "TO_DEFECT",
    "TO_SCRAP",
    "TO_SUPPLIER_RETURN",
    "TO_CHECK",
];

// -- View --------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerReturnHeadView {
    pub id: i64,
    pub return_no: String,
    pub customer_id: i64,
    pub customer_name: Option<String>,
    pub return_date: Date,
    pub original_doc_no: Option<String>,
    pub operator_id: Option<i64>,
    pub doc_status: String,
    pub remark: Option<String>,
    pub lines: Vec<CustomerReturnLineView>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerReturnLineView {
    pub id: i64,
    pub line_no: i32,
    pub material_id: i64,
    pub material_code: Option<String>,
    pub batch_no: String,
    pub qty: Decimal,
    pub unit: String,
    pub return_reason: String,
    pub judge_method: Option<String>,
    pub judge_note: Option<String>,
    pub note: Option<String>,
}

// -- Commands ----------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateCustomerReturnCommand {
    pub customer_id: i64,
    pub return_date: Date,
    #[serde(default)]
    pub original_doc_no: Option<String>,
    #[serde(default)]
    pub remark: Option<String>,

    /// 临时待检仓(未判定时)
    pub return_wh_id: i64,
    pub return_loc_id: i64,
    /// TO_DEFECT 目的仓(可选)
    #[serde(default)]
    pub defect_wh_id: Option<i64>,
    #[serde(default)]
    pub defect_loc_id: Option<i64>,
    /// TO_SCRAP 目的仓(可选)
    #[serde(default)]
    pub scrap_wh_id: Option<i64>,
    #[serde(default)]
    pub scrap_loc_id: Option<i64>,

    pub lines: Vec<CreateCustomerReturnLine>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateCustomerReturnLine {
    pub line_no: i32,
    pub material_id: i64,
    #[serde(default)]
    pub batch_no: String,
    pub qty: Decimal,
    #[validate(length(min = 1, max = 30))]
    pub unit: String,
    #[validate(length(min = 1, max = 200))]
    pub return_reason: String,
    #[serde(default)]
    pub judge_method: Option<String>,
    #[serde(default)]
    pub judge_note: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JudgeLineCommand {
    pub line_id: i64,
    pub judge_method: String,
    #[serde(default)]
    pub judge_note: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QueryCustomerReturns {
    pub return_no: Option<String>,
    pub customer_id: Option<i64>,
    pub doc_status: Option<String>,
    pub date_from: Option<Date>,
    pub date_to: Option<Date>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubmitCustomerReturnResult {
    pub return_id: i64,
    pub return_no: String,
    pub txn_no: String,
    pub doc_status: String,
}

// -- Service -----------------------------------------------------------------

#[derive(Clone)]
pub struct CustomerReturnService {
    repo: Arc<dyn CustomerReturnRepository>,
    inventory: InventoryService,
}

impl CustomerReturnService {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: Arc::new(PgCustomerReturnRepository::new(pool.clone())),
            inventory: InventoryService::new(pool),
        }
    }

    pub async fn create(
        &self,
        ctx: &AuditContext,
        cmd: CreateCustomerReturnCommand,
    ) -> Result<CustomerReturnHeadView, AppError> {
        if cmd.lines.is_empty() {
            return Err(AppError::business(CR_EMPTY_LINES, "客户退货行不能为空"));
        }
        for l in &cmd.lines {
            l.validate().map_err(|e| AppError::validation(e.to_string()))?;
            if l.qty <= Decimal::ZERO {
                return Err(AppError::validation("数量必须 > 0"));
            }
            if let Some(j) = &l.judge_method {
                if !JUDGE_METHODS.contains(&j.as_str()) {
                    return Err(AppError::business(
                        CR_INVALID_JUDGE,
                        format!("未知判定: {j}"),
                    ));
                }
            }
        }
        self.repo.create(ctx, &cmd).await
    }

    pub async fn judge(
        &self,
        _ctx: &AuditContext,
        id: i64,
        lines: Vec<JudgeLineCommand>,
    ) -> Result<(), AppError> {
        for j in &lines {
            if !JUDGE_METHODS.contains(&j.judge_method.as_str()) {
                return Err(AppError::business(
                    CR_INVALID_JUDGE,
                    format!("未知判定: {}", j.judge_method),
                ));
            }
        }
        self.repo.update_judges(id, &lines).await
    }

    /// submit:按各行 judge_method 分组,产生多个 IN 事务
    ///
    /// 为简化,这里把所有行视作一个"集合入库到 return_wh/loc 的 TO_CHECK",
    /// 如果要按判定分流多仓位,可以演化为多个 txn。
    ///
    /// 本期实现:
    /// 1. 按 judge_method 分组
    /// 2. 每组一笔 IN 事务(scene_code = CUSTOMER_RETURN_IN),status 对应 QUALIFIED/BAD/SCRAPPED
    /// 3. 缺省(未判定)视作 TO_CHECK → return_wh/loc
    pub async fn submit(
        &self,
        ctx: &AuditContext,
        id: i64,
    ) -> Result<SubmitCustomerReturnResult, AppError> {
        let head = self.repo.get(id).await?;
        let status = DocStatus::try_from(head.doc_status.as_str())?;
        if !matches!(status, DocStatus::Draft | DocStatus::Submitted) {
            return Err(AppError::business(
                CR_INVALID_TRANSITION,
                format!("当前状态 {} 不允许 submit", head.doc_status),
            ));
        }

        let locs = self.repo.get_locations(id).await?;

        // 按 judge_method 分组;每组走一笔 IN 事务
        use std::collections::HashMap;
        let mut groups: HashMap<String, Vec<&CustomerReturnLineView>> = HashMap::new();
        for l in &head.lines {
            let key = l.judge_method.clone().unwrap_or_else(|| "TO_CHECK".into());
            groups.entry(key).or_default().push(l);
        }

        let mut last_txn_no = String::new();
        for (method, lines) in groups {
            let (wh, loc, stat) = match method.as_str() {
                "RETURN_TO_STOCK" => (locs.return_wh, locs.return_loc, StockStatus::Qualified),
                "TO_DEFECT" => {
                    let w = locs.defect_wh.ok_or_else(|| {
                        AppError::validation("TO_DEFECT 需要 defect_wh_id/loc_id")
                    })?;
                    let l = locs.defect_loc.ok_or_else(|| {
                        AppError::validation("TO_DEFECT 需要 defect_loc_id")
                    })?;
                    (w, l, StockStatus::Bad)
                }
                "TO_SCRAP" => {
                    let w = locs.scrap_wh.ok_or_else(|| {
                        AppError::validation("TO_SCRAP 需要 scrap_wh_id")
                    })?;
                    let l = locs.scrap_loc.ok_or_else(|| {
                        AppError::validation("TO_SCRAP 需要 scrap_loc_id")
                    })?;
                    (w, l, StockStatus::Scrapped)
                }
                "TO_SUPPLIER_RETURN" => {
                    // 先入不良仓,等退供单再出
                    let w = locs.defect_wh.unwrap_or(locs.return_wh);
                    let l = locs.defect_loc.unwrap_or(locs.return_loc);
                    (w, l, StockStatus::Bad)
                }
                _ => (locs.return_wh, locs.return_loc, StockStatus::ToCheck),
            };

            let mut txn_lines = Vec::with_capacity(lines.len());
            for (i, l) in lines.iter().enumerate() {
                txn_lines.push(TxnLineInput {
                    line_no: (i as i32) + 1,
                    material_id: l.material_id,
                    batch_no: l.batch_no.clone(),
                    qty: l.qty,
                    unit: l.unit.clone(),
                    io_flag: IoFlag::In,
                    source_material_id: None,
                    target_material_id: None,
                    stock_status: Some(stat),
                    status_change_flag: false,
                    location_change_flag: false,
                    item_change_flag: false,
                    recoverable_flag: false,
                    scrap_flag: stat == StockStatus::Scrapped,
                    note: l.note.clone(),
                });
            }

            let tcmd = CommitTxnCommand {
                txn_type: TxnType::In,
                scene_code: "CUSTOMER_RETURN_IN".into(),
                scene_name: Some(format!("客户退货 ({method})")),
                doc_type: "CUSTOMER_RETURN".into(),
                doc_no: head.return_no.clone(),
                source_object_type: None,
                source_object_id: None,
                target_object_type: Some("CUSTOMER".into()),
                target_object_id: Some(head.customer_id),
                source: None,
                target: Some(TxnSideInput { wh_id: wh, loc_id: loc, status: Some(stat) }),
                lines: txn_lines,
                is_exception: method != "RETURN_TO_STOCK",
                exception_type: if method != "RETURN_TO_STOCK" {
                    Some(method.clone())
                } else {
                    None
                },
                related_doc_no: head.original_doc_no.clone(),
                snapshot_json: None,
                remark: head.remark.clone(),
            };
            let committed = self.inventory.commit(ctx, tcmd).await?;
            last_txn_no = committed.txn_no;
        }

        self.repo.update_status(id, DocStatus::Completed).await?;

        Ok(SubmitCustomerReturnResult {
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
                CR_INVALID_TRANSITION,
                format!("当前状态 {} 不允许作废", head.doc_status),
            ));
        }
        self.repo.update_status(id, DocStatus::Voided).await
    }

    pub async fn get(&self, id: i64) -> Result<CustomerReturnHeadView, AppError> {
        self.repo.get(id).await
    }

    pub async fn list(
        &self,
        q: &QueryCustomerReturns,
    ) -> Result<Vec<CustomerReturnHeadView>, AppError> {
        self.repo.list(q).await
    }
}
