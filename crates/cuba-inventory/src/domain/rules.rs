//! 领域规则
//!
//! 核心函数:
//! - [`validate_txn`]   : 校验事务头 + 行的一致性
//! - [`compute_deltas`] : 把事务行翻译成对 `wms_inventory_balance` 的 delta 列表
//!
//! ## 五类动作的余额影响
//!
//! | 动作     | 行结构              | 余额变化                                |
//! |---------|---------------------|----------------------------------------|
//! | IN      | 一行 io_flag=IN     | target 定位 +book +available(或 +bad/...)|
//! | OUT     | 一行 io_flag=OUT    | source 定位 -book -available            |
//! | TRANSFER| 两行 OUT + IN,同批 | source -,target +(状态可能变)         |
//! | CONVERT | N 行(拆/组合)      | source 料 -,target 料 + (换料用)       |
//! | RESERVE | 一行 OUT            | source 定位 available -,occupied +     |
//! | RELEASE | 一行 IN             | source 定位 available +,occupied -     |
//!
//! ## 状态到字段的映射
//! - `QUALIFIED`        : book + available
//! - `BAD`              : book + bad
//! - `SCRAPPED`         : scrap(不计入 book)
//! - `PREISSUE_PENDING` : pending(不计入 book / available,且 book 可为负)
//! - `TO_CHECK / FROZEN / IN_PROCESS / OUTSOURCE / CUSTOMER_RETURN_PENDING / RECOVERY` :
//!                        book,不计入 available
//!
//! 这个映射决定了"可用库存"的口径:只有 QUALIFIED 算 available。

use std::collections::HashMap;

use rust_decimal::Decimal;

use cuba_shared::{
    error::AppError,
    types::{IoFlag, StockStatus, TxnType},
};

pub use super::{
    errors::{InventoryError},
    model::{StockDelta, StockLocator, TxnHead, TxnLine},
};

/// 校验事务结构一致性
///
/// 不查库,只做结构校验(两端信息与 txn_type 是否匹配、每行 qty>0、TRANSFER 批次一致等)。
pub fn validate_txn(head: &TxnHead, lines: &[TxnLine]) -> Result<(), AppError> {
    if lines.is_empty() {
        return Err(InventoryError::invalid_txn("事务行不能为空"));
    }
    for l in lines {
        if l.qty <= Decimal::ZERO {
            return Err(InventoryError::nonpositive_qty());
        }
    }

    match head.txn_type {
        TxnType::In => {
            if head.target.is_none() {
                return Err(InventoryError::invalid_txn("IN 必须指定 target"));
            }
            if lines.iter().any(|l| l.io_flag != IoFlag::In) {
                return Err(InventoryError::invalid_txn("IN 事务的行必须全为 io_flag=IN"));
            }
        }
        TxnType::Out | TxnType::Reserve | TxnType::Release => {
            if head.source.is_none() {
                return Err(InventoryError::invalid_txn(format!(
                    "{} 必须指定 source",
                    head.txn_type
                )));
            }
            let expected = match head.txn_type {
                TxnType::Release => IoFlag::In, // RELEASE 是把占用还回去,行是 IN
                _ => IoFlag::Out,
            };
            if lines.iter().any(|l| l.io_flag != expected) {
                return Err(InventoryError::invalid_txn(format!(
                    "{} 事务的行必须全为 io_flag={expected}",
                    head.txn_type
                )));
            }
        }
        TxnType::Transfer => {
            if head.source.is_none() || head.target.is_none() {
                return Err(InventoryError::invalid_txn("TRANSFER 必须同时指定 source + target"));
            }
            // TRANSFER 行数必须是偶数,成对出现
            if lines.len() % 2 != 0 {
                return Err(InventoryError::invalid_txn("TRANSFER 行数必须成对"));
            }
            // 简化规则:奇数行 OUT、偶数行 IN,相邻两行物料/批次一致
            for pair in lines.chunks(2) {
                let (o, i) = (&pair[0], &pair[1]);
                if o.io_flag != IoFlag::Out || i.io_flag != IoFlag::In {
                    return Err(InventoryError::invalid_txn("TRANSFER 行必须是 OUT/IN 成对"));
                }
                if o.material_id != i.material_id || o.batch_no != i.batch_no || o.qty != i.qty {
                    return Err(InventoryError::transfer_mismatch());
                }
            }
        }
        TxnType::Convert => {
            // CONVERT 是换料/换批,source 和 target 至少一个必须在(可以同仓位换料)
            if head.source.is_none() && head.target.is_none() {
                return Err(InventoryError::invalid_txn("CONVERT 至少指定 source 或 target"));
            }
            // 行级至少有一条 OUT + 一条 IN
            let has_out = lines.iter().any(|l| l.io_flag == IoFlag::Out);
            let has_in = lines.iter().any(|l| l.io_flag == IoFlag::In);
            if !has_out || !has_in {
                return Err(InventoryError::invalid_txn("CONVERT 必须同时包含 OUT 与 IN 行"));
            }
        }
    }
    Ok(())
}

