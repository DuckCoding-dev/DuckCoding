//! SQL 查询缓存实现
//!
//! 提供基于 SQL 语句和参数的查询缓存，支持：
//! - 表级别的批量失效
//! - 序列化查询结果为字节流
//! - 线程安全访问
//!
//! # 使用示例
//!
//! ```rust
//! use std::time::Duration;
//! use crate::data::cache::{SqlQueryCache, QueryKey};
//!
//! let cache = SqlQueryCache::new(100, Duration::from_secs(300));
//!
//! // 构造查询键
//! let key = QueryKey::new(
//!     "SELECT * FROM users WHERE id = ?".to_string(),
//!     vec!["1".to_string()]
//! );
//!
//! // 插入查询结果
//! let result = vec![1, 2, 3, 4]; // 序列化的查询结果
//! cache.insert(key.clone(), result, vec!["users".to_string()]);
//!
//! // 获取缓存
//! if let Some(cached_result) = cache.get(&key) {
//!     println!("缓存命中");
//! }
//!
//! // 使 users 表的所有查询失效
//! cache.invalidate_table("users");
//! ```

use super::LruCache;
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// SQL 查询键
///
/// 由 SQL 语句和参数组成，用于唯一标识一个查询。
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct QueryKey {
    /// SQL 语句
    pub sql: String,
    /// 参数列表（序列化为字符串）
    pub params: Vec<String>,
}

impl QueryKey {
    /// 创建新的查询键
    pub fn new(sql: String, params: Vec<String>) -> Self {
        Self { sql, params }
    }
}

/// SQL 查询缓存
///
/// 使用 LRU 缓存存储序列化的查询结果，并维护表依赖关系。
#[derive(Debug, Clone)]
pub struct SqlQueryCache {
    /// LRU 缓存，键为查询键，值为序列化的查询结果
    cache: Arc<RwLock<LruCache<QueryKey, Vec<u8>>>>,
    /// 表依赖映射，记录每个表被哪些查询使用
    table_deps: Arc<RwLock<HashMap<String, Vec<QueryKey>>>>,
    /// 缓存容量
    capacity: usize,
    /// 缓存 TTL（存储用于查询）
    #[allow(dead_code)]
    ttl: Duration,
}

impl SqlQueryCache {
    /// 创建新的 SQL 查询缓存
    ///
    /// # 参数
    ///
    /// - `capacity`: 缓存容量（最大查询数）
    /// - `ttl`: 缓存项的生存时间
    ///
    /// # 示例
    ///
    /// ```rust
    /// use std::time::Duration;
    /// let cache = SqlQueryCache::new(100, Duration::from_secs(300)); // 100 个查询，5 分钟 TTL
    /// ```
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(capacity, ttl))),
            table_deps: Arc::new(RwLock::new(HashMap::new())),
            capacity,
            ttl,
        }
    }

    /// 获取缓存的查询结果
    ///
    /// # 返回
    ///
    /// - `Some(Vec<u8>)`: 缓存命中且未过期
    /// - `None`: 缓存未命中或已过期
    pub fn get(&self, key: &QueryKey) -> Option<Vec<u8>> {
        let mut cache = self.cache.write().ok()?;
        cache.get(key).cloned()
    }

    /// 插入查询结果
    ///
    /// # 参数
    ///
    /// - `key`: 查询键
    /// - `result`: 序列化的查询结果
    /// - `tables`: 查询涉及的表列表
    pub fn insert(&self, key: QueryKey, result: Vec<u8>, tables: Vec<String>) {
        // 插入缓存
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(key.clone(), result);
        }

        // 注册表依赖
        if let Ok(mut deps) = self.table_deps.write() {
            for table in tables {
                deps.entry(table).or_insert_with(Vec::new).push(key.clone());
            }
        }
    }

    /// 使某个表的所有相关查询失效
    ///
    /// # 参数
    ///
    /// - `table`: 表名
    pub fn invalidate_table(&self, table: &str) {
        // 获取该表的所有相关查询
        let queries_to_remove = if let Ok(mut deps) = self.table_deps.write() {
            deps.remove(table).unwrap_or_default()
        } else {
            return;
        };

        // 删除这些查询的缓存
        if let Ok(mut cache) = self.cache.write() {
            for query_key in queries_to_remove {
                cache.remove(&query_key);
            }
        }
    }

    /// 清空所有缓存
    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }

        if let Ok(mut deps) = self.table_deps.write() {
            deps.clear();
        }
    }

    /// 获取当前缓存项数量
    pub fn len(&self) -> usize {
        self.cache.read().map(|c| c.len()).unwrap_or(0)
    }

    /// 检查缓存是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 获取缓存容量
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 动态调整缓存容量
    pub fn set_capacity(&self, new_capacity: usize) {
        if let Ok(mut cache) = self.cache.write() {
            cache.set_capacity(new_capacity);
        }
    }

    /// 动态调整 TTL
    pub fn set_ttl(&self, new_ttl: Duration) {
        if let Ok(mut cache) = self.cache.write() {
            cache.set_ttl(new_ttl);
        }
    }
}

