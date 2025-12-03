//! JSON 配置管理器
//!
//! 提供 JSON 配置文件的读写和操作，支持：
//! - 双模式：带缓存（全局配置）和无缓存（工具原生配置）
//! - 键路径访问（支持嵌套键如 "env.API_KEY"）
//! - 深度合并
//! - 自动创建父目录
//! - Unix 权限设置（0o600）
//!
//! # 使用示例
//!
//! ```rust
//! use std::path::Path;
//! use std::time::Duration;
//! use crate::data::managers::JsonManager;
//!
//! // 带缓存模式（用于全局配置）
//! let manager = JsonManager::with_cache(50, Duration::from_secs(300));
//! let config = manager.read(Path::new("~/.duckcoding/config.json"))?;
//!
//! // 无缓存模式（用于工具原生配置）
//! let manager = JsonManager::without_cache();
//! let settings = manager.read(Path::new("~/.claude/settings.json"))?;
//!
//! // 键路径访问
//! manager.set(
//!     Path::new("config.json"),
//!     "env.ANTHROPIC_AUTH_TOKEN",
//!     serde_json::json!("sk-ant-xxx")
//! )?;
//! ```

use crate::data::cache::JsonConfigCache;
use crate::data::{DataError, Result};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::time::Duration;

/// JSON 配置管理器
///
/// 支持带缓存和无缓存两种模式。
pub struct JsonManager {
    /// JSON 配置缓存（None 表示无缓存模式）
    cache: Option<JsonConfigCache>,
}

impl JsonManager {
    /// 创建带缓存的管理器（用于全局配置、Profile 配置等）
    ///
    /// # 参数
    ///
    /// - `capacity`: 缓存容量（最大文件数）
    /// - `ttl`: 缓存 TTL
    ///
    /// # 示例
    ///
    /// ```rust
    /// use std::time::Duration;
    /// let manager = JsonManager::with_cache(50, Duration::from_secs(300));
    /// ```
    pub fn with_cache(capacity: usize, ttl: Duration) -> Self {
        Self {
            cache: Some(JsonConfigCache::new(capacity, ttl)),
        }
    }

    /// 创建无缓存的管理器（用于工具原生配置）
    ///
    /// # 示例
    ///
    /// ```rust
    /// let manager = JsonManager::without_cache();
    /// ```
    pub fn without_cache() -> Self {
        Self { cache: None }
    }

    /// 读取整个 JSON 文件
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    ///
    /// # 返回
    ///
    /// - `Ok(Value)`: JSON 值
    /// - `Err(DataError)`: 读取或解析失败
    pub fn read(&self, path: &Path) -> Result<Value> {
        // 尝试从缓存获取
        if let Some(cache) = &self.cache {
            if let Some(cached_value) = cache.get(path) {
                return Ok(cached_value);
            }
        }

        // 缓存未命中或无缓存模式，从文件读取
        let content = fs::read_to_string(path).map_err(|e| DataError::io(path.to_path_buf(), e))?;

        let value: Value = serde_json::from_str(&content)?;

        // 插入缓存
        if let Some(cache) = &self.cache {
            let checksum =
                compute_checksum(path).map_err(|e| DataError::io(path.to_path_buf(), e))?;
            cache.insert(path.to_path_buf(), value.clone(), checksum);
        }

        Ok(value)
    }

