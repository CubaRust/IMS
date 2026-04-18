//! cuba-metrics
//!
//! Prometheus 指标注册。整个进程共享一个 `Registry`,
//! 各业务 crate 通过 `record_*` 辅助函数上报。
//!
//! 暴露的指标:
//! - `cuba_http_requests_total{method,path,status}` - 计数
//! - `cuba_http_request_duration_seconds{method,path}` - 直方图
//! - `cuba_inventory_txn_total{txn_type,scene_code,result}` - 库存事务计数
//! - `cuba_business_errors_total{code}` - 业务错误码分布

#![deny(unsafe_code)]

use once_cell::sync::Lazy;
use prometheus::{
    register_counter_vec_with_registry, register_histogram_vec_with_registry,
    register_int_counter_vec_with_registry, CounterVec, Encoder, HistogramVec, IntCounterVec,
    Registry, TextEncoder,
};

pub static REGISTRY: Lazy<Registry> = Lazy::new(Registry::new);

pub static HTTP_REQUESTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec_with_registry!(
        "cuba_http_requests_total",
        "HTTP 请求数",
        &["method", "path", "status"],
        REGISTRY
    )
    .expect("register http_requests_total")
});

pub static HTTP_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec_with_registry!(
        "cuba_http_request_duration_seconds",
        "HTTP 请求耗时(秒)",
        &["method", "path"],
        // 桶设计:覆盖 5ms 到 10s
        vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0],
        REGISTRY
    )
    .expect("register http_duration")
});

pub static INVENTORY_TXN: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec_with_registry!(
        "cuba_inventory_txn_total",
        "库存事务提交次数",
        &["txn_type", "scene_code", "result"],
        REGISTRY
    )
    .expect("register inventory_txn_total")
});

pub static BUSINESS_ERRORS: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec_with_registry!(
        "cuba_business_errors_total",
        "业务错误码分布(HTTP 200 + code != 0)",
        &["code"],
        REGISTRY
    )
    .expect("register business_errors")
});

/// HTTP 中间件上报
pub fn record_http(method: &str, path: &str, status: u16, elapsed_secs: f64) {
    HTTP_REQUESTS
        .with_label_values(&[method, path, &status.to_string()])
        .inc();
    HTTP_DURATION
        .with_label_values(&[method, path])
        .observe(elapsed_secs);
}

/// inventory engine 上报
pub fn record_txn(txn_type: &str, scene_code: &str, ok: bool) {
    let result = if ok { "ok" } else { "error" };
    INVENTORY_TXN
        .with_label_values(&[txn_type, scene_code, result])
        .inc();
}

/// 业务错误码上报
pub fn record_business_error(code: u32) {
    BUSINESS_ERRORS
        .with_label_values(&[&code.to_string()])
        .inc();
}

/// 导出 Prometheus 文本格式,交给 /metrics 路由返回
#[must_use]
pub fn gather_text() -> String {
    let encoder = TextEncoder::new();
    let families = REGISTRY.gather();
    let mut buf = Vec::with_capacity(4096);
    if let Err(e) = encoder.encode(&families, &mut buf) {
        tracing::warn!(error = %e, "metrics encode failed");
        return String::new();
    }
    String::from_utf8(buf).unwrap_or_default()
}
