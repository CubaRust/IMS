//! 分页请求 / 响应
//!
//! ## 约定
//! - `page` 从 1 开始
//! - `size` 默认 20,上限 200
//! - 返回 `total` 用于前端分页控件;若只需要"有无下一页",可以走游标分页(另行定义)

use serde::{Deserialize, Serialize};

/// 分页请求
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default)]
pub struct PageQuery {
    pub page: u32,
    pub size: u32,
}

impl Default for PageQuery {
    fn default() -> Self {
        Self { page: 1, size: 20 }
    }
}

impl PageQuery {
    /// 规范化:限制上下界
    #[must_use]
    pub fn normalize(self) -> Self {
        Self {
            page: self.page.max(1),
            size: self.size.clamp(1, 200),
        }
    }

    /// SQL OFFSET
    #[must_use]
    pub const fn offset(self) -> i64 {
        ((self.page.saturating_sub(1)) as i64) * (self.size as i64)
    }

    /// SQL LIMIT
    #[must_use]
    pub const fn limit(self) -> i64 {
        self.size as i64
    }
}

/// 分页响应
#[derive(Debug, Clone, Serialize)]
pub struct PageResponse<T> {
    pub page: u32,
    pub size: u32,
    pub total: i64,
    pub items: Vec<T>,
}

impl<T> PageResponse<T> {
    pub const fn new(query: PageQuery, total: i64, items: Vec<T>) -> Self {
        Self {
            page: query.page,
            size: query.size,
            total,
            items,
        }
    }

    pub const fn empty(query: PageQuery) -> Self {
        Self {
            page: query.page,
            size: query.size,
            total: 0,
            items: Vec::new(),
        }
    }

    pub fn map<U, F>(self, f: F) -> PageResponse<U>
    where
        F: FnMut(T) -> U,
    {
        PageResponse {
            page: self.page,
            size: self.size,
            total: self.total,
            items: self.items.into_iter().map(f).collect(),
        }
    }
}
