//! SQLite 数据库管理器
//!
//! 提供 SQLite 数据库的统一管理接口，支持：
//! - 连接池管理（单连接 + Arc<Mutex>）
//! - 查询缓存（集成 SqlQueryCache）
//! - 事务支持
//! - 自动表依赖追踪
//! - 批量操作
//!
//! # 使用示例
//!
//! ```rust
//! use std::path::Path;
//! use std::time::Duration;
//! use crate::data::managers::SqliteManager;
//!
//! // 创建管理器（带缓存）
//! let manager = SqliteManager::with_cache(
//!     Path::new("app.db"),
//!     100,
//!     Duration::from_secs(300)
//! )?;
//!
//! // 执行查询（自动缓存）
//! let rows = manager.query("SELECT * FROM users WHERE id = ?", &["1"])?;
//!
//! // 执行更新（自动失效缓存）
//! manager.execute("UPDATE users SET name = ? WHERE id = ?", &["Alice", "1"])?;
//!
//! // 使用事务
//! manager.transaction(|tx| {
//!     tx.execute("INSERT INTO users (id, name) VALUES (?, ?)", &["2", "Bob"])?;
//!     tx.execute("UPDATE stats SET count = count + 1", &[])?;
//!     Ok(())
//! })?;
//! ```

use crate::data::cache::{extract_tables, QueryKey, SqlQueryCache};
use crate::data::{DataError, Result};
use rusqlite::{params_from_iter, Connection, Row, Transaction};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// SQLite 管理器
///
/// 支持带缓存和无缓存两种模式。
pub struct SqliteManager {
    /// 数据库连接
    conn: Arc<Mutex<Connection>>,
    /// 查询缓存（None 表示无缓存模式）
    cache: Option<SqlQueryCache>,
    /// 数据库路径（用于错误报告）
    db_path: PathBuf,
}

/// 查询结果行（通用 JSON 格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRow {
    pub columns: Vec<String>,
    pub values: Vec<serde_json::Value>,
}

