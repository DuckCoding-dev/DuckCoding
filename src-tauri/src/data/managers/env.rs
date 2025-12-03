//! ENV 文件管理器
//!
//! 提供 ENV 文件的读写和操作，支持：
//! - 保留注释和空行
//! - 键值对操作
//! - 自动创建父目录
//! - Unix 权限设置（0o600）
//!
//! # 使用示例
//!
//! ```rust
//! use std::path::Path;
//! use std::collections::HashMap;
//! use crate::data::managers::EnvManager;
//!
//! let manager = EnvManager::new();
//!
//! // 读取 ENV 文件
//! let env_vars = manager.read(Path::new(".env"))?;
//!
//! // 设置值（保留注释）
//! manager.set(Path::new(".env"), "API_KEY", "secret")?;
//! ```

use crate::data::{DataError, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// ENV 文件管理器
pub struct EnvManager;

impl EnvManager {
    /// 创建新的 ENV 管理器
    pub fn new() -> Self {
        Self
    }

    /// 读取 ENV 文件为键值对
    ///
    /// 忽略注释和空行。
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    ///
    /// # 返回
    ///
    /// - `Ok(HashMap)`: 键值对映射
    /// - `Err(DataError)`: 读取失败
    pub fn read(&self, path: &Path) -> Result<HashMap<String, String>> {
        let content = fs::read_to_string(path).map_err(|e| DataError::io(path.to_path_buf(), e))?;

        let mut pairs = HashMap::new();
        for line in content.lines() {
            if let Some((key, value)) = parse_env_line(line) {
                pairs.insert(key, value);
            }
        }

        Ok(pairs)
    }

    /// 读取为原始行（包含注释和空行）
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    ///
    /// # 返回
    ///
    /// - `Ok(Vec<String>)`: 所有行
    /// - `Err(DataError)`: 读取失败
    pub fn read_raw(&self, path: &Path) -> Result<Vec<String>> {
        let content = fs::read_to_string(path).map_err(|e| DataError::io(path.to_path_buf(), e))?;

        Ok(content.lines().map(String::from).collect())
    }

    /// 写入 ENV 文件
    ///
    /// 自动排序键，并设置权限（Unix 平台 0o600）。
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    /// - `pairs`: 键值对映射
    pub fn write(&self, path: &Path, pairs: &HashMap<String, String>) -> Result<()> {
        // 创建父目录
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| DataError::io(parent.to_path_buf(), e))?;
        }

        // 排序键并生成内容
        let mut keys: Vec<_> = pairs.keys().collect();
        keys.sort();

        let lines: Vec<String> = keys
            .iter()
            .map(|k| format!("{}={}", k, pairs.get(*k).unwrap()))
            .collect();

        let content = lines.join("\n") + "\n";

        // 写入文件
        fs::write(path, content).map_err(|e| DataError::io(path.to_path_buf(), e))?;

        // 设置权限
        set_permissions(path)?;

        Ok(())
    }

    /// 获取指定键的值
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    /// - `key`: 键名
    pub fn get(&self, path: &Path, key: &str) -> Result<String> {
        let pairs = self.read(path)?;
        pairs
            .get(key)
            .cloned()
            .ok_or_else(|| DataError::NotFound(format!("键 '{}' 不存在", key)))
    }

    /// 设置指定键的值（保留其他行）
    ///
    /// 保留注释和空行，只更新或添加指定键。
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    /// - `key`: 键名
    /// - `value`: 值
    pub fn set(&self, path: &Path, key: &str, value: &str) -> Result<()> {
        let mut lines = if path.exists() {
            fs::read_to_string(path)
                .map_err(|e| DataError::io(path.to_path_buf(), e))?
                .lines()
                .map(String::from)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        let mut found = false;
        for line in &mut lines {
            if let Some((k, _)) = parse_env_line(line) {
                if k == key {
                    *line = format!("{}={}", key, value);
                    found = true;
                    break;
                }
            }
        }

        if !found {
            lines.push(format!("{}={}", key, value));
        }

        // 创建父目录
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| DataError::io(parent.to_path_buf(), e))?;
        }

        // 写入文件
        let content = lines.join("\n") + "\n";
        fs::write(path, content).map_err(|e| DataError::io(path.to_path_buf(), e))?;

        // 设置权限
        set_permissions(path)?;

        Ok(())
    }

    /// 检查文件或键是否存在
    pub fn exists(&self, path: &Path, key: Option<&str>) -> bool {
        if !path.exists() {
            return false;
        }

        if let Some(k) = key {
            if let Ok(pairs) = self.read(path) {
                pairs.contains_key(k)
            } else {
                false
            }
        } else {
            true
        }
    }

    /// 删除指定键
    ///
    /// 保留注释和空行，只删除指定键的行。
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    /// - `key`: 键名
    pub fn delete(&self, path: &Path, key: &str) -> Result<()> {
        let lines = fs::read_to_string(path)
            .map_err(|e| DataError::io(path.to_path_buf(), e))?
            .lines()
            .filter(|line| {
                if let Some((k, _)) = parse_env_line(line) {
                    k != key
                } else {
                    true // 保留注释和空行
                }
            })
            .map(String::from)
            .collect::<Vec<_>>();

        let content = lines.join("\n") + "\n";
        fs::write(path, content).map_err(|e| DataError::io(path.to_path_buf(), e))?;

        set_permissions(path)?;

        Ok(())
    }
}

