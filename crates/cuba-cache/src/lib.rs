//! cuba-cache
//!
//! 进程内主数据缓存(moka,TTL)。
//!
//! ## 使用
//! ```ignore
//! use cuba_cache::TypedCache;
//! let cache: TypedCache<i64, Material> = TypedCache::new("material", 10_000, 300);
//! let v = cache.get_or_load(tenant_id, mat_id, || async { load_from_db().await }).await?;
//! ```
//!
//! ## Key 约定
//! 缓存 key 一律包含 tenant_id 前缀避免跨租户污染:
//! `(tenant_id, inner_key)`
//!
//! ## 失效
//! 写入/更新操作必须调 `invalidate(tenant_id, key)` 清除;或 `invalidate_all()`。

#![deny(unsafe_code)]

use std::fmt::Debug;
use std::future::Future;
use std::hash::Hash;
use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;

/// 泛型缓存
///
/// `K` 是内部 key(不含 tenant_id);`V` 是值。
pub struct TypedCache<K, V>
where
    K: Hash + Eq + Send + Sync + Clone + 'static,
    V: Clone + Send + Sync + 'static,
{
    name: &'static str,
    inner: Cache<(i64, K), Arc<V>>,
}

impl<K, V> TypedCache<K, V>
where
    K: Hash + Eq + Send + Sync + Clone + Debug + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// 新建缓存
    ///
    /// - `name`: 用于 metrics 标签
    /// - `max_capacity`: 条目上限
    /// - `ttl_secs`: 写入后过期时间
    pub fn new(name: &'static str, max_capacity: u64, ttl_secs: u64) -> Self {
        Self {
            name,
            inner: Cache::builder()
                .name(name)
                .max_capacity(max_capacity)
                .time_to_live(Duration::from_secs(ttl_secs))
                .build(),
        }
    }

    /// 取;未命中时调 `loader` 加载并写入
    pub async fn get_or_load<F, Fut, E>(
        &self,
        tenant_id: i64,
        key: K,
        loader: F,
    ) -> Result<Arc<V>, E>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<V, E>>,
    {
        let full_key = (tenant_id, key.clone());
        if let Some(v) = self.inner.get(&full_key).await {
            record_hit(self.name);
            return Ok(v);
        }
        record_miss(self.name);
        let v = loader().await?;
        let arc = Arc::new(v);
        self.inner.insert(full_key, arc.clone()).await;
        Ok(arc)
    }

    /// 取(不加载)
    pub async fn get(&self, tenant_id: i64, key: &K) -> Option<Arc<V>> {
        let full_key = (tenant_id, key.clone());
        let v = self.inner.get(&full_key).await;
        if v.is_some() {
            record_hit(self.name);
        } else {
            record_miss(self.name);
        }
        v
    }

    /// 手动写入
    pub async fn put(&self, tenant_id: i64, key: K, value: V) {
        self.inner.insert((tenant_id, key), Arc::new(value)).await;
    }

    /// 失效单个 key
    pub async fn invalidate(&self, tenant_id: i64, key: &K) {
        self.inner.invalidate(&(tenant_id, key.clone())).await;
    }

    /// 失效整个租户的所有 key(主数据批量改时用)
    pub fn invalidate_all(&self) {
        self.inner.invalidate_all();
    }

    #[must_use]
    pub fn entry_count(&self) -> u64 {
        self.inner.entry_count()
    }
}

fn record_hit(name: &str) {
    // 借 cuba_inventory_txn_total 的 pattern 记录,scene_code=CACHE-<name>
    cuba_metrics::record_txn("CACHE_GET", name, true);
    tracing::trace!(cache = name, "hit");
}

fn record_miss(name: &str) {
    cuba_metrics::record_txn("CACHE_GET", name, false);
    tracing::trace!(cache = name, "miss");
}
