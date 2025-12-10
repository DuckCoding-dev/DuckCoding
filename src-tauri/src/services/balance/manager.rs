// Balance Manager - 余额监控配置管理服务
//
// 提供余额监控配置的 CRUD 操作，使用 DataManager 统一文件管理

use crate::data::DataManager;
use crate::models::{BalanceConfig, BalanceStore};
use anyhow::{Context, Result};
use std::path::PathBuf;

/// 余额监控管理器
pub struct BalanceManager {
    data_manager: DataManager,
    file_path: PathBuf,
}

impl BalanceManager {
    /// 创建新的 BalanceManager 实例
    pub fn new() -> Result<Self> {
        let home_dir = dirs::home_dir().context("无法获取用户主目录")?;
        let file_path = home_dir.join(".duckcoding").join("balance.json");

        Ok(Self {
            data_manager: DataManager::new(),
            file_path,
        })
    }

    /// 加载存储
    ///
    /// 如果文件不存在，返回默认的空存储
    pub fn load_store(&self) -> Result<BalanceStore> {
        if !self.file_path.exists() {
            tracing::debug!("balance.json 不存在，返回默认空存储");
            return Ok(BalanceStore::default());
        }

        let value = self
            .data_manager
            .json()
            .read(&self.file_path)
            .context("读取 balance.json 失败")?;

        serde_json::from_value(value).context("解析 balance.json 失败")
    }

    /// 保存存储
    ///
    /// 自动创建目录，原子写入
    pub fn save_store(&self, store: &BalanceStore) -> Result<()> {
        let value = serde_json::to_value(store).context("序列化 BalanceStore 失败")?;

        self.data_manager
            .json()
            .write(&self.file_path, &value)
            .context("保存 balance.json 失败")
    }

    /// 添加配置
    ///
    /// 自动设置 created_at 和 updated_at
    pub fn add_config(&self, mut config: BalanceConfig) -> Result<()> {
        let mut store = self.load_store()?;

        // 检查 ID 是否已存在
        if store.configs.iter().any(|c| c.id == config.id) {
            anyhow::bail!("配置 ID 已存在: {}", config.id);
        }

        // 确保时间戳正确
        let now = chrono::Utc::now().timestamp_millis();
        config.created_at = now;
        config.updated_at = now;

        store.configs.push(config);
        self.save_store(&store)?;

        tracing::debug!("已添加配置，当前总数: {}", store.configs.len());
        Ok(())
    }

    /// 更新配置
    ///
    /// 自动更新 updated_at
    pub fn update_config(&self, mut config: BalanceConfig) -> Result<()> {
        let mut store = self.load_store()?;

        let index = store
            .configs
            .iter()
            .position(|c| c.id == config.id)
            .context(format!("未找到配置: {}", config.id))?;

        // 保留 created_at，更新 updated_at
        config.updated_at = chrono::Utc::now().timestamp_millis();

        store.configs[index] = config;
        self.save_store(&store)?;

        tracing::debug!("已更新配置: {}", store.configs[index].id);
        Ok(())
    }

    /// 删除配置
    pub fn delete_config(&self, id: &str) -> Result<()> {
        let mut store = self.load_store()?;

        let original_len = store.configs.len();
        store.configs.retain(|c| c.id != id);

        if store.configs.len() == original_len {
            anyhow::bail!("未找到配置: {}", id);
        }

        self.save_store(&store)?;

        tracing::debug!("已删除配置: {}，剩余 {}", id, store.configs.len());
        Ok(())
    }

    /// 获取单个配置
    pub fn get_config(&self, id: &str) -> Result<Option<BalanceConfig>> {
        let store = self.load_store()?;
        Ok(store.configs.into_iter().find(|c| c.id == id))
    }

    /// 列出所有配置
    ///
    /// 按 updated_at 降序排序（最新的在前）
    pub fn list_configs(&self) -> Result<Vec<BalanceConfig>> {
        let mut store = self.load_store()?;
        store
            .configs
            .sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(store.configs)
    }

