// Dashboard Manager Service
//
// 仪表板状态管理服务

use crate::data::DataManager;
use crate::models::dashboard::DashboardStore;
use crate::utils::config::config_dir;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// 仪表板状态管理器
pub struct DashboardManager {
    data_manager: Arc<DataManager>,
    store_path: PathBuf,
    cache: Arc<Mutex<Option<DashboardStore>>>,
}

impl DashboardManager {
    /// 创建新的 DashboardManager 实例
    pub fn new() -> Result<Self> {
        let data_manager = Arc::new(DataManager::new());
        let store_path = config_dir()
            .map_err(|e| anyhow::anyhow!("获取配置目录失败: {}", e))?
            .join("dashboard.json");

        Ok(Self {
            data_manager,
            store_path,
            cache: Arc::new(Mutex::new(None)),
        })
    }

    /// 读取存储（带缓存）
    pub fn load_store(&self) -> Result<DashboardStore> {
        // 检查缓存
        if let Some(cached) = self.cache.lock().unwrap().as_ref() {
            return Ok(cached.clone());
        }

        // 文件不存在则返回默认值
        if !self.store_path.exists() {
            tracing::warn!("dashboard.json 不存在，返回默认配置");
            let default_store = DashboardStore::default();
            // 初次创建时保存默认配置
            let _ = self.save_store(&default_store);
            return Ok(default_store);
        }

        // 从文件读取
        let json_value = self.data_manager.json().read(&self.store_path)?;
        let store: DashboardStore = serde_json::from_value(json_value)
            .map_err(|e| anyhow::anyhow!("反序列化 DashboardStore 失败: {}", e))?;

        // 更新缓存
        *self.cache.lock().unwrap() = Some(store.clone());

        Ok(store)
    }

    /// 保存存储
    fn save_store(&self, store: &DashboardStore) -> Result<()> {
        let json_value = serde_json::to_value(store)
            .map_err(|e| anyhow::anyhow!("序列化 DashboardStore 失败: {}", e))?;
        self.data_manager
            .json()
            .write(&self.store_path, &json_value)?;
        *self.cache.lock().unwrap() = Some(store.clone());
        Ok(())
    }

    /// 获取工具实例选择
    pub fn get_tool_instance_selection(&self, tool_id: &str) -> Result<Option<String>> {
        Ok(self
            .load_store()?
            .tool_instance_selections
            .get(tool_id)
            .cloned())
    }

    /// 设置工具实例选择
    pub fn set_tool_instance_selection(&self, tool_id: String, instance_id: String) -> Result<()> {
        let mut store = self.load_store()?;

        store.tool_instance_selections.insert(tool_id, instance_id);
        store.updated_at = chrono::Utc::now().timestamp();

        self.save_store(&store)?;
        Ok(())
    }

    /// 获取最后选中的供应商 ID
    pub fn get_selected_provider_id(&self) -> Result<Option<String>> {
        Ok(self.load_store()?.selected_provider_id)
    }

    /// 设置最后选中的供应商 ID
    pub fn set_selected_provider_id(&self, provider_id: Option<String>) -> Result<()> {
        let mut store = self.load_store()?;

        store.selected_provider_id = provider_id;
        store.updated_at = chrono::Utc::now().timestamp();

        self.save_store(&store)?;
        Ok(())
    }

    /// 清除缓存（用于测试或强制刷新）
    pub fn clear_cache(&self) {
        *self.cache.lock().unwrap() = None;
    }
}

impl Default for DashboardManager {
    fn default() -> Self {
        Self::new().expect("Failed to create DashboardManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_manager_creation() {
        let manager = DashboardManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_load_default_store() {
        let manager = DashboardManager::new().unwrap();
        let store = manager.load_store().unwrap();
        assert_eq!(store.version, 1);
        assert!(store.tool_instance_selections.is_empty());
        assert!(store.selected_provider_id.is_none());
    }

    #[test]
    fn test_tool_instance_selection() {
        let manager = DashboardManager::new().unwrap();
        manager.clear_cache(); // 清除可能的旧数据

        // 设置选择
        manager
            .set_tool_instance_selection("claude-code".to_string(), "claude-code-local".to_string())
            .unwrap();

        // 读取验证
        let selection = manager.get_tool_instance_selection("claude-code").unwrap();
        assert_eq!(selection, Some("claude-code-local".to_string()));

        // 读取不存在的工具
        let none_selection = manager.get_tool_instance_selection("unknown").unwrap();
        assert_eq!(none_selection, None);
    }

    #[test]
    fn test_selected_provider_id() {
        let manager = DashboardManager::new().unwrap();
        manager.clear_cache();

        // 默认为 None
        let default_id = manager.get_selected_provider_id().unwrap();
        assert_eq!(default_id, None);

        // 设置供应商 ID
        manager
            .set_selected_provider_id(Some("duckcoding".to_string()))
            .unwrap();

        // 读取验证
        let selected_id = manager.get_selected_provider_id().unwrap();
        assert_eq!(selected_id, Some("duckcoding".to_string()));

        // 清除供应商 ID
        manager.set_selected_provider_id(None).unwrap();
        let cleared_id = manager.get_selected_provider_id().unwrap();
        assert_eq!(cleared_id, None);
    }
}