/// 计算对余额的增量列表
///
/// 返回的列表每个元素代表一个唯一 `StockLocator` 的合计 delta,已折叠重复定位。
///
/// 这里不做"库存是否够扣"的校验 —— 那个在 repo 层 upsert 时由 DB 的 CHECK 约束
/// (`book_qty >= 0 or stock_status = 'PREISSUE_PENDING'`)兜底,并转为 `INV_INSUFFICIENT` 错误。
pub fn compute_deltas(head: &TxnHead, lines: &[TxnLine]) -> Result<Vec<StockDelta>, AppError> {
    let mut map: HashMap<StockLocator, StockDelta> = HashMap::new();

    for line in lines {
        let status = line
            .stock_status
            .or_else(|| side_status(head, line.io_flag))
            .ok_or_else(|| InventoryError::invalid_txn("无法推断 stock_status"))?;

        let side = match line.io_flag {
            IoFlag::In => head
                .target
                .as_ref()
                .or(head.source.as_ref())
                .ok_or_else(|| InventoryError::invalid_txn("IN 行找不到仓位"))?,
            IoFlag::Out => head
                .source
                .as_ref()
                .or(head.target.as_ref())
                .ok_or_else(|| InventoryError::invalid_txn("OUT 行找不到仓位"))?,
        };

        let locator =
            StockLocator::new(line.material_id, side.wh_id, side.loc_id, &line.batch_no, status);

        let delta = map.entry(locator.clone()).or_insert_with(|| StockDelta::zero(locator));

        apply_line_to_delta(head, line, delta, status);
    }

    Ok(map.into_values().collect())
}

/// 把单行应用到 delta 上
fn apply_line_to_delta(head: &TxnHead, line: &TxnLine, delta: &mut StockDelta, status: StockStatus) {
    let signed = match line.io_flag {
        IoFlag::In => line.qty,
        IoFlag::Out => -line.qty,
    };

    // RESERVE / RELEASE 只动 available 与 occupied,不动 book(占用不出仓)
    match head.txn_type {
        TxnType::Reserve => {
            // RESERVE:single OUT 行,扣 available 加 occupied
            delta.available += signed; // signed 为负,available 减少
            delta.occupied -= signed; // occupied 相应增加
            return;
        }
        TxnType::Release => {
            // RELEASE:single IN 行,回 available 扣 occupied
            delta.available += signed; // signed 为正
            delta.occupied -= signed;
            return;
        }
        _ => {}
    }

    // IN / OUT / TRANSFER / CONVERT:book 总要变
    delta.book += signed;

    // 按状态映射到分项
    match status {
        StockStatus::Qualified => {
            delta.available += signed;
        }
        StockStatus::Bad => {
            delta.bad += signed;
        }
        StockStatus::Scrapped => {
            // 报废不计 book,改扣 book 反向 + 加 scrap
            // 实际建模:scrap 增加,book 也跟着增加(保留在"报废仓/报废状态"里,可追溯)
            delta.scrap += signed;
        }
        StockStatus::PreissuePending => {
            // 异常先发:pending 增加(用 OUT 行减库存时 signed 为负,
            // 对应 pending 应该是"被占用" → 这里直接记录 delta.pending,
            // 余额行用的是 PREISSUE_PENDING 状态,CHECK 放行 book<0)
            delta.pending += signed.abs();
            // book 不再重复扣,因为是"占位"不是真出库
            delta.book -= signed;
        }
        // 其余状态:已在 delta.book 记录,不额外映射到 available/bad/scrap
        StockStatus::ToCheck
        | StockStatus::Frozen
        | StockStatus::InProcess
        | StockStatus::Outsource
        | StockStatus::CustomerReturnPending
        | StockStatus::Recovery => {}
    }
}

