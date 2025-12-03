//! 通用 LRU 缓存实现
//!
//! 提供基于 LRU (Least Recently Used) 淘汰策略的缓存，支持：
//! - 容量限制：超过容量自动淘汰最久未使用的项
//! - TTL 过期：基于时间的自动失效
//! - 线程安全：可在多线程环境中使用
//!
//! # 使用示例
//!
//! ```rust
//! use std::time::Duration;
//! use crate::data::cache::LruCache;
//!
//! let mut cache = LruCache::new(100, Duration::from_secs(300));
//! cache.insert("key", "value");
//! assert_eq!(cache.get(&"key"), Some(&"value"));
//! ```

use linked_hash_map::LinkedHashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};

/// 缓存条目，包含值和插入时间
#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    inserted_at: Instant,
}

impl<V> CacheEntry<V> {
    fn new(value: V) -> Self {
        Self {
            value,
            inserted_at: Instant::now(),
        }
    }

    /// 检查是否已过期
    fn is_expired(&self, ttl: Duration) -> bool {
        self.inserted_at.elapsed() > ttl
    }
}

/// LRU 缓存实现
///
/// 使用 `LinkedHashMap` 保证插入顺序，实现 LRU 淘汰策略。
///
/// # 泛型参数
///
/// - `K`: 键类型，必须实现 `Eq + Hash`
/// - `V`: 值类型
#[derive(Debug)]
pub struct LruCache<K: Eq + Hash, V> {
    cache: LinkedHashMap<K, CacheEntry<V>>,
    capacity: usize,
    ttl: Duration,
}

