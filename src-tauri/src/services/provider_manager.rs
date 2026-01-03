// Provider Manager Service
//
// 供应商配置管理服务

use crate::data::DataManager;
use crate::models::provider::{Provider, ProviderStore};
use crate::utils::config::config_dir;
use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// 供应商管理器
pub struct ProviderManager {
    data_manager: Arc<DataManager>,
    store_path: PathBuf,
    cache: Arc<Mutex<Option<ProviderStore>>>,
}

impl ProviderManager {
    /// 创建新的 ProviderManager 实例
    pub fn new() -> Result<Self> {
        let data_manager = Arc::new(DataManager::new());
        let store_path = config_dir()
            .map_err(|e| anyhow::anyhow!("获取配置目录失败: {}", e))?
            .join("providers.json");

        Ok(Self {
            data_manager,
            store_path,
            cache: Arc::new(Mutex::new(None)),
        })
    }

    /// 读取存储（带缓存）
    pub fn load_store(&self) -> Result<ProviderStore> {
        // 检查缓存
        if let Some(cached) = self.cache.lock().unwrap().as_ref() {
            return Ok(cached.clone());
        }

        // 文件不存在则返回默认值（迁移会创建）
        if !self.store_path.exists() {
            tracing::warn!("providers.json 不存在，返回默认配置");
            return Ok(ProviderStore::default());
        }

        // 从文件读取
        let json_value = self.data_manager.json().read(&self.store_path)?;
        let store: ProviderStore = serde_json::from_value(json_value)
            .map_err(|e| anyhow::anyhow!("反序列化 ProviderStore 失败: {}", e))?;

        // 更新缓存
        *self.cache.lock().unwrap() = Some(store.clone());

        Ok(store)
    }

    /// 保存存储
    fn save_store(&self, store: &ProviderStore) -> Result<()> {
        let json_value = serde_json::to_value(store)
            .map_err(|e| anyhow::anyhow!("序列化 ProviderStore 失败: {}", e))?;
        self.data_manager
            .json()
            .write(&self.store_path, &json_value)?;
        *self.cache.lock().unwrap() = Some(store.clone());
        Ok(())
    }

    /// 列出所有供应商
    pub fn list_providers(&self) -> Result<Vec<Provider>> {
        Ok(self.load_store()?.providers)
    }

    /// 创建供应商
    pub fn create_provider(&self, mut provider: Provider) -> Result<Provider> {
        let mut store = self.load_store()?;

        // 检查 ID 冲突
        if store.providers.iter().any(|p| p.id == provider.id) {
            return Err(anyhow!("供应商 ID 已存在: {}", provider.id));
        }

        let now = chrono::Utc::now().timestamp();
        provider.created_at = now;
        provider.updated_at = now;

        store.providers.push(provider.clone());
        store.updated_at = now;

        self.save_store(&store)?;
        Ok(provider)
    }

    /// 更新供应商
    pub fn update_provider(&self, id: &str, updated: Provider) -> Result<Provider> {
        let mut store = self.load_store()?;

        let provider = store
            .providers
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| anyhow!("供应商不存在: {}", id))?;

        provider.name = updated.name;
        provider.website_url = updated.website_url;
        provider.user_id = updated.user_id;
        provider.access_token = updated.access_token;
        provider.username = updated.username;
        provider.updated_at = chrono::Utc::now().timestamp();

        let updated_at = provider.updated_at;
        let result = provider.clone();

        store.updated_at = updated_at;
        self.save_store(&store)?;

        Ok(result)
    }

    /// 删除供应商
    pub fn delete_provider(&self, id: &str) -> Result<()> {
        let mut store = self.load_store()?;

        // 不允许删除默认供应商
        if store.providers.iter().any(|p| p.id == id && p.is_default) {
            return Err(anyhow!("无法删除默认供应商"));
        }

        store.providers.retain(|p| p.id != id);
        store.updated_at = chrono::Utc::now().timestamp();
        self.save_store(&store)?;

        Ok(())
    }

    /// 清除缓存（用于测试或强制刷新）
    pub fn clear_cache(&self) {
        *self.cache.lock().unwrap() = None;
    }
}

impl Default for ProviderManager {
    fn default() -> Self {
        Self::new().expect("Failed to create ProviderManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_manager_creation() {
        let manager = ProviderManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_load_default_store() {
        let manager = ProviderManager::new().unwrap();
        let store = manager.load_store().unwrap();
        assert_eq!(store.version, 1);
        assert_eq!(store.providers.len(), 1);
        assert_eq!(store.providers[0].id, "duckcoding");
    }
}
