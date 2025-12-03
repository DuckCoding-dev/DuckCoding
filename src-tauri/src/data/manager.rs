//! 统一数据管理入口
//!
//! 提供所有数据管理器的统一访问接口，支持：
//! - 双 JSON 管理器模式（缓存 vs 实时）
//! - SQLite 连接池管理
//! - 统一缓存配置
//! - 线程安全设计
//!
//! # 使用示例
//!
//! ```rust
//! use std::path::Path;
//! use crate::data::DataManager;
//!
//! // 创建管理器（默认配置）
//! let manager = DataManager::new();
//!
//! // 读取全局配置（使用缓存）
//! let config = manager.json().read(Path::new("config.json"))?;
//!
//! // 读取工具配置（实时模式）
//! let settings = manager.json_uncached().read(Path::new("settings.json"))?;
//!
//! // 访问 SQLite 数据库
//! let db = manager.sqlite(Path::new("app.db"))?;
//! let rows = db.query("SELECT * FROM users", &[])?;
//! ```

use crate::data::managers::{EnvManager, JsonManager, SqliteManager, TomlManager};
use crate::data::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// 统一数据管理器
///
/// 提供所有数据格式管理器的统一访问接口。
pub struct DataManager {
    /// 带缓存的 JSON 管理器（用于全局配置、Profile 配置等）
    json_cached: Arc<JsonManager>,
    /// 无缓存的 JSON 管理器（用于工具原生配置）
    json_uncached: Arc<JsonManager>,
    /// TOML 管理器
    toml: Arc<TomlManager>,
    /// ENV 管理器
    env: Arc<EnvManager>,
    /// SQLite 连接池（按路径复用连接）
    sqlite_connections: Arc<RwLock<HashMap<PathBuf, Arc<SqliteManager>>>>,
    /// 缓存配置
    cache_config: CacheConfig,
}

impl DataManager {
    /// 创建默认配置的管理器
    ///
    /// 默认配置：
    /// - JSON 缓存容量：50 项
    /// - JSON 缓存 TTL：5 分钟
    /// - SQLite 缓存容量：100 项
    /// - SQLite 缓存 TTL：5 分钟
    ///
    /// # 示例
    ///
    /// ```rust
    /// let manager = DataManager::new();
    /// ```
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }

    /// 创建自定义缓存配置的管理器
    ///
    /// # 参数
    ///
    /// - `config`: 缓存配置
    ///
    /// # 示例
    ///
    /// ```rust
    /// use std::time::Duration;
    ///
    /// let config = CacheConfig {
    ///     json_capacity: 100,
    ///     json_ttl: Duration::from_secs(600),
    ///     sqlite_capacity: 200,
    ///     sqlite_ttl: Duration::from_secs(600),
    /// };
    /// let manager = DataManager::with_config(config);
    /// ```
    pub fn with_config(config: CacheConfig) -> Self {
        Self {
            json_cached: Arc::new(JsonManager::with_cache(
                config.json_capacity,
                config.json_ttl,
            )),
            json_uncached: Arc::new(JsonManager::without_cache()),
            toml: Arc::new(TomlManager::new()),
            env: Arc::new(EnvManager::new()),
            sqlite_connections: Arc::new(RwLock::new(HashMap::new())),
            cache_config: config,
        }
    }

    /// 获取带缓存的 JSON 管理器（用于全局配置、Profile 配置等）
    ///
    /// **适用场景：**
    /// - 读取全局配置（`~/.duckcoding/config.json`）
    /// - 批量读取 Profile 配置
    /// - 频繁读取的配置文件
    ///
    /// **缓存策略：**
    /// - 使用 SHA-256 校验和验证文件变化
    /// - 文件修改后自动失效缓存
    /// - 缓存容量和 TTL 可配置
    ///
    /// # 示例
    ///
    /// ```rust
    /// let config = manager.json().read(Path::new("~/.duckcoding/config.json"))?;
    /// ```
    pub fn json(&self) -> &JsonManager {
        &self.json_cached
    }

    /// 获取无缓存的 JSON 管理器（用于工具原生配置）
    ///
    /// **适用场景：**
    /// - 读取工具原生配置（`~/.claude/settings.json`、`~/.codex/config.toml` 等）
    /// - 需要实时生效的配置文件
    /// - 用户手动修改的配置文件
    ///
    /// **特点：**
    /// - 每次读取都直接访问文件
    /// - 修改立即生效
    /// - 不占用缓存空间
    ///
    /// # 示例
    ///
    /// ```rust
    /// let settings = manager.json_uncached().read(Path::new("~/.claude/settings.json"))?;
    /// ```
    pub fn json_uncached(&self) -> &JsonManager {
        &self.json_uncached
    }

    /// 获取 TOML 管理器
    ///
    /// **适用场景：**
    /// - 读取 TOML 配置文件
    /// - 需要保留注释和格式的配置文件
    ///
    /// **特点：**
    /// - 使用 `toml_edit` 保留注释和格式
    /// - 支持深度合并
    /// - 支持键路径访问
    ///
    /// # 示例
    ///
    /// ```rust
    /// let config = manager.toml().read(Path::new("config.toml"))?;
    /// ```
    pub fn toml(&self) -> &TomlManager {
        &self.toml
    }

    /// 获取 ENV 管理器
    ///
    /// **适用场景：**
    /// - 读取 `.env` 文件
    /// - 需要保留注释的环境变量文件
    ///
    /// **特点：**
    /// - 保留注释和空行
    /// - 自动排序键
    /// - Unix 权限设置（0o600）
    ///
    /// # 示例
    ///
    /// ```rust
    /// let env_vars = manager.env().read(Path::new(".env"))?;
    /// ```
    pub fn env(&self) -> &EnvManager {
        &self.env
    }

    /// 获取或创建 SQLite 连接
    ///
    /// **连接池特性：**
    /// - 按路径复用连接
    /// - 线程安全访问
    /// - 自动查询缓存
    ///
    /// # 参数
    ///
    /// - `db_path`: 数据库文件路径
    ///
    /// # 返回
    ///
    /// 返回共享的 SQLite 管理器实例
    ///
    /// # 示例
    ///
    /// ```rust
    /// let db = manager.sqlite(Path::new("app.db"))?;
    /// let rows = db.query("SELECT * FROM users", &[])?;
    /// ```
    pub fn sqlite(&self, db_path: &Path) -> Result<Arc<SqliteManager>> {
        let path_buf = db_path.to_path_buf();

        // 读锁检查是否已存在
        {
            let connections = self
                .sqlite_connections
                .read()
                .map_err(|e| crate::data::DataError::Concurrency(e.to_string()))?;
            if let Some(manager) = connections.get(&path_buf) {
                return Ok(Arc::clone(manager));
            }
        }

        // 写锁创建新连接
        let mut connections = self
            .sqlite_connections
            .write()
            .map_err(|e| crate::data::DataError::Concurrency(e.to_string()))?;

        // 双重检查（避免并发创建）
        if let Some(manager) = connections.get(&path_buf) {
            return Ok(Arc::clone(manager));
        }

        let manager = Arc::new(SqliteManager::with_cache(
            &path_buf,
            self.cache_config.sqlite_capacity,
            self.cache_config.sqlite_ttl,
        )?);
        connections.insert(path_buf, Arc::clone(&manager));
        Ok(manager)
    }

    /// 清空所有缓存
    ///
    /// 清空内容包括：
    /// - JSON 缓存管理器的所有缓存
    /// - 所有 SQLite 连接的查询缓存
    ///
    /// # 示例
    ///
    /// ```rust
    /// manager.clear_all_caches();
    /// ```
    pub fn clear_all_caches(&self) {
        // 清空 JSON 缓存
        self.json_cached.clear_cache();

        // 清空所有 SQLite 缓存
        if let Ok(connections) = self.sqlite_connections.read() {
            for manager in connections.values() {
                manager.clear_cache();
            }
        }
    }

    /// 获取缓存配置
    pub fn cache_config(&self) -> &CacheConfig {
        &self.cache_config
    }
}