/// 从 SQL 语句中提取表名
///
/// 使用简单的正则表达式匹配 FROM、JOIN、INSERT INTO、UPDATE、DELETE FROM 后的表名。
/// 注意：这是一个简化的实现，对于复杂的 SQL 可能不准确。
/// 如需更准确的解析，可使用 `sqlparser` crate。
///
/// # 参数
///
/// - `sql`: SQL 语句
///
/// # 返回
///
/// 表名列表
///
/// # 示例
///
/// ```rust
/// let tables = extract_tables("SELECT * FROM users JOIN sessions ON users.id = sessions.user_id");
/// assert_eq!(tables, vec!["users", "sessions"]);
///
/// let tables = extract_tables("INSERT INTO users (name) VALUES ('Alice')");
/// assert_eq!(tables, vec!["users"]);
///
/// let tables = extract_tables("UPDATE users SET name = 'Bob' WHERE id = 1");
/// assert_eq!(tables, vec!["users"]);
/// ```
pub fn extract_tables(sql: &str) -> Vec<String> {
    let mut tables = Vec::new();

    // 正则表达式匹配 FROM、JOIN、INSERT INTO、UPDATE、DELETE FROM 后的表名
    // 匹配模式：
    // - FROM/JOIN 后跟表名（可能有模式前缀，如 main.users）
    // - INSERT INTO 后跟表名
    // - UPDATE 后跟表名
    // - DELETE FROM 后跟表名
    let re =
        Regex::new(r"(?i)\b(?:FROM|JOIN|INSERT\s+INTO|UPDATE|DELETE\s+FROM)\s+(?:(\w+)\.)?(\w+)")
            .unwrap();

    for cap in re.captures_iter(sql) {
        // 优先使用不带模式的表名（cap[2]），如果有模式则忽略（cap[1]）
        if let Some(table_match) = cap.get(2) {
            let table_name = table_match.as_str().to_lowercase();
            if !tables.contains(&table_name) {
                tables.push(table_name);
            }
        }
    }

    tables
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_query_key_creation() {
        let key = QueryKey::new(
            "SELECT * FROM users WHERE id = ?".to_string(),
            vec!["1".to_string()],
        );
        assert_eq!(key.sql, "SELECT * FROM users WHERE id = ?");
        assert_eq!(key.params, vec!["1"]);
    }

    #[test]
    fn test_query_key_equality() {
        let key1 = QueryKey::new("SELECT * FROM users".to_string(), vec![]);
        let key2 = QueryKey::new("SELECT * FROM users".to_string(), vec![]);
        assert_eq!(key1, key2);

        let key3 = QueryKey::new("SELECT * FROM posts".to_string(), vec![]);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_basic_insert_and_get() {
        let cache = SqlQueryCache::new(10, Duration::from_secs(60));
        let key = QueryKey::new("SELECT * FROM users".to_string(), vec![]);
        let result = vec![1, 2, 3, 4];

        cache.insert(key.clone(), result.clone(), vec!["users".to_string()]);

        let cached = cache.get(&key).unwrap();
        assert_eq!(cached, result);
    }

    #[test]
    fn test_cache_miss() {
        let cache = SqlQueryCache::new(10, Duration::from_secs(60));
        let key = QueryKey::new("SELECT * FROM users".to_string(), vec![]);

        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn test_invalidate_table() {
        let cache = SqlQueryCache::new(10, Duration::from_secs(60));

        // 插入两个查询，都涉及 users 表
        let key1 = QueryKey::new("SELECT * FROM users".to_string(), vec![]);
        let key2 = QueryKey::new(
            "SELECT id FROM users WHERE active = ?".to_string(),
            vec!["true".to_string()],
        );
        let key3 = QueryKey::new("SELECT * FROM posts".to_string(), vec![]);

        cache.insert(key1.clone(), vec![1, 2, 3], vec!["users".to_string()]);
        cache.insert(key2.clone(), vec![4, 5, 6], vec!["users".to_string()]);
        cache.insert(key3.clone(), vec![7, 8, 9], vec!["posts".to_string()]);

        assert_eq!(cache.len(), 3);

        // 使 users 表的查询失效
        cache.invalidate_table("users");

        // key1 和 key2 应该被删除，key3 保留
        assert!(cache.get(&key1).is_none());
        assert!(cache.get(&key2).is_none());
        assert!(cache.get(&key3).is_some());
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_clear() {
        let cache = SqlQueryCache::new(10, Duration::from_secs(60));

        for i in 0..5 {
            let key = QueryKey::new(format!("SELECT * FROM table{}", i), vec![]);
            cache.insert(key, vec![i as u8], vec![format!("table{}", i)]);
        }

        assert_eq!(cache.len(), 5);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_capacity_limit() {
        let cache = SqlQueryCache::new(3, Duration::from_secs(60));

        // 插入 4 个查询，应该淘汰最旧的
        for i in 0..4 {
            let key = QueryKey::new(format!("SELECT * FROM table{}", i), vec![]);
            cache.insert(key, vec![i as u8], vec![]);
        }

        // 缓存容量为 3
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn test_ttl_expiration() {
        let cache = SqlQueryCache::new(10, Duration::from_millis(100));
        let key = QueryKey::new("SELECT * FROM users".to_string(), vec![]);

        cache.insert(key.clone(), vec![1, 2, 3], vec!["users".to_string()]);
        assert!(cache.get(&key).is_some());

        // 等待超过 TTL
        thread::sleep(Duration::from_millis(150));

        // 缓存应该已过期
        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn test_concurrent_access() {
        let cache = Arc::new(SqlQueryCache::new(100, Duration::from_secs(60)));
        let mut handles = vec![];

        // 10 个线程并发插入
        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let handle = thread::spawn(move || {
                for j in 0..10 {
                    let key = QueryKey::new(
                        format!("SELECT * FROM table{} WHERE id = ?", i),
                        vec![j.to_string()],
                    );
                    cache_clone.insert(key, vec![i as u8, j as u8], vec![format!("table{}", i)]);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // 验证所有数据都已插入
        assert_eq!(cache.len(), 100);
    }

    #[test]
    fn test_extract_tables_simple() {
        let sql = "SELECT * FROM users";
        let tables = extract_tables(sql);
        assert_eq!(tables, vec!["users"]);
    }

    #[test]
    fn test_extract_tables_join() {
        let sql = "SELECT * FROM users JOIN sessions ON users.id = sessions.user_id";
        let tables = extract_tables(sql);
        assert_eq!(tables, vec!["users", "sessions"]);
    }

    #[test]
    fn test_extract_tables_multiple_joins() {
        let sql = "SELECT * FROM users u JOIN posts p ON u.id = p.user_id JOIN comments c ON p.id = c.post_id";
        let tables = extract_tables(sql);
        assert_eq!(tables, vec!["users", "posts", "comments"]);
    }

    #[test]
    fn test_extract_tables_with_schema() {
        let sql = "SELECT * FROM main.users JOIN temp.sessions ON users.id = sessions.user_id";
        let tables = extract_tables(sql);
        assert_eq!(tables, vec!["users", "sessions"]);
    }

    #[test]
    fn test_extract_tables_case_insensitive() {
        let sql = "select * from Users JOIN Sessions on Users.id = Sessions.user_id";
        let tables = extract_tables(sql);
        assert_eq!(tables, vec!["users", "sessions"]);
    }

    #[test]
    fn test_extract_tables_inner_join() {
        let sql = "SELECT * FROM users INNER JOIN posts ON users.id = posts.user_id";
        let tables = extract_tables(sql);
        assert_eq!(tables, vec!["users", "posts"]);
    }

    #[test]
    fn test_extract_tables_left_join() {
        let sql = "SELECT * FROM users LEFT JOIN profiles ON users.id = profiles.user_id";
        let tables = extract_tables(sql);
        assert_eq!(tables, vec!["users", "profiles"]);
    }

    #[test]
    fn test_multi_table_invalidation() {
        let cache = SqlQueryCache::new(10, Duration::from_secs(60));

        // 创建一个涉及多个表的查询
        let key = QueryKey::new(
            "SELECT * FROM users JOIN posts ON users.id = posts.user_id".to_string(),
            vec![],
        );
        cache.insert(
            key.clone(),
            vec![1, 2, 3],
            vec!["users".to_string(), "posts".to_string()],
        );

        // 使 posts 表失效，应该删除这个查询
        cache.invalidate_table("posts");
        assert!(cache.get(&key).is_none());
    }
}
