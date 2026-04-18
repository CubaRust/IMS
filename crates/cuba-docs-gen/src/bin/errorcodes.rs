//! 扫源码生成错误码表(JSON + Markdown)
//!
//! 跑法:`cargo run -p cuba-docs-gen --bin errorcodes`
//!
//! 扫描规则:遍历 `crates/cuba-*/src/**/*.rs`,匹配形如
//!   `pub const NAME: ErrorCode = ErrorCode::custom(NNNNN);`
//! 和 `pub const NAME: Self = Self(NNNNN);` 的定义,外加行内注释/文档注释作为描述。

use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct ErrorEntry {
    code: u32,
    name: String,
    segment: String,
    segment_name: String,
    crate_name: String,
    description: String,
}

fn main() -> anyhow::Result<()> {
    let repo_root = find_repo_root()?;
    let crates_dir = repo_root.join("crates");

    let mut entries: Vec<ErrorEntry> = Vec::new();

    for entry in fs::read_dir(&crates_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let crate_name = entry.file_name().to_string_lossy().into_owned();
        if !crate_name.starts_with("cuba-") {
            continue;
        }
        let src = entry.path().join("src");
        if !src.exists() {
            continue;
        }
        walk_dir(&src, &crate_name, &mut entries);
    }

    // 排序
    entries.sort_by_key(|e| e.code);

    let out_dir = repo_root.join("docs/api");
    fs::create_dir_all(&out_dir)?;

    // 写 JSON
    let json_path = out_dir.join("error-codes.json");
    let grouped: BTreeMap<String, Vec<&ErrorEntry>> = entries.iter().fold(
        BTreeMap::new(),
        |mut acc, e| {
            acc.entry(e.segment.clone()).or_default().push(e);
            acc
        },
    );
    let json = serde_json::json!({
        "generated_at": current_iso(),
        "total": entries.len(),
        "segments": grouped.iter().map(|(k, v)| {
            serde_json::json!({
                "segment": k,
                "segment_name": v.first().map(|e| e.segment_name.clone()).unwrap_or_default(),
                "codes": v,
            })
        }).collect::<Vec<_>>(),
    });
    fs::write(&json_path, serde_json::to_string_pretty(&json)?)?;
    println!("✅ {}", json_path.display());

    // 写 MD
    let md_path = out_dir.join("error-codes.md");
    let mut md = String::new();
    md.push_str("# 错误码表\n\n");
    md.push_str(&format!("> 自动生成自源码,共 {} 条。手动修改无效。\n\n", entries.len()));
    md.push_str("## 约定\n\n");
    md.push_str("- 业务错误 → HTTP 200 + `code != 0`\n");
    md.push_str("- 系统错误 → HTTP 4xx/5xx + `code`\n");
    md.push_str("- 前端应按 `code` 做统一拦截,而非 HTTP 状态\n\n");
    md.push_str("## 段位划分\n\n");
    md.push_str("| 段位 | 模块 |\n|---|---|\n");
    for (seg, codes) in &grouped {
        if let Some(e) = codes.first() {
            md.push_str(&format!("| `{}` | {} |\n", seg, e.segment_name));
        }
    }
    md.push('\n');

    for (seg, codes) in &grouped {
        if let Some(e) = codes.first() {
            md.push_str(&format!("\n## {} — {}\n\n", seg, e.segment_name));
        } else {
            continue;
        }
        md.push_str("| 码 | 常量名 | 来源 | 含义 |\n|---|---|---|---|\n");
        for c in codes {
            md.push_str(&format!(
                "| `{}` | `{}` | `{}` | {} |\n",
                c.code,
                c.name,
                c.crate_name,
                if c.description.is_empty() { "—" } else { &c.description }
            ));
        }
    }

    fs::write(&md_path, &md)?;
    println!("✅ {}", md_path.display());
    println!("\n共 {} 条错误码", entries.len());
    Ok(())
}

fn current_iso() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

