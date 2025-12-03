//! TOML 配置管理器
//!
//! 提供 TOML 配置文件的读写和操作，支持：
//! - 保留注释和格式（使用 `toml_edit`）
//! - 键路径访问（支持嵌套键如 "model_providers.duckcoding.base_url"）
//! - 深度合并
//! - 自动创建父目录
//! - Unix 权限设置（0o600）
//!
//! # 使用示例
//!
//! ```rust
//! use std::path::Path;
//! use crate::data::managers::TomlManager;
//!
//! let manager = TomlManager::new();
//!
//! // 读取 TOML 文件
//! let config = manager.read(Path::new("config.toml"))?;
//!
//! // 编辑并保留注释
//! let mut doc = manager.read_document(Path::new("config.toml"))?;
//! doc["key"] = toml_edit::value("new_value");
//! manager.write(Path::new("config.toml"), &doc)?;
//! ```

use crate::data::{DataError, Result};
use std::fs;
use std::path::Path;
use toml::Value as TomlValue;
use toml_edit::{DocumentMut, Item, Table, Value as EditValue};

/// TOML 配置管理器
///
/// 使用 `toml_edit` 保留注释和格式。
pub struct TomlManager;

impl TomlManager {
    /// 创建新的 TOML 管理器
    pub fn new() -> Self {
        Self
    }

    /// 读取整个 TOML 文件（返回 `toml::Value`）
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    ///
    /// # 返回
    ///
    /// - `Ok(TomlValue)`: TOML 值
    /// - `Err(DataError)`: 读取或解析失败
    pub fn read(&self, path: &Path) -> Result<TomlValue> {
        let content = fs::read_to_string(path).map_err(|e| DataError::io(path.to_path_buf(), e))?;

        toml::from_str(&content).map_err(Into::into)
    }

    /// 读取为可编辑文档（返回 `toml_edit::DocumentMut`）
    ///
    /// 使用此方法保留注释和格式。
    pub fn read_document(&self, path: &Path) -> Result<DocumentMut> {
        let content = fs::read_to_string(path).map_err(|e| DataError::io(path.to_path_buf(), e))?;

        content
            .parse::<DocumentMut>()
            .map_err(|e| DataError::TomlEdit(e.to_string()))
    }

    /// 写入 TOML 文档
    ///
    /// 自动创建父目录并设置权限（Unix 平台 0o600）。
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    /// - `doc`: TOML 文档
    pub fn write(&self, path: &Path, doc: &DocumentMut) -> Result<()> {
        // 创建父目录
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| DataError::io(parent.to_path_buf(), e))?;
        }

        // 写入文件
        fs::write(path, doc.to_string()).map_err(|e| DataError::io(path.to_path_buf(), e))?;

        // 设置权限
        set_permissions(path)?;

        Ok(())
    }

    /// 获取指定键的值
    ///
    /// 支持嵌套键，如 "model_providers.duckcoding.base_url"。
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    /// - `key`: 键路径（使用 `.` 分隔）
    pub fn get(&self, path: &Path, key: &str) -> Result<TomlValue> {
        let value = self.read(path)?;
        let key_path = parse_key_path(key);

        get_nested(&value, &key_path)
            .cloned()
            .ok_or_else(|| DataError::NotFound(format!("键 '{}' 不存在", key)))
    }

    /// 设置指定键的值（保留注释）
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    /// - `key`: 键路径（使用 `.` 分隔）
    /// - `value`: 新值
    pub fn set(&self, path: &Path, key: &str, value: TomlValue) -> Result<()> {
        let mut doc = if path.exists() {
            self.read_document(path)?
        } else {
            DocumentMut::new()
        };

        let key_path = parse_key_path(key);
        set_nested_in_document(&mut doc, &key_path, value)?;

        self.write(path, &doc)
    }

    /// 检查文件或键是否存在
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

    /// 删除指定键
    pub fn delete(&self, path: &Path, key: &str) -> Result<()> {
        let mut doc = self.read_document(path)?;
        let key_path = parse_key_path(key);
        delete_nested_in_document(&mut doc, &key_path)?;
        self.write(path, &doc)
    }

    /// 深度合并 TOML 表（保留注释）
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    /// - `source_table`: 要合并的表
    pub fn merge_table(&self, path: &Path, source_table: &Table) -> Result<()> {
        let mut doc = if path.exists() {
            self.read_document(path)?
        } else {
            DocumentMut::new()
        };

        merge_toml_tables(doc.as_table_mut(), source_table);
        self.write(path, &doc)
    }
}

