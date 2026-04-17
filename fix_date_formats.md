# 日期格式修复指南

## 问题
所有使用 `time::Date` 类型的 Command 结构都需要添加自定义反序列化，以支持 `YYYY-MM-DD` 格式的字符串输入。

## 解决方案
使用 `cuba-shared` 中提供的 `serde_helpers` 模块。

## 需要修复的模块

### 1. cuba-outbound ✅ (待修复)
文件: `crates/cuba-outbound/src/application/mod.rs`

添加导入:
```rust
use cuba_shared::serde_helpers::date_format;
```

修改字段:
```rust
#[serde(deserialize_with = "date_format::deserialize")]
pub outbound_date: Date,
```

### 2. cuba-defect ✅ (待修复)
文件: `crates/cuba-defect/src/application/mod.rs`

添加导入:
```rust
use cuba_shared::serde_helpers::date_format;
```

修改字段:
```rust
#[serde(deserialize_with = "date_format::deserialize")]
pub found_date: Date,
```

### 3. cuba-scrap ✅ (待修复)
文件: `crates/cuba-scrap/src/application/mod.rs`

添加导入:
```rust
use cuba_shared::serde_helpers::{date_format, optional_date_format};
```

修改字段:
```rust
#[serde(deserialize_with = "date_format::deserialize")]
pub scrap_date: Date,

// 如果有 Option<Date> 字段
#[serde(default, deserialize_with = "optional_date_format::deserialize")]
pub date_from: Option<Date>,
```

### 4. cuba-recovery ✅ (待修复)
文件: `crates/cuba-recovery/src/application/mod.rs`

添加导入:
```rust
use cuba_shared::serde_helpers::date_format;
```

修改字段:
```rust
#[serde(deserialize_with = "date_format::deserialize")]
pub recovery_date: Date,
```

### 5. cuba-inbound ✅ (已修复)
已经修复，但可以改用共享的辅助函数。

### 6. cuba-preissue ✅ (已修复)
已经修复，但可以改用共享的辅助函数。

## 快速修复命令

对于每个模块，需要:
1. 添加导入语句
2. 为每个 `Date` 字段添加 `#[serde(deserialize_with = "date_format::deserialize")]`
3. 为每个 `Option<Date>` 字段添加 `#[serde(default, deserialize_with = "optional_date_format::deserialize")]`

## 测试
修复后，使用以下格式的日期字符串应该都能正常工作:
- `"2026-04-17"` (YYYY-MM-DD)
- ISO 8601 格式

## 注意事项
- 确保所有 Command 结构中的日期字段都已修复
- Query 结构中的 `Option<Date>` 字段也需要修复
- 修复后需要重新编译并重启服务器
