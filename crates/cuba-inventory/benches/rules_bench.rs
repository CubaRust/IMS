//! rules benchmarks
//!
//! 衡量 `validate_txn` 和 `compute_deltas` 在不同规模下的耗时。
//! 跑:`cargo bench -p cuba-inventory`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rust_decimal::Decimal;

use cuba_inventory::domain::{
    model::{TxnHead, TxnLine, TxnSide},
    rules,
};
use cuba_shared::types::{IoFlag, StockStatus, TxnType};

fn make_in_head() -> TxnHead {
    TxnHead {
        txn_type: TxnType::In,
        scene_code: "BENCH".into(),
        scene_name: None,
        doc_type: "INBOUND".into(),
        doc_no: "BENCH-001".into(),
        source_object_type: None,
        source_object_id: None,
        target_object_type: None,
        target_object_id: None,
        source: None,
        target: Some(TxnSide {
            wh_id: 1,
            loc_id: 1,
            status: Some(StockStatus::Qualified),
        }),
        is_exception: false,
        exception_type: None,
        operator_id: Some(1),
        related_doc_no: None,
        snapshot_json: serde_json::json!({}),
        remark: None,
    }
}

fn make_lines(n: usize) -> Vec<TxnLine> {
    (0..n)
        .map(|i| TxnLine {
            line_no: i as i32 + 1,
            material_id: 100 + i as i64,
            batch_no: format!("B{i:03}"),
            qty: Decimal::from(10 + (i as i64) % 90),
            unit: "PCS".into(),
            io_flag: IoFlag::In,
            source_material_id: None,
            target_material_id: None,
            stock_status: Some(StockStatus::Qualified),
            status_change_flag: false,
            location_change_flag: false,
            item_change_flag: false,
            recoverable_flag: false,
            scrap_flag: false,
            note: None,
        })
        .collect()
}

fn bench_validate(c: &mut Criterion) {
    let head = make_in_head();
    let mut group = c.benchmark_group("validate_txn");
    for n in [1usize, 10, 50, 200].iter() {
        let lines = make_lines(*n);
        group.bench_with_input(BenchmarkId::from_parameter(n), n, |b, _| {
            b.iter(|| {
                let _ = rules::validate_txn(black_box(&head), black_box(&lines));
            })
        });
    }
    group.finish();
}

fn bench_compute_deltas(c: &mut Criterion) {
    let head = make_in_head();
    let mut group = c.benchmark_group("compute_deltas");
    for n in [1usize, 10, 50, 200].iter() {
        let lines = make_lines(*n);
        group.bench_with_input(BenchmarkId::from_parameter(n), n, |b, _| {
            b.iter(|| {
                let _ = rules::compute_deltas(black_box(&head), black_box(&lines));
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench_validate, bench_compute_deltas);
criterion_main!(benches);