impl SqliteManager {
    /// 创建带缓存的管理器
    ///
    /// # 参数
    ///
    /// - `path`: 数据库文件路径
    /// - `capacity`: 缓存容量（最大查询数）
    /// - `ttl`: 缓存 TTL
    ///
    /// # 示例
    ///
    /// ```rust
    /// use std::time::Duration;
    /// let manager = SqliteManager::with_cache(
    ///     Path::new("app.db"),
    ///     100,
    ///     Duration::from_secs(300)
    /// )?;
    /// ```
    pub fn with_cache(path: &Path, capacity: usize, ttl: Duration) -> Result<Self> {
        let conn = Self::open_connection(path)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            cache: Some(SqlQueryCache::new(capacity, ttl)),
            db_path: path.to_path_buf(),
        })
    }

    /// 创建无缓存的管理器
    ///
    /// # 示例
    ///
    /// ```rust
    /// let manager = SqliteManager::without_cache(Path::new("app.db"))?;
    /// ```
    pub fn without_cache(path: &Path) -> Result<Self> {
        let conn = Self::open_connection(path)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            cache: None,
            db_path: path.to_path_buf(),
        })
    }

    /// 打开数据库连接
    fn open_connection(path: &Path) -> Result<Connection> {
        // 创建父目录
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DataError::io(parent.to_path_buf(), e))?;
        }

        Connection::open(path).map_err(DataError::Database)
    }

    /// 执行查询（返回通用行格式）
    ///
    /// # 参数
    ///
    /// - `sql`: SQL 查询语句
    /// - `params`: 查询参数
    ///
    /// # 示例
    ///
    /// ```rust
    /// let rows = manager.query("SELECT * FROM users WHERE age > ?", &["18"])?;
    /// for row in rows {
    ///     println!("{:?}", row);
    /// }
    /// ```
    pub fn query(&self, sql: &str, params: &[&str]) -> Result<Vec<QueryRow>> {
        // 尝试从缓存获取
        if let Some(cache) = &self.cache {
            let cache_key = QueryKey::new(
                sql.to_string(),
                params.iter().map(|s| s.to_string()).collect(),
            );
            if let Some(cached_bytes) = cache.get(&cache_key) {
                // 反序列化缓存数据（使用 serde_json 而不是 bincode）
                let cached_str = std::str::from_utf8(&cached_bytes)
                    .map_err(|e| DataError::CacheValidation(e.to_string()))?;
                return serde_json::from_str(cached_str)
                    .map_err(|e| DataError::CacheValidation(e.to_string()));
            }
        }

        // 缓存未命中，执行查询
        let conn = self
            .conn
            .lock()
            .map_err(|e| DataError::Concurrency(e.to_string()))?;

        let mut stmt = conn.prepare(sql).map_err(DataError::Database)?;

        // 获取列名
        let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

        let column_count = column_names.len();

        // 执行查询
        let rows = stmt
            .query_map(params_from_iter(params.iter()), |row| {
                Self::row_to_query_row(row, &column_names, column_count)
            })
            .map_err(DataError::Database)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(DataError::Database)?;

        // 插入缓存（使用 serde_json 序列化）
        if let Some(cache) = &self.cache {
            let cache_key = QueryKey::new(
                sql.to_string(),
                params.iter().map(|s| s.to_string()).collect(),
            );
            let serialized =
                serde_json::to_vec(&rows).map_err(|e| DataError::CacheValidation(e.to_string()))?;
            let tables = extract_tables(sql);
            cache.insert(cache_key, serialized, tables);
        }

        Ok(rows)
    }

    /// 将 rusqlite::Row 转换为 QueryRow
    fn row_to_query_row(
        row: &Row,
        column_names: &[String],
        column_count: usize,
    ) -> rusqlite::Result<QueryRow> {
        let mut values = Vec::with_capacity(column_count);

        for i in 0..column_count {
            let value = Self::get_value_as_json(row, i)?;
            values.push(value);
        }

        Ok(QueryRow {
            columns: column_names.to_vec(),
            values,
        })
    }

    /// 从 Row 中获取 JSON 值
    fn get_value_as_json(row: &Row, idx: usize) -> rusqlite::Result<serde_json::Value> {
        use rusqlite::types::ValueRef;

        match row.get_ref(idx)? {
            ValueRef::Null => Ok(serde_json::Value::Null),
            ValueRef::Integer(i) => Ok(serde_json::Value::Number(i.into())),
            ValueRef::Real(f) => Ok(serde_json::Number::from_f64(f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)),
            ValueRef::Text(s) => {
                let text = std::str::from_utf8(s).unwrap_or("");
                Ok(serde_json::Value::String(text.to_string()))
            }
            ValueRef::Blob(b) => Ok(serde_json::Value::String(format!(
                "<blob {} bytes>",
                b.len()
            ))),
        }
    }

    /// 执行更新/插入/删除（自动失效缓存）
    ///
    /// # 参数
    ///
    /// - `sql`: SQL 语句
    /// - `params`: 参数
    ///
    /// # 返回
    ///
    /// 受影响的行数
    ///
    /// # 示例
    ///
    /// ```rust
    /// let affected = manager.execute(
    ///     "UPDATE users SET name = ? WHERE id = ?",
    ///     &["Alice", "1"]
    /// )?;
    /// println!("Updated {} rows", affected);
    /// ```
    pub fn execute(&self, sql: &str, params: &[&str]) -> Result<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| DataError::Concurrency(e.to_string()))?;

        let affected = conn
            .execute(sql, params_from_iter(params.iter()))
            .map_err(DataError::Database)?;

        // 失效相关表的缓存
        if let Some(cache) = &self.cache {
            let tables = extract_tables(sql);
            for table in &tables {
                cache.invalidate_table(table);
            }
        }

        Ok(affected)
    }

    /// 执行批量更新
    ///
    /// # 参数
    ///
    /// - `sql`: SQL 语句
    /// - `params_list`: 参数列表（每个元素是一组参数）
    ///
    /// # 返回
    ///
    /// 每次执行受影响的行数
    pub fn execute_batch(&self, sql: &str, params_list: &[Vec<String>]) -> Result<Vec<usize>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| DataError::Concurrency(e.to_string()))?;

        let mut results = Vec::new();
        for params in params_list {
            let param_refs: Vec<&str> = params.iter().map(|s| s.as_str()).collect();
            let affected = conn
                .execute(sql, params_from_iter(param_refs.iter()))
                .map_err(DataError::Database)?;
            results.push(affected);
        }

        // 失效相关表的缓存
        if let Some(cache) = &self.cache {
            let tables = extract_tables(sql);
            for table in &tables {
                cache.invalidate_table(table);
            }
        }

        Ok(results)
    }

    /// 执行事务
    ///
    /// # 参数
    ///
    /// - `f`: 事务函数，接收 `&Transaction` 并返回 `Result<T>`
    ///
    /// # 示例
    ///
    /// ```rust
    /// manager.transaction(|tx| {
    ///     tx.execute("INSERT INTO users (id, name) VALUES (?, ?)", &["1", "Alice"])?;
    ///     tx.execute("UPDATE stats SET count = count + 1", &[])?;
    ///     Ok(())
    /// })?;
    /// ```
    pub fn transaction<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Transaction) -> Result<T>,
    {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| DataError::Concurrency(e.to_string()))?;

        let tx = conn.transaction().map_err(DataError::Database)?;
        let result = f(&tx)?;
        tx.commit().map_err(DataError::Database)?;

        // 清空所有缓存（事务可能影响多张表）
        if let Some(cache) = &self.cache {
            cache.clear();
        }

        Ok(result)
    }

    /// 检查表是否存在
    ///
    /// # 参数
    ///
    /// - `table_name`: 表名
    pub fn table_exists(&self, table_name: &str) -> Result<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| DataError::Concurrency(e.to_string()))?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
                [table_name],
                |row| row.get(0),
            )
            .map_err(DataError::Database)?;

        Ok(count > 0)
    }

    /// 执行原始 SQL（用于 DDL 等操作）
    ///
    /// # 参数
    ///
    /// - `sql`: SQL 语句
    pub fn execute_raw(&self, sql: &str) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| DataError::Concurrency(e.to_string()))?;

        conn.execute_batch(sql).map_err(DataError::Database)?;

        // 清空所有缓存（DDL 可能影响表结构）
        if let Some(cache) = &self.cache {
            cache.clear();
        }

        Ok(())
    }

    /// 清空缓存
    pub fn clear_cache(&self) {
        if let Some(cache) = &self.cache {
            cache.clear();
        }
    }

    /// 使指定表的缓存失效
    ///
    /// # 参数
    ///
    /// - `table_name`: 表名
    pub fn invalidate_table(&self, table_name: &str) {
        if let Some(cache) = &self.cache {
            cache.invalidate_table(table_name);
        }
    }

    /// 获取数据库路径
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}