impl Default for TomlManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 解析键路径
fn parse_key_path(key: &str) -> Vec<&str> {
    key.split('.').collect()
}

/// 获取嵌套值
fn get_nested<'a>(value: &'a TomlValue, path: &[&str]) -> Option<&'a TomlValue> {
    let mut current = value;
    for segment in path {
        current = current.get(segment)?;
    }
    Some(current)
}

/// 在文档中设置嵌套值
fn set_nested_in_document(doc: &mut DocumentMut, path: &[&str], value: TomlValue) -> Result<()> {
    if path.is_empty() {
        return Err(DataError::InvalidKey("空键路径".into()));
    }

    // 导航到父表
    let mut current_table = doc.as_table_mut();
    for &segment in &path[..path.len() - 1] {
        // 确保路径上的项都是表
        if !current_table.contains_key(segment) {
            current_table[segment] = Item::Table(Table::new());
        }

        current_table = current_table[segment]
            .as_table_mut()
            .ok_or_else(|| DataError::InvalidKey(format!("'{}' 不是表", segment)))?;
    }

    // 设置最终值
    let final_key = path[path.len() - 1];
    current_table[final_key] = toml_value_to_item(value);

    Ok(())
}

/// 在文档中删除嵌套值
fn delete_nested_in_document(doc: &mut DocumentMut, path: &[&str]) -> Result<()> {
    if path.is_empty() {
        return Err(DataError::InvalidKey("空键路径".into()));
    }

    if path.len() == 1 {
        // 直接删除
        doc.as_table_mut().remove(path[0]);
        return Ok(());
    }

    // 导航到父表
    let mut current_table = doc.as_table_mut();
    for &segment in &path[..path.len() - 1] {
        current_table = current_table
            .get_mut(segment)
            .and_then(|item| item.as_table_mut())
            .ok_or_else(|| DataError::NotFound(format!("键路径 '{}' 不存在", path.join("."))))?;
    }

    // 删除最终键
    current_table.remove(path[path.len() - 1]);
    Ok(())
}

/// 深度合并 TOML 表（保留注释）
fn merge_toml_tables(target: &mut Table, source: &Table) {
    // 删除 target 中不存在于 source 的键
    let keys_to_remove: Vec<String> = target
        .iter()
        .map(|(k, _)| k.to_string())
        .filter(|k| !source.contains_key(k))
        .collect();

    for key in keys_to_remove {
        target.remove(&key);
    }

    // 合并或更新键
    for (key, item) in source.iter() {
        match item {
            Item::Table(source_table) => {
                // 递归合并表
                if let Some(target_item) = target.get_mut(key) {
                    if let Some(target_table) = target_item.as_table_mut() {
                        merge_toml_tables(target_table, source_table);
                        continue;
                    }
                }
                target.insert(key, item.clone());
            }
            Item::Value(source_value) => {
                // 保留原有的注释装饰
                if let Some(existing_item) = target.get_mut(key) {
                    if let Some(existing_value) = existing_item.as_value_mut() {
                        let prefix = existing_value.decor().prefix().cloned();
                        let suffix = existing_value.decor().suffix().cloned();
                        *existing_value = source_value.clone();
                        let decor = existing_value.decor_mut();
                        decor.clear();
                        if let Some(pref) = prefix {
                            decor.set_prefix(pref);
                        }
                        if let Some(suf) = suffix {
                            decor.set_suffix(suf);
                        }
                        continue;
                    }
                }
                target.insert(key, item.clone());
            }
            _ => {
                target.insert(key, item.clone());
            }
        }
    }
}

/// 将 `toml::Value` 转换为 `toml_edit::Item`
fn toml_value_to_item(value: TomlValue) -> Item {
    match value {
        TomlValue::String(s) => Item::Value(EditValue::from(s)),
        TomlValue::Integer(i) => Item::Value(EditValue::from(i)),
        TomlValue::Float(f) => Item::Value(EditValue::from(f)),
        TomlValue::Boolean(b) => Item::Value(EditValue::from(b)),
        TomlValue::Datetime(dt) => Item::Value(EditValue::from(dt.to_string())),
        TomlValue::Array(arr) => {
            let mut edit_arr = toml_edit::Array::new();
            for v in arr {
                if let Item::Value(edit_val) = toml_value_to_item(v) {
                    edit_arr.push(edit_val);
                }
            }
            Item::Value(EditValue::Array(edit_arr))
        }
        TomlValue::Table(tbl) => {
            let mut edit_tbl = Table::new();
            for (k, v) in tbl {
                edit_tbl.insert(&k, toml_value_to_item(v));
            }
            Item::Table(edit_tbl)
        }
    }
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
    fn test_read_write() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.toml");

        let manager = TomlManager::new();

        // 创建测试文档
        let mut doc = DocumentMut::new();
        doc["key"] = toml_edit::value("value");

        // 写入
        manager.write(&file_path, &doc).unwrap();

        // 读取
        let read_value = manager.read(&file_path).unwrap();
        assert_eq!(read_value.get("key").unwrap().as_str().unwrap(), "value");
    }

    #[test]
    fn test_preserve_comments() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.toml");

        // 写入带注释的 TOML
        let content = r#"