    /// 写入整个 JSON 文件
    ///
    /// 自动创建父目录并设置权限（Unix 平台 0o600）。
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    /// - `value`: JSON 值
    pub fn write(&self, path: &Path, value: &Value) -> Result<()> {
        // 创建父目录
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| DataError::io(parent.to_path_buf(), e))?;
        }

        // 写入文件（格式化输出）
        let content = serde_json::to_string_pretty(value)?;
        fs::write(path, content).map_err(|e| DataError::io(path.to_path_buf(), e))?;

        // 设置权限
        set_permissions(path)?;

        // 使缓存失效（文件已变更）
        if let Some(cache) = &self.cache {
            cache.invalidate(path);
        }

        Ok(())
    }

    /// 获取指定键的值
    ///
    /// 支持嵌套键，如 "env.API_KEY"。
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    /// - `key`: 键路径（使用 `.` 分隔）
    ///
    /// # 示例
    ///
    /// ```rust
    /// let value = manager.get(Path::new("config.json"), "env.ANTHROPIC_AUTH_TOKEN")?;
    /// ```
    pub fn get(&self, path: &Path, key: &str) -> Result<Value> {
        let value = self.read(path)?;
        let key_path = parse_key_path(key);

        get_nested(&value, &key_path)
            .cloned()
            .ok_or_else(|| DataError::NotFound(format!("键 '{}' 不存在", key)))
    }

    /// 设置指定键的值
    ///
    /// 自动创建不存在的中间对象。
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    /// - `key`: 键路径（使用 `.` 分隔）
    /// - `new_value`: 新值
    ///
    /// # 示例
    ///
    /// ```rust
    /// manager.set(
    ///     Path::new("config.json"),
    ///     "env.ANTHROPIC_AUTH_TOKEN",
    ///     serde_json::json!("sk-ant-xxx")
    /// )?;
    /// ```
    pub fn set(&self, path: &Path, key: &str, new_value: Value) -> Result<()> {
        let mut value = if path.exists() {
            self.read(path)?
        } else {
            Value::Object(serde_json::Map::new())
        };

        let key_path = parse_key_path(key);
        set_nested(&mut value, &key_path, new_value)?;

        self.write(path, &value)
    }

    /// 检查文件或键是否存在
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    /// - `key`: 可选的键路径
    ///
    /// # 返回
    ///
    /// - 如果 `key` 为 `None`，检查文件是否存在
    /// - 如果 `key` 为 `Some(k)`，检查键是否存在
    pub fn exists(&self, path: &Path, key: Option<&str>) -> bool {
        if !path.exists() {
            return false;
        }

        if let Some(k) = key {
            if let Ok(value) = self.read(path) {
                let key_path = parse_key_path(k);
                get_nested(&value, &key_path).is_some()
            } else {
                false
            }
        } else {
            true
        }
    }

    /// 删除文件或键
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    /// - `key`: 可选的键路径
    ///
    /// # 返回
    ///
    /// - 如果 `key` 为 `None`，删除整个文件
    /// - 如果 `key` 为 `Some(k)`，删除指定键
    pub fn delete(&self, path: &Path, key: Option<&str>) -> Result<()> {
        if let Some(k) = key {
            // 删除指定键
            let mut value = self.read(path)?;
            let key_path = parse_key_path(k);
            delete_nested(&mut value, &key_path)?;
            self.write(path, &value)
        } else {
            // 删除整个文件
            fs::remove_file(path).map_err(|e| DataError::io(path.to_path_buf(), e))?;

            // 使缓存失效
            if let Some(cache) = &self.cache {
                cache.invalidate(path);
            }

            Ok(())
        }
    }

    /// 深度合并 JSON 对象
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    /// - `patch`: 要合并的 JSON 值
    pub fn merge(&self, path: &Path, patch: &Value) -> Result<()> {
        let mut value = if path.exists() {
            self.read(path)?
        } else {
            Value::Object(serde_json::Map::new())
        };

        merge_values(&mut value, patch);
        self.write(path, &value)
    }

    /// 清空缓存（仅缓存模式有效）
    pub fn clear_cache(&self) {
        if let Some(cache) = &self.cache {
            cache.clear();
        }
    }
}

/// 解析键路径
///
/// # 示例
///
/// ```rust
/// let path = parse_key_path("env.ANTHROPIC_AUTH_TOKEN");
/// assert_eq!(path, vec!["env", "ANTHROPIC_AUTH_TOKEN"]);
/// ```
fn parse_key_path(key: &str) -> Vec<&str> {
    key.split('.').collect()
}

/// 获取嵌套值
fn get_nested<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for segment in path {
        current = current.get(segment)?;
    }
    Some(current)
}

