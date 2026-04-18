//! Tracing 初始化
//!
//! - dev   : 人读日志,彩色
//! - prod  : JSON 结构化日志(方便 ELK / Loki 等聚合)
//! - 过滤规则读自 `RUST_LOG`,默认 `info,cuba=debug`

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::config::AppEnv;

pub fn init(env: AppEnv) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,cuba=debug"));

    let registry = tracing_subscriber::registry().with(filter);

    // 用 match + 各 arm 的 init() 避免 fmt layer 类型不一致的编译问题
    match env {
        AppEnv::Prod | AppEnv::Staging => {
            registry
                .with(fmt::layer().json().with_current_span(false))
                .init();
        }
        AppEnv::Dev | AppEnv::Test => {
            registry
                .with(
                    fmt::layer()
                        .with_target(true)
                        .with_thread_ids(false)
                        .with_file(false)
                        .with_line_number(false),
                )
                .init();
        }
    }
}