impl<K: Eq + Hash, V> LruCache<K, V> {
    /// 创建新的 LRU 缓存
    ///
    /// # 参数
    ///
    /// - `capacity`: 缓存容量（最大条目数）
    /// - `ttl`: 缓存项的生存时间
    ///
    /// # 示例
    ///
    /// ```rust
    /// use std::time::Duration;
    /// let cache = LruCache::<String, i32>::new(100, Duration::from_secs(300));
    /// ```
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            cache: LinkedHashMap::new(),
            capacity,
            ttl,
        }
    }

    /// 获取缓存值
    ///
    /// 如果键存在且未过期，返回 `Some(&V)` 并将该项移至最近使用位置。
    /// 如果键不存在或已过期，返回 `None`。
    ///
    /// # 示例
    ///
    /// ```rust
    /// let mut cache = LruCache::new(10, Duration::from_secs(60));
    /// cache.insert("key", 42);
    /// assert_eq!(cache.get(&"key"), Some(&42));
    /// ```
    pub fn get(&mut self, key: &K) -> Option<&V> {
        // 检查是否存在
        if !self.cache.contains_key(key) {
            return None;
        }

        // 检查是否过期
        if let Some(entry) = self.cache.get(key) {
            if entry.is_expired(self.ttl) {
                // 过期，删除并返回 None
                self.cache.remove(key);
                return None;
            }
        }

        // 未过期，刷新 LRU 位置（移至末尾）
        self.cache.get_refresh(key).map(|entry| &entry.value)
    }

    /// 插入缓存值
    ///
    /// 如果键已存在，更新其值和插入时间。
    /// 如果超过容量限制，自动淘汰最久未使用的项。
    ///
    /// # 示例
    ///
    /// ```rust
    /// let mut cache = LruCache::new(2, Duration::from_secs(60));
    /// cache.insert("a", 1);
    /// cache.insert("b", 2);
    /// cache.insert("c", 3); // 淘汰 "a"
    /// assert_eq!(cache.get(&"a"), None);
    /// ```
    pub fn insert(&mut self, key: K, value: V) {
        // 如果键已存在，先删除旧值
        if self.cache.contains_key(&key) {
            self.cache.remove(&key);
        }

        // 检查容量，超过则淘汰最旧的项
        if self.cache.len() >= self.capacity {
            self.cache.pop_front();
        }

        // 插入新值（自动放在末尾）
        self.cache.insert(key, CacheEntry::new(value));
    }

    /// 删除指定键
    ///
    /// 返回被删除的值（如果存在）。
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.cache.remove(key).map(|entry| entry.value)
    }

    /// 清空所有缓存
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// 获取当前缓存项数量
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// 检查缓存是否为空
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// 获取缓存容量
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 动态调整缓存容量
    ///
    /// 如果新容量小于当前缓存项数量，会淘汰最旧的项直到满足容量限制。
    pub fn set_capacity(&mut self, new_capacity: usize) {
        self.capacity = new_capacity;
        while self.cache.len() > self.capacity {
            self.cache.pop_front();
        }
    }

    /// 动态调整 TTL
    pub fn set_ttl(&mut self, new_ttl: Duration) {
        self.ttl = new_ttl;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_basic_insert_and_get() {
        let mut cache = LruCache::new(10, Duration::from_secs(60));
        cache.insert("key1", "value1");
        assert_eq!(cache.get(&"key1"), Some(&"value1"));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_get_nonexistent_key() {
        let mut cache = LruCache::<String, i32>::new(10, Duration::from_secs(60));
        assert_eq!(cache.get(&"missing".to_string()), None);
    }

    #[test]
    fn test_capacity_limit() {
        let mut cache = LruCache::new(3, Duration::from_secs(60));
        cache.insert("a", 1);
        cache.insert("b", 2);
        cache.insert("c", 3);

        // 缓存已满
        assert_eq!(cache.len(), 3);

        // 插入第 4 个元素，应该淘汰最旧的 "a"
        cache.insert("d", 4);
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&"a"), None);
        assert_eq!(cache.get(&"b"), Some(&2));
        assert_eq!(cache.get(&"c"), Some(&3));
        assert_eq!(cache.get(&"d"), Some(&4));
    }

    #[test]
    fn test_lru_eviction_order() {
        let mut cache = LruCache::new(3, Duration::from_secs(60));
        cache.insert("a", 1);
        cache.insert("b", 2);
        cache.insert("c", 3);

        // 访问 "a"，使其成为最近使用
        cache.get(&"a");

        // 插入 "d"，应该淘汰 "b"（最久未使用）
        cache.insert("d", 4);
        assert_eq!(cache.get(&"a"), Some(&1));
        assert_eq!(cache.get(&"b"), None);
        assert_eq!(cache.get(&"c"), Some(&3));
        assert_eq!(cache.get(&"d"), Some(&4));
    }

    #[test]
    fn test_update_existing_key() {
        let mut cache = LruCache::new(10, Duration::from_secs(60));
        cache.insert("key", "value1");
        cache.insert("key", "value2");
        assert_eq!(cache.get(&"key"), Some(&"value2"));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_ttl_expiration() {
        let mut cache = LruCache::new(10, Duration::from_millis(100));
        cache.insert("key", "value");

        // 立即获取，应该成功
        assert_eq!(cache.get(&"key"), Some(&"value"));

        // 等待超过 TTL
        thread::sleep(Duration::from_millis(150));

        // 应该已过期
        assert_eq!(cache.get(&"key"), None);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_remove() {
        let mut cache = LruCache::new(10, Duration::from_secs(60));
        cache.insert("key", "value");
        assert_eq!(cache.remove(&"key"), Some("value"));
        assert_eq!(cache.get(&"key"), None);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_clear() {
        let mut cache = LruCache::new(10, Duration::from_secs(60));
        cache.insert("a", 1);
        cache.insert("b", 2);
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_is_empty() {
        let mut cache = LruCache::<String, i32>::new(10, Duration::from_secs(60));
        assert!(cache.is_empty());
        cache.insert("key".to_string(), 42);
        assert!(!cache.is_empty());
    }

    #[test]
    fn test_capacity_getter() {
        let cache = LruCache::<String, i32>::new(100, Duration::from_secs(60));
        assert_eq!(cache.capacity(), 100);
    }

    #[test]
    fn test_set_capacity_shrink() {
        let mut cache = LruCache::new(5, Duration::from_secs(60));
        cache.insert("a", 1);
        cache.insert("b", 2);
        cache.insert("c", 3);
        cache.insert("d", 4);
        cache.insert("e", 5);

        // 缩小容量到 3，应该淘汰 "a" 和 "b"
        cache.set_capacity(3);
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&"a"), None);
        assert_eq!(cache.get(&"b"), None);
        assert_eq!(cache.get(&"c"), Some(&3));
    }

    #[test]
    fn test_set_capacity_expand() {
        let mut cache = LruCache::new(2, Duration::from_secs(60));
        cache.insert("a", 1);
        cache.insert("b", 2);

        // 扩大容量
        cache.set_capacity(5);
        assert_eq!(cache.capacity(), 5);

        // 可以插入更多项
        cache.insert("c", 3);
        cache.insert("d", 4);
        assert_eq!(cache.len(), 4);
    }

    #[test]
    fn test_set_ttl() {
        let mut cache = LruCache::new(10, Duration::from_secs(60));
        cache.insert("key", "value");

        // 缩短 TTL
        cache.set_ttl(Duration::from_millis(50));
        thread::sleep(Duration::from_millis(100));

        // 应该已过期
        assert_eq!(cache.get(&"key"), None);
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::{Arc, RwLock};

        let cache = Arc::new(RwLock::new(LruCache::new(100, Duration::from_secs(60))));
        let mut handles = vec![];

        // 10 个线程并发写入
        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let handle = thread::spawn(move || {
                for j in 0..10 {
                    let key = format!("key-{}-{}", i, j);
                    cache_clone.write().unwrap().insert(key.clone(), i * 10 + j);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // 验证数据
        let cache_read = cache.read().unwrap();
        assert_eq!(cache_read.len(), 100);
    }
}