/// 设置嵌套值
fn set_nested(value: &mut Value, path: &[&str], new_value: Value) -> Result<()> {
    if path.is_empty() {
        return Err(DataError::InvalidKey("空键路径".into()));
    }

    // 确保根是对象
    if !value.is_object() {
        *value = Value::Object(serde_json::Map::new());
    }

    let mut current = value;
    for &segment in &path[..path.len() - 1] {
        // 确保中间路径都是对象
        if !current.is_object() {
            *current = Value::Object(serde_json::Map::new());
        }

        current = current
            .as_object_mut()
            .unwrap()
            .entry(segment)
            .or_insert(Value::Object(serde_json::Map::new()));
    }

    // 设置最终值
    if let Some(obj) = current.as_object_mut() {
        obj.insert(path[path.len() - 1].to_string(), new_value);
    }

    Ok(())
}

/// 删除嵌套值
fn delete_nested(value: &mut Value, path: &[&str]) -> Result<()> {
    if path.is_empty() {
        return Err(DataError::InvalidKey("空键路径".into()));
    }

    if path.len() == 1 {
        // 直接删除
        if let Some(obj) = value.as_object_mut() {
            obj.remove(path[0]);
        }
        return Ok(());
    }

    // 递归到父对象
    let mut current = value;
    for &segment in &path[..path.len() - 1] {
        current = current
            .get_mut(segment)
            .ok_or_else(|| DataError::NotFound(format!("键路径 '{}' 不存在", path.join("."))))?;
    }

    // 删除最终键
    if let Some(obj) = current.as_object_mut() {
        obj.remove(path[path.len() - 1]);
    }

    Ok(())
}

/// 深度合并 JSON 值
fn merge_values(target: &mut Value, source: &Value) {
    match (target, source) {
        (Value::Object(target_obj), Value::Object(source_obj)) => {
            for (key, value) in source_obj {
                if let Some(target_value) = target_obj.get_mut(key) {
                    // 递归合并
                    merge_values(target_value, value);
                } else {
                    // 插入新键
                    target_obj.insert(key.clone(), value.clone());
                }
            }
        }
        (target, source) => {
            // 非对象类型，直接替换
            *target = source.clone();
        }
    }
}

/// 计算文件 SHA-256 校验和
fn compute_checksum(path: &Path) -> std::io::Result<String> {
    let content = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(format!("{:x}", hasher.finalize()))
}

/// 设置文件权限（Unix 平台 0o600）
#[cfg(unix)]
fn set_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = fs::metadata(path).map_err(|e| DataError::io(path.to_path_buf(), e))?;
    let mut perms = metadata.permissions();
    perms.set_mode(0o600);
    fs::set_permissions(path, perms).map_err(|e| DataError::io(path.to_path_buf(), e))
}