/// 基于 io_flag 推断默认状态:IN 用 target.status,OUT 用 source.status
fn side_status(head: &TxnHead, io: IoFlag) -> Option<StockStatus> {
    match io {
        IoFlag::In => head.target.as_ref().and_then(|s| s.status),
        IoFlag::Out => head.source.as_ref().and_then(|s| s.status),
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::*;
    use crate::domain::model::TxnSide;
    use crate::errors;

    fn head(txn_type: TxnType, source: Option<TxnSide>, target: Option<TxnSide>) -> TxnHead {
        TxnHead {
            txn_type,
            scene_code: "TEST".into(),
            scene_name: None,
            doc_type: "TEST".into(),
            doc_no: "T0001".into(),
            source_object_type: None,
            source_object_id: None,
            target_object_type: None,
            target_object_id: None,
            source,
            target,
            is_exception: false,
            exception_type: None,
            operator_id: Some(1),
            related_doc_no: None,
            snapshot_json: serde_json::json!({}),
            remark: None,
        }
    }

    fn side(wh_id: i64, loc_id: i64, status: Option<StockStatus>) -> TxnSide {
        TxnSide { wh_id, loc_id, status }
    }

    fn line(line_no: i32, io: IoFlag, qty: i64) -> TxnLine {
        TxnLine {
            line_no,
            material_id: 100,
            batch_no: "B1".into(),
            qty: Decimal::from(qty),
            unit: "PCS".into(),
            io_flag: io,
            source_material_id: None,
            target_material_id: None,
            stock_status: None,
            status_change_flag: false,
            location_change_flag: false,
            item_change_flag: false,
            recoverable_flag: false,
            scrap_flag: false,
            note: None,
        }
    }

    #[test]
    fn in_txn_produces_positive_delta() {
        let h = head(TxnType::In, None, Some(side(1, 1, Some(StockStatus::Qualified))));
        let lines = vec![line(1, IoFlag::In, 10)];
        let deltas = compute_deltas(&h, &lines).unwrap();
        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].book, Decimal::from(10));
        assert_eq!(deltas[0].available, Decimal::from(10));
    }

    #[test]
    fn out_txn_produces_negative_delta() {
        let h = head(TxnType::Out, Some(side(1, 1, Some(StockStatus::Qualified))), None);
        let lines = vec![line(1, IoFlag::Out, 3)];
        let deltas = compute_deltas(&h, &lines).unwrap();
        assert_eq!(deltas[0].book, Decimal::from(-3));
        assert_eq!(deltas[0].available, Decimal::from(-3));
    }

    #[test]
    fn transfer_touches_both_sides() {
        let h = head(
            TxnType::Transfer,
            Some(side(1, 1, Some(StockStatus::Qualified))),
            Some(side(2, 2, Some(StockStatus::Qualified))),
        );
        let lines = vec![line(1, IoFlag::Out, 5), line(2, IoFlag::In, 5)];
        let deltas = compute_deltas(&h, &lines).unwrap();
        assert_eq!(deltas.len(), 2);
        // 源端 -5,目标端 +5
        let source = deltas.iter().find(|d| d.locator.wh_id == 1).unwrap();
        let target = deltas.iter().find(|d| d.locator.wh_id == 2).unwrap();
        assert_eq!(source.book, Decimal::from(-5));
        assert_eq!(target.book, Decimal::from(5));
    }

    #[test]
    fn reserve_only_moves_available_to_occupied() {
        let h = head(TxnType::Reserve, Some(side(1, 1, Some(StockStatus::Qualified))), None);
        let lines = vec![line(1, IoFlag::Out, 4)];
        let deltas = compute_deltas(&h, &lines).unwrap();
        // book 不动,available -4,occupied +4
        assert_eq!(deltas[0].book, Decimal::ZERO);
        assert_eq!(deltas[0].available, Decimal::from(-4));
        assert_eq!(deltas[0].occupied, Decimal::from(4));
    }

    #[test]
    fn preissue_pending_records_pending_not_book() {
        let h = head(
            TxnType::Out,
            Some(side(1, 1, Some(StockStatus::PreissuePending))),
            None,
        );
        let lines = vec![line(1, IoFlag::Out, 2)];
        let deltas = compute_deltas(&h, &lines).unwrap();
        // pending +2,book 应该回到 0(先 -2 再 +2)
        assert_eq!(deltas[0].pending, Decimal::from(2));
    }

    #[test]
    fn validate_rejects_empty_lines() {
        let h = head(TxnType::In, None, Some(side(1, 1, Some(StockStatus::Qualified))));
        let err = validate_txn(&h, &[]).unwrap_err();
        assert_eq!(err.code(), errors::INV_INVALID_TXN);
    }

    #[test]
    fn validate_rejects_transfer_with_mismatched_batch() {
        let h = head(
            TxnType::Transfer,
            Some(side(1, 1, Some(StockStatus::Qualified))),
            Some(side(2, 2, Some(StockStatus::Qualified))),
        );
        let mut out = line(1, IoFlag::Out, 5);
        out.batch_no = "B1".into();
        let mut inn = line(2, IoFlag::In, 5);
        inn.batch_no = "B2".into();
        let err = validate_txn(&h, &[out, inn]).unwrap_err();
        assert_eq!(err.code(), errors::INV_TRANSFER_MISMATCH);
    }
}