    /// 批量保存配置（用于迁移）
    ///
    /// 覆盖现有的所有配置
    pub fn save_all_configs(&self, configs: Vec<BalanceConfig>) -> Result<()> {
        let store = BalanceStore {
            version: 1,
            configs,
        };

        self.save_store(&store)?;
        tracing::info!("已批量保存 {} 个配置", store.configs.len());
        Ok(())
    }

    /// 获取文件路径（用于测试）
    #[cfg(test)]
    pub fn file_path(&self) -> &PathBuf {
        &self.file_path
    }
}

impl Default for BalanceManager {
    fn default() -> Self {
        Self::new().expect("无法创建 BalanceManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn create_test_manager() -> (BalanceManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("balance.json");

        let manager = BalanceManager {
            data_manager: DataManager::new(),
            file_path,
        };

        (manager, temp_dir)
    }

    fn create_test_config(id: &str, name: &str) -> BalanceConfig {
        BalanceConfig {
            id: id.to_string(),
            name: name.to_string(),
            endpoint: "https://api.example.com/balance".to_string(),
            method: "GET".to_string(),
            static_headers: Some(HashMap::from([(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )])),
            extractor_script: "return response.balance;".to_string(),
            interval_sec: Some(300),
            timeout_ms: Some(5000),
            save_api_key: false,
            api_key: None,
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn test_load_store_empty() {
        let (manager, _temp) = create_test_manager();

        let store = manager.load_store().unwrap();
        assert_eq!(store.version, 1);
        assert_eq!(store.configs.len(), 0);
    }

    #[test]
    fn test_add_config() {
        let (manager, _temp) = create_test_manager();

        let config = create_test_config("test-1", "Test Config");
        manager.add_config(config.clone()).unwrap();

        let store = manager.load_store().unwrap();
        assert_eq!(store.configs.len(), 1);
        assert_eq!(store.configs[0].id, "test-1");
        assert_eq!(store.configs[0].name, "Test Config");
        assert!(store.configs[0].created_at > 0);
        assert!(store.configs[0].updated_at > 0);
    }

    #[test]
    fn test_add_duplicate_id() {
        let (manager, _temp) = create_test_manager();

        let config = create_test_config("test-1", "Test Config");
        manager.add_config(config.clone()).unwrap();

        let result = manager.add_config(config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("已存在"));
    }

    #[test]
    fn test_update_config() {
        let (manager, _temp) = create_test_manager();

        let mut config = create_test_config("test-1", "Original Name");
        manager.add_config(config.clone()).unwrap();

        // 修改名称
        config.name = "Updated Name".to_string();
        manager.update_config(config).unwrap();

        let updated = manager.get_config("test-1").unwrap().unwrap();
        assert_eq!(updated.name, "Updated Name");
        assert!(updated.updated_at > updated.created_at);
    }

    #[test]
    fn test_update_nonexistent() {
        let (manager, _temp) = create_test_manager();

        let config = create_test_config("nonexistent", "Test");
        let result = manager.update_config(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_config() {
        let (manager, _temp) = create_test_manager();

        let config = create_test_config("test-1", "Test Config");
        manager.add_config(config).unwrap();

        manager.delete_config("test-1").unwrap();

        let store = manager.load_store().unwrap();
        assert_eq!(store.configs.len(), 0);
    }

    #[test]
    fn test_delete_nonexistent() {
        let (manager, _temp) = create_test_manager();

        let result = manager.delete_config("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_configs_sorted() {
        let (manager, _temp) = create_test_manager();

        // 添加多个配置
        let config1 = create_test_config("test-1", "Config 1");
        manager.add_config(config1).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let config2 = create_test_config("test-2", "Config 2");
        manager.add_config(config2).unwrap();

        let configs = manager.list_configs().unwrap();
        assert_eq!(configs.len(), 2);
        // 最新的在前
        assert_eq!(configs[0].id, "test-2");
        assert_eq!(configs[1].id, "test-1");
    }

    #[test]
    fn test_save_all_configs() {
        let (manager, _temp) = create_test_manager();

        let configs = vec![
            create_test_config("test-1", "Config 1"),
            create_test_config("test-2", "Config 2"),
            create_test_config("test-3", "Config 3"),
        ];

        manager.save_all_configs(configs).unwrap();

        let store = manager.load_store().unwrap();
        assert_eq!(store.configs.len(), 3);
    }
}