# This is a comment
key = "value"
"#;
        fs::write(&file_path, content).unwrap();

        let manager = TomlManager::new();

        // 读取并修改
        let mut doc = manager.read_document(&file_path).unwrap();
        doc["key"] = toml_edit::value("new_value");

        // 写回
        manager.write(&file_path, &doc).unwrap();

        // 验证注释仍然存在
        let new_content = fs::read_to_string(&file_path).unwrap();
        assert!(new_content.contains("# This is a comment"));
        assert!(new_content.contains("new_value"));
    }

    #[test]
    fn test_get_set() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.toml");

        let manager = TomlManager::new();

        // 设置值
        manager
            .set(&file_path, "key", TomlValue::String("value".to_string()))
            .unwrap();

        // 获取值
        let value = manager.get(&file_path, "key").unwrap();
        assert_eq!(value.as_str().unwrap(), "value");
    }

    #[test]
    fn test_nested_set() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.toml");

        let manager = TomlManager::new();

        // 设置嵌套值
        manager
            .set(
                &file_path,
                "section.key",
                TomlValue::String("value".to_string()),
            )
            .unwrap();

        // 获取值
        let value = manager.get(&file_path, "section.key").unwrap();
        assert_eq!(value.as_str().unwrap(), "value");
    }

    #[test]
    fn test_exists() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.toml");

        let manager = TomlManager::new();

        // 文件不存在
        assert!(!manager.exists(&file_path, None));

        // 写入文件
        let mut doc = DocumentMut::new();
        doc["key"] = toml_edit::value("value");
        manager.write(&file_path, &doc).unwrap();

        // 文件存在
        assert!(manager.exists(&file_path, None));
        assert!(manager.exists(&file_path, Some("key")));
        assert!(!manager.exists(&file_path, Some("missing")));
    }

    #[test]
    fn test_delete() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.toml");

        let manager = TomlManager::new();

        // 创建文档
        let mut doc = DocumentMut::new();
        doc["a"] = toml_edit::value("1");
        doc["b"] = toml_edit::value("2");
        manager.write(&file_path, &doc).unwrap();

        // 删除键
        manager.delete(&file_path, "a").unwrap();

        // 验证
        let value = manager.read(&file_path).unwrap();
        assert!(value.get("a").is_none());
        assert!(value.get("b").is_some());
    }

    #[test]
    fn test_merge_table() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.toml");

        let manager = TomlManager::new();

        // 初始文档
        let mut doc = DocumentMut::new();
        doc["a"] = toml_edit::value(1);
        let mut section = Table::new();
        section["c"] = toml_edit::value(2);
        doc["b"] = Item::Table(section);
        manager.write(&file_path, &doc).unwrap();

        // 合并表
        let mut source = Table::new();
        let mut source_section = Table::new();
        source_section["d"] = toml_edit::value(3);
        source["b"] = Item::Table(source_section);
        source["e"] = toml_edit::value(4);

        manager.merge_table(&file_path, &source).unwrap();

        // 验证：合并后只保留 source 中的键
        let value = manager.read(&file_path).unwrap();

        // a 应该被删除（因为不在 source 中）
        assert!(value.get("a").is_none());

        // b 表应该包含 d（从 source），但 c 也应该被删除了（因为 source 的 b 表中没有 c）
        assert_eq!(
            value
                .get("b")
                .unwrap()
                .get("d")
                .unwrap()
                .as_integer()
                .unwrap(),
            3
        );

        // e 是新添加的
        assert_eq!(value.get("e").unwrap().as_integer().unwrap(), 4);
    }

    #[test]
    fn test_auto_create_parent_dir() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("subdir").join("config.toml");

        let manager = TomlManager::new();

        let mut doc = DocumentMut::new();
        doc["key"] = toml_edit::value("value");
        manager.write(&file_path, &doc).unwrap();

        assert!(file_path.exists());
    }
}
