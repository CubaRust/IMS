//! 时间辅助
//!
//! 本系统统一用 `time::OffsetDateTime`,序列化到 JSON 走 RFC3339。
//! 数据库里是 `timestamp`(无时区),存储时统一按 UTC 截掉时区信息。

use time::{format_description::well_known::Rfc3339, OffsetDateTime, PrimitiveDateTime};

/// 当前 UTC 时间
#[must_use]
pub fn utc_now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

/// 当前 UTC 时间(转为 PrimitiveDateTime,写库用)
#[must_use]
pub fn utc_now_primitive() -> PrimitiveDateTime {
    let now = OffsetDateTime::now_utc();
    PrimitiveDateTime::new(now.date(), now.time())
}

/// OffsetDateTime -> RFC3339 字符串
pub fn format_rfc3339(t: OffsetDateTime) -> String {
    t.format(&Rfc3339).unwrap_or_else(|_| t.to_string())
}

/// 解析 RFC3339 字符串 -> OffsetDateTime
pub fn parse_rfc3339(s: &str) -> Result<OffsetDateTime, time::error::Parse> {
    OffsetDateTime::parse(s, &Rfc3339)
}