fn walk_dir(dir: &Path, crate_name: &str, out: &mut Vec<ErrorEntry>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, crate_name, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            scan_file(&path, crate_name, out);
        }
    }
}

fn scan_file(path: &Path, crate_name: &str, out: &mut Vec<ErrorEntry>) {
    let Ok(text) = fs::read_to_string(path) else { return };
    // 正则在 stdlib 没有,手工解析
    let mut pending_doc: Vec<String> = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with("///") {
            pending_doc.push(line.trim_start_matches('/').trim().to_string());
            continue;
        }
        if line.is_empty() || line.starts_with("//") {
            if !line.starts_with("///") {
                // 非文档注释不打断,但清空 pending
                pending_doc.clear();
            }
            continue;
        }

        // 形式 1: pub const NAME: ErrorCode = ErrorCode::custom(20101);
        if let Some(entry) = parse_custom(line, crate_name, &pending_doc) {
            out.push(entry);
            pending_doc.clear();
            continue;
        }
        // 形式 2: pub const NAME: Self = Self(10401);  (codes.rs 里)
        if let Some(entry) = parse_self(line, crate_name, &pending_doc) {
            out.push(entry);
            pending_doc.clear();
            continue;
        }

        // 其它代码行打断 pending
        if !line.starts_with('#') {
            pending_doc.clear();
        }
    }
}

fn parse_custom(line: &str, crate_name: &str, doc: &[String]) -> Option<ErrorEntry> {
    // 例: pub const INV_INSUFFICIENT: ErrorCode = ErrorCode::custom(20101);
    let s = line.strip_prefix("pub const ")?;
    let colon = s.find(':')?;
    let name = s[..colon].trim().to_string();
    let rest = &s[colon..];
    let start = rest.find("::custom(")?;
    let tail = &rest[start + "::custom(".len()..];
    let end = tail.find(')')?;
    let code: u32 = tail[..end].trim().parse().ok()?;
    Some(make_entry(code, name, crate_name, doc))
}

fn parse_self(line: &str, crate_name: &str, doc: &[String]) -> Option<ErrorEntry> {
    // 例: pub const UNAUTHENTICATED: Self = Self(10401);
    let s = line.strip_prefix("pub const ")?;
    if !s.contains(": Self") {
        return None;
    }
    let colon = s.find(':')?;
    let name = s[..colon].trim().to_string();
    let start = s.find("Self(")?;
    let tail = &s[start + "Self(".len()..];
    let end = tail.find(')')?;
    let code: u32 = tail[..end].trim().parse().ok()?;
    Some(make_entry(code, name, crate_name, doc))
}

fn make_entry(code: u32, name: String, crate_name: &str, doc: &[String]) -> ErrorEntry {
    let (segment, segment_name) = classify(code);
    let description = doc.join(" ").trim().to_string();
    ErrorEntry {
        code,
        name,
        segment,
        segment_name,
        crate_name: crate_name.to_string(),
        description,
    }
}

fn classify(code: u32) -> (String, String) {
    let seg = code / 1000;
    let seg_str = format!("{seg}xxx");
    let name = match seg {
        10 => "通用",
        11 => "身份 identity",
        20 => "库存 inventory",
        21 => "仓库 warehouse",
        22 => "主数据 catalog",
        30 => "入库 inbound",
        31 => "出库 outbound",
        33 => "异常先发 preissue",
        40 => "不良 defect",
        41 => "拆解 recovery",
        42 => "报废 scrap",
        44 => "客退 customer-return",
        45 => "退供 supplier-return",
        46 => "委外 pmc",
        47 => "盘点 stocktake",
        _ => "未分类",
    };
    (seg_str, name.into())
}

fn find_repo_root() -> anyhow::Result<PathBuf> {
    let mut cur = std::env::current_dir()?;
    loop {
        if cur.join("Cargo.toml").exists() && cur.join("crates").exists() {
            return Ok(cur);
        }
        if !cur.pop() {
            anyhow::bail!("无法定位仓库根(找不到 crates/ 目录)");
        }
    }
}