impl Default for SqliteManager {
    fn default() -> Self {
        // 默认使用内存数据库（用于测试）
        Self::without_cache(Path::new(":memory:")).expect("Failed to create in-memory database")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_db() -> (TempDir, SqliteManager) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let manager = SqliteManager::with_cache(&db_path, 100, Duration::from_secs(60)).unwrap();

        // 创建测试表
        manager
            .execute_raw(
                "CREATE TABLE users (
                    id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    age INTEGER
                )",
            )
            .unwrap();

        (temp_dir, manager)
    }

    #[test]
    fn test_create_manager() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let manager = SqliteManager::with_cache(&db_path, 100, Duration::from_secs(60)).unwrap();

        assert!(manager.db_path().exists());
    }

    #[test]
    fn test_execute_and_query() {
        let (_temp_dir, manager) = create_test_db();

        // 插入数据
        let affected = manager
            .execute(
                "INSERT INTO users (id, name, age) VALUES (?, ?, ?)",
                &["1", "Alice", "30"],
            )
            .unwrap();
        assert_eq!(affected, 1);

        // 查询数据
        let rows = manager
            .query("SELECT * FROM users WHERE id = ?", &["1"])
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].columns, vec!["id", "name", "age"]);
        assert_eq!(
            rows[0].values[1],
            serde_json::Value::String("Alice".to_string())
        );
    }

    #[test]
    fn test_query_cache() {
        let (_temp_dir, manager) = create_test_db();

        manager
            .execute(
                "INSERT INTO users (id, name, age) VALUES (?, ?, ?)",
                &["1", "Alice", "30"],
            )
            .unwrap();

        // 第一次查询（缓存未命中）
        let rows1 = manager
            .query("SELECT * FROM users WHERE id = ?", &["1"])
            .unwrap();

        // 第二次查询（缓存命中）
        let rows2 = manager
            .query("SELECT * FROM users WHERE id = ?", &["1"])
            .unwrap();

        assert_eq!(rows1.len(), rows2.len());
    }

    #[test]
    fn test_cache_invalidation() {
        let (_temp_dir, manager) = create_test_db();

        manager
            .execute(
                "INSERT INTO users (id, name, age) VALUES (?, ?, ?)",
                &["1", "Alice", "30"],
            )
            .unwrap();

        // 查询并缓存
        let rows1 = manager.query("SELECT * FROM users", &[]).unwrap();
        assert_eq!(rows1.len(), 1);

        // 更新数据（应该失效缓存）
        manager
            .execute(
                "INSERT INTO users (id, name, age) VALUES (?, ?, ?)",
                &["2", "Bob", "25"],
            )
            .unwrap();

        // 再次查询（应该返回新数据）
        let rows2 = manager.query("SELECT * FROM users", &[]).unwrap();
        assert_eq!(rows2.len(), 2);
    }

    #[test]
    fn test_transaction() {
        let (_temp_dir, manager) = create_test_db();

        // 成功事务
        manager
            .transaction(|tx| {
                tx.execute(
                    "INSERT INTO users (id, name, age) VALUES (?, ?, ?)",
                    ["1", "Alice", "30"],
                )?;
                tx.execute(
                    "INSERT INTO users (id, name, age) VALUES (?, ?, ?)",
                    ["2", "Bob", "25"],
                )?;
                Ok(())
            })
            .unwrap();

        let rows = manager.query("SELECT * FROM users", &[]).unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_transaction_rollback() {
        let (_temp_dir, manager) = create_test_db();

        // 失败事务（应该回滚）
        let result: Result<()> = manager.transaction(|tx| {
            tx.execute(
                "INSERT INTO users (id, name, age) VALUES (?, ?, ?)",
                ["1", "Alice", "30"],
            )?;
            // 故意违反主键约束
            tx.execute(
                "INSERT INTO users (id, name, age) VALUES (?, ?, ?)",
                ["1", "Bob", "25"],
            )?;
            Ok(())
        });

        assert!(result.is_err());

        // 验证没有插入任何数据
        let rows = manager.query("SELECT * FROM users", &[]).unwrap();
        assert_eq!(rows.len(), 0);
    }

    #[test]
    fn test_execute_batch() {
        let (_temp_dir, manager) = create_test_db();

        let params_list = vec![
            vec!["1".to_string(), "Alice".to_string(), "30".to_string()],
            vec!["2".to_string(), "Bob".to_string(), "25".to_string()],
            vec!["3".to_string(), "Charlie".to_string(), "35".to_string()],
        ];

        let results = manager
            .execute_batch(
                "INSERT INTO users (id, name, age) VALUES (?, ?, ?)",
                &params_list,
            )
            .unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(results, vec![1, 1, 1]);

        let rows = manager.query("SELECT * FROM users", &[]).unwrap();
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn test_table_exists() {
        let (_temp_dir, manager) = create_test_db();

        assert!(manager.table_exists("users").unwrap());
        assert!(!manager.table_exists("nonexistent").unwrap());
    }

    #[test]
    fn test_without_cache() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let manager = SqliteManager::without_cache(&db_path).unwrap();

        manager
            .execute_raw("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)")
            .unwrap();

        manager
            .execute("INSERT INTO test (id, value) VALUES (?, ?)", &["1", "test"])
            .unwrap();

        let rows = manager.query("SELECT * FROM test", &[]).unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_clear_cache() {
        let (_temp_dir, manager) = create_test_db();

        manager
            .execute(
                "INSERT INTO users (id, name, age) VALUES (?, ?, ?)",
                &["1", "Alice", "30"],
            )
            .unwrap();

        // 查询并缓存
        manager.query("SELECT * FROM users", &[]).unwrap();

        // 清空缓存
        manager.clear_cache();

        // 插入新数据但不触发缓存失效（因为已清空）
        manager
            .execute(
                "INSERT INTO users (id, name, age) VALUES (?, ?, ?)",
                &["2", "Bob", "25"],
            )
            .unwrap();

        let rows = manager.query("SELECT * FROM users", &[]).unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_invalidate_table() {
        let (_temp_dir, manager) = create_test_db();

        manager
            .execute(
                "INSERT INTO users (id, name, age) VALUES (?, ?, ?)",
                &["1", "Alice", "30"],
            )
            .unwrap();

        // 查询并缓存
        manager.query("SELECT * FROM users", &[]).unwrap();

        // 手动失效表缓存
        manager.invalidate_table("users");

        let rows = manager.query("SELECT * FROM users", &[]).unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_query_row_conversion() {
        let (_temp_dir, manager) = create_test_db();

        manager
            .execute(
                "INSERT INTO users (id, name, age) VALUES (?, ?, ?)",
                &["1", "Alice", "30"],
            )
            .unwrap();

        let rows = manager
            .query("SELECT id, name, age FROM users", &[])
            .unwrap();
        assert_eq!(rows.len(), 1);

        let row = &rows[0];
        assert_eq!(row.columns, vec!["id", "name", "age"]);
        assert_eq!(row.values[0], serde_json::Value::Number(1.into()));
        assert_eq!(
            row.values[1],
            serde_json::Value::String("Alice".to_string())
        );
        assert_eq!(row.values[2], serde_json::Value::Number(30.into()));
    }
}