impl Default for EnvManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 解析 ENV 文件的一行
///
/// # 返回
///
/// - `Some((key, value))`: 成功解析
/// - `None`: 注释或空行
fn parse_env_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    let (key, value) = trimmed.split_once('=')?;
    Some((key.trim().to_string(), value.trim().to_string()))
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
    use tempfile::TempDir;

    #[test]
    fn test_parse_env_line() {
        assert_eq!(
            parse_env_line("KEY=value"),
            Some(("KEY".to_string(), "value".to_string()))
        );
        assert_eq!(
            parse_env_line("  KEY  =  value  "),
            Some(("KEY".to_string(), "value".to_string()))
        );
        assert_eq!(parse_env_line("# comment"), None);
        assert_eq!(parse_env_line(""), None);
    }

    #[test]
    fn test_read_write() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join(".env");

        let manager = EnvManager::new();

        let mut pairs = HashMap::new();
        pairs.insert("KEY1".to_string(), "value1".to_string());
        pairs.insert("KEY2".to_string(), "value2".to_string());

        // 写入
        manager.write(&file_path, &pairs).unwrap();

        // 读取
        let read_pairs = manager.read(&file_path).unwrap();
        assert_eq!(read_pairs, pairs);
    }

    #[test]
    fn test_preserve_comments() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join(".env");

        // 写入带注释的 ENV 文件
        let content = r#"# This is a comment
KEY1=value1

# Another comment
KEY2=value2
"#;
        fs::write(&file_path, content).unwrap();

        let manager = EnvManager::new();

        // 设置值
        manager.set(&file_path, "KEY1", "new_value").unwrap();

        // 验证注释仍然存在
        let new_content = fs::read_to_string(&file_path).unwrap();
        assert!(new_content.contains("# This is a comment"));
        assert!(new_content.contains("# Another comment"));
        assert!(new_content.contains("KEY1=new_value"));
        assert!(new_content.contains("KEY2=value2"));
    }

    #[test]
    fn test_get_set() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join(".env");

        let manager = EnvManager::new();

        // 设置值
        manager.set(&file_path, "KEY", "value").unwrap();

        // 获取值
        let value = manager.get(&file_path, "KEY").unwrap();
        assert_eq!(value, "value");
    }

    #[test]
    fn test_exists() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join(".env");

        let manager = EnvManager::new();

        // 文件不存在
        assert!(!manager.exists(&file_path, None));

        // 写入文件
        let mut pairs = HashMap::new();
        pairs.insert("KEY".to_string(), "value".to_string());
        manager.write(&file_path, &pairs).unwrap();

        // 文件存在
        assert!(manager.exists(&file_path, None));
        assert!(manager.exists(&file_path, Some("KEY")));
        assert!(!manager.exists(&file_path, Some("MISSING")));
    }

    #[test]
    fn test_delete() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join(".env");

        let manager = EnvManager::new();

        // 写入文件
        let content = r#"# Comment
KEY1=value1
KEY2=value2
"#;
        fs::write(&file_path, content).unwrap();

        // 删除键
        manager.delete(&file_path, "KEY1").unwrap();

        // 验证
        let pairs = manager.read(&file_path).unwrap();
        assert!(!pairs.contains_key("KEY1"));
        assert!(pairs.contains_key("KEY2"));

        // 验证注释仍然存在
        let new_content = fs::read_to_string(&file_path).unwrap();
        assert!(new_content.contains("# Comment"));
    }

    #[test]
    fn test_read_raw() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join(".env");

        let content = r#"# Comment
KEY1=value1

KEY2=value2
"#;
        fs::write(&file_path, content).unwrap();

        let manager = EnvManager::new();
        let lines = manager.read_raw(&file_path).unwrap();

        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], "# Comment");
        assert_eq!(lines[1], "KEY1=value1");
        assert_eq!(lines[2], "");
        assert_eq!(lines[3], "KEY2=value2");
    }

    #[test]
    fn test_auto_create_parent_dir() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("subdir").join(".env");

        let manager = EnvManager::new();

        let mut pairs = HashMap::new();
        pairs.insert("KEY".to_string(), "value".to_string());
        manager.write(&file_path, &pairs).unwrap();

        assert!(file_path.exists());
    }

    #[test]
    fn test_set_new_key() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join(".env");

        let manager = EnvManager::new();

        // 创建文件
        manager.set(&file_path, "KEY1", "value1").unwrap();

        // 添加新键
        manager.set(&file_path, "KEY2", "value2").unwrap();

        // 验证
        let pairs = manager.read(&file_path).unwrap();
        assert_eq!(pairs.get("KEY1").unwrap(), "value1");
        assert_eq!(pairs.get("KEY2").unwrap(), "value2");
    }
}