#[cfg(not(unix))]
fn set_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn test_parse_key_path() {
        assert_eq!(parse_key_path("key"), vec!["key"]);
        assert_eq!(parse_key_path("env.API_KEY"), vec!["env", "API_KEY"]);
        assert_eq!(parse_key_path("a.b.c.d"), vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn test_get_nested() {
        let value = json!({
            "env": {
                "API_KEY": "secret"
            }
        });

        assert_eq!(
            get_nested(&value, &["env", "API_KEY"]),
            Some(&json!("secret"))
        );
        assert_eq!(
            get_nested(&value, &["env"]),
            Some(&json!({"API_KEY": "secret"}))
        );
        assert_eq!(get_nested(&value, &["missing"]), None);
    }

    #[test]
    fn test_set_nested() {
        let mut value = json!({});
        set_nested(&mut value, &["env", "API_KEY"], json!("secret")).unwrap();

        assert_eq!(value, json!({"env": {"API_KEY": "secret"}}));
    }

    #[test]
    fn test_set_nested_create_intermediate() {
        let mut value = json!({});
        set_nested(&mut value, &["a", "b", "c"], json!("value")).unwrap();

        assert_eq!(value, json!({"a": {"b": {"c": "value"}}}));
    }

    #[test]
    fn test_delete_nested() {
        let mut value = json!({
            "env": {
                "API_KEY": "secret",
                "OTHER": "value"
            }
        });

        delete_nested(&mut value, &["env", "API_KEY"]).unwrap();
        assert_eq!(value, json!({"env": {"OTHER": "value"}}));
    }

    #[test]
    fn test_merge_values() {
        let mut target = json!({
            "a": 1,
            "b": {
                "c": 2
            }
        });

        let source = json!({
            "b": {
                "d": 3
            },
            "e": 4
        });

        merge_values(&mut target, &source);

        assert_eq!(
            target,
            json!({
                "a": 1,
                "b": {
                    "c": 2,
                    "d": 3
                },
                "e": 4
            })
        );
    }

    #[test]
    fn test_read_write_without_cache() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");

        let manager = JsonManager::without_cache();
        let content = json!({"key": "value"});

        // 写入
        manager.write(&file_path, &content).unwrap();

        // 读取
        let read_content = manager.read(&file_path).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_read_write_with_cache() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");

        let manager = JsonManager::with_cache(10, Duration::from_secs(60));
        let content = json!({"key": "value"});

        // 写入
        manager.write(&file_path, &content).unwrap();

        // 第一次读取（缓存未命中）
        let read1 = manager.read(&file_path).unwrap();
        assert_eq!(read1, content);

        // 第二次读取（缓存命中）
        let read2 = manager.read(&file_path).unwrap();
        assert_eq!(read2, content);
    }

    #[test]
    fn test_get_set() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");

        let manager = JsonManager::without_cache();

        // 设置值
        manager
            .set(&file_path, "env.API_KEY", json!("secret"))
            .unwrap();

        // 获取值
        let value = manager.get(&file_path, "env.API_KEY").unwrap();
        assert_eq!(value, json!("secret"));
    }

    #[test]
    fn test_exists() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");

        let manager = JsonManager::without_cache();

        // 文件不存在
        assert!(!manager.exists(&file_path, None));

        // 写入文件
        manager.write(&file_path, &json!({"key": "value"})).unwrap();

        // 文件存在
        assert!(manager.exists(&file_path, None));
        assert!(manager.exists(&file_path, Some("key")));
        assert!(!manager.exists(&file_path, Some("missing")));
    }

    #[test]
    fn test_delete_key() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");

        let manager = JsonManager::without_cache();
        manager.write(&file_path, &json!({"a": 1, "b": 2})).unwrap();

        // 删除键
        manager.delete(&file_path, Some("a")).unwrap();

        let content = manager.read(&file_path).unwrap();
        assert_eq!(content, json!({"b": 2}));
    }

    #[test]
    fn test_delete_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");

        let manager = JsonManager::without_cache();
        manager.write(&file_path, &json!({"key": "value"})).unwrap();

        assert!(file_path.exists());

        // 删除文件
        manager.delete(&file_path, None).unwrap();

        assert!(!file_path.exists());
    }

    #[test]
    fn test_merge() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");

        let manager = JsonManager::without_cache();
        manager
            .write(&file_path, &json!({"a": 1, "b": {"c": 2}}))
            .unwrap();

        // 合并
        manager
            .merge(&file_path, &json!({"b": {"d": 3}, "e": 4}))
            .unwrap();

        let content = manager.read(&file_path).unwrap();
        assert_eq!(content, json!({"a": 1, "b": {"c": 2, "d": 3}, "e": 4}));
    }

    #[test]
    fn test_clear_cache() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");

        let manager = JsonManager::with_cache(10, Duration::from_secs(60));
        manager.write(&file_path, &json!({"key": "value"})).unwrap();

        // 读取以填充缓存
        manager.read(&file_path).unwrap();

        // 清空缓存
        manager.clear_cache();

        // 缓存应该已清空（下次读取会重新从文件加载）
        let content = manager.read(&file_path).unwrap();
        assert_eq!(content, json!({"key": "value"}));
    }

    #[test]
    fn test_auto_create_parent_dir() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("subdir").join("config.json");

        let manager = JsonManager::without_cache();
        manager.write(&file_path, &json!({"key": "value"})).unwrap();

        assert!(file_path.exists());
    }

    #[test]
    #[cfg(unix)]
    fn test_permissions_unix() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");

        let manager = JsonManager::without_cache();
        manager.write(&file_path, &json!({"key": "value"})).unwrap();

        let metadata = fs::metadata(&file_path).unwrap();
        let perms = metadata.permissions();
        assert_eq!(perms.mode() & 0o777, 0o600);
    }
}