impl Default for DataManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 缓存配置
///
/// 用于配置各管理器的缓存参数。
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// JSON 缓存容量（最大文件数）
    pub json_capacity: usize,
    /// JSON 缓存 TTL
    pub json_ttl: Duration,
    /// SQLite 缓存容量（最大查询数）
    pub sqlite_capacity: usize,
    /// SQLite 缓存 TTL
    pub sqlite_ttl: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            json_capacity: 50,
            json_ttl: Duration::from_secs(300), // 5 分钟
            sqlite_capacity: 100,
            sqlite_ttl: Duration::from_secs(300), // 5 分钟
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn test_data_manager_creation() {
        let manager = DataManager::new();
        assert_eq!(manager.cache_config().json_capacity, 50);
        assert_eq!(manager.cache_config().sqlite_capacity, 100);
    }

    #[test]
    fn test_data_manager_with_custom_config() {
        let config = CacheConfig {
            json_capacity: 100,
            json_ttl: Duration::from_secs(600),
            sqlite_capacity: 200,
            sqlite_ttl: Duration::from_secs(600),
        };
        let manager = DataManager::with_config(config);
        assert_eq!(manager.cache_config().json_capacity, 100);
        assert_eq!(manager.cache_config().sqlite_capacity, 200);
    }

    #[test]
    fn test_json_cached_manager() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let manager = DataManager::new();

        // 写入配置
        let test_config = json!({"key": "value"});
        manager.json().write(&config_path, &test_config).unwrap();

        // 读取配置（缓存）
        let value = manager.json().read(&config_path).unwrap();
        assert_eq!(value["key"], "value");

        // 再次读取（缓存命中）
        let value2 = manager.json().read(&config_path).unwrap();
        assert_eq!(value2["key"], "value");
    }

    #[test]
    fn test_json_uncached_manager() {
        let temp_dir = TempDir::new().unwrap();
        let settings_path = temp_dir.path().join("settings.json");

        let manager = DataManager::new();

        // 写入配置
        let test_settings = json!({"setting": "value"});
        manager
            .json_uncached()
            .write(&settings_path, &test_settings)
            .unwrap();

        // 读取配置（无缓存）
        let value = manager.json_uncached().read(&settings_path).unwrap();
        assert_eq!(value["setting"], "value");
    }

    #[test]
    fn test_toml_manager() {
        let temp_dir = TempDir::new().unwrap();
        let toml_path = temp_dir.path().join("config.toml");

        let manager = DataManager::new();

        // 设置值
        manager
            .toml()
            .set(&toml_path, "key", toml::Value::String("value".to_string()))
            .unwrap();

        // 获取值
        let value = manager.toml().get(&toml_path, "key").unwrap();
        assert_eq!(value.as_str().unwrap(), "value");
    }

    #[test]
    fn test_env_manager() {
        let temp_dir = TempDir::new().unwrap();
        let env_path = temp_dir.path().join(".env");

        let manager = DataManager::new();

        // 设置值
        manager.env().set(&env_path, "KEY", "value").unwrap();

        // 获取值
        let value = manager.env().get(&env_path, "KEY").unwrap();
        assert_eq!(value, "value");
    }

    #[test]
    fn test_sqlite_connection_reuse() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let manager = DataManager::new();

        // 第一次获取连接
        let db1 = manager.sqlite(&db_path).unwrap();
        db1.execute_raw("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)")
            .unwrap();

        // 第二次获取连接（应该复用）
        let db2 = manager.sqlite(&db_path).unwrap();
        db2.execute("INSERT INTO test (id, value) VALUES (?, ?)", &["1", "test"])
            .unwrap();

        // 验证两个连接是同一个实例
        let rows = db1.query("SELECT * FROM test", &[]).unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_sqlite_multiple_databases() {
        let temp_dir = TempDir::new().unwrap();
        let db1_path = temp_dir.path().join("db1.db");
        let db2_path = temp_dir.path().join("db2.db");

        let manager = DataManager::new();

        // 创建两个数据库
        let db1 = manager.sqlite(&db1_path).unwrap();
        let db2 = manager.sqlite(&db2_path).unwrap();

        db1.execute_raw("CREATE TABLE test1 (id INTEGER PRIMARY KEY)")
            .unwrap();
        db2.execute_raw("CREATE TABLE test2 (id INTEGER PRIMARY KEY)")
            .unwrap();

        // 验证两个数据库独立
        assert!(db1.table_exists("test1").unwrap());
        assert!(!db1.table_exists("test2").unwrap());
        assert!(db2.table_exists("test2").unwrap());
        assert!(!db2.table_exists("test1").unwrap());
    }

    #[test]
    fn test_clear_all_caches() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        let db_path = temp_dir.path().join("test.db");

        let manager = DataManager::new();

        // 填充 JSON 缓存
        let test_config = json!({"key": "value"});
        manager.json().write(&config_path, &test_config).unwrap();
        manager.json().read(&config_path).unwrap();

        // 填充 SQLite 缓存
        let db = manager.sqlite(&db_path).unwrap();
        db.execute_raw("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)")
            .unwrap();
        db.execute("INSERT INTO test (id, value) VALUES (?, ?)", &["1", "test"])
            .unwrap();
        db.query("SELECT * FROM test", &[]).unwrap();

        // 清空所有缓存
        manager.clear_all_caches();

        // 验证缓存已清空（无法直接验证，但确保不抛出异常）
        let value = manager.json().read(&config_path).unwrap();
        assert_eq!(value["key"], "value");

        let rows = db.query("SELECT * FROM test", &[]).unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.json_capacity, 50);
        assert_eq!(config.json_ttl, Duration::from_secs(300));
        assert_eq!(config.sqlite_capacity, 100);
        assert_eq!(config.sqlite_ttl, Duration::from_secs(300));
    }

    #[test]
    fn test_concurrent_sqlite_access() {
        use std::thread;

        let temp_dir = Arc::new(TempDir::new().unwrap());
        let db_path = temp_dir.path().join("test.db");

        let manager = Arc::new(DataManager::new());

        // 主线程创建表
        let db = manager.sqlite(&db_path).unwrap();
        db.execute_raw("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)")
            .unwrap();

        // 多线程并发访问
        let mut handles = vec![];
        for i in 0..5 {
            let manager_clone = Arc::clone(&manager);
            let db_path_clone = db_path.clone();
            let handle = thread::spawn(move || {
                let db = manager_clone.sqlite(&db_path_clone).unwrap();
                db.execute(
                    "INSERT INTO test (id, value) VALUES (?, ?)",
                    &[&i.to_string(), &format!("value{}", i)],
                )
                .unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // 验证所有数据都已插入
        let rows = db.query("SELECT * FROM test", &[]).unwrap();
        assert_eq!(rows.len(), 5);
    }
}
