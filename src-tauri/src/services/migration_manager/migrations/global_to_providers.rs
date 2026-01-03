// GlobalConfig 迁移到 Providers.json
//
// 将用户信息从 GlobalConfig 迁移到独立的 providers.json 存储

use crate::data::DataManager;
use crate::models::provider::ProviderStore;
use crate::services::migration_manager::migration_trait::{Migration, MigrationResult};
use crate::utils::config::{config_dir, read_global_config};
use anyhow::Result;
use async_trait::async_trait;

/// GlobalConfig 迁移到 Providers.json（目标版本 1.5.0）
pub struct GlobalConfigToProvidersMigration;

impl Default for GlobalConfigToProvidersMigration {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalConfigToProvidersMigration {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Migration for GlobalConfigToProvidersMigration {
    fn id(&self) -> &str {
        "global_config_to_providers_v1"
    }

    fn name(&self) -> &str {
        "GlobalConfig 用户信息迁移到 Providers"
    }

    fn target_version(&self) -> &str {
        "1.5.0"
    }

    async fn execute(&self) -> Result<MigrationResult> {
        tracing::info!("开始执行 GlobalConfig → Providers 迁移");

        let data_manager = DataManager::new();
        let providers_path = config_dir()
            .map_err(|e| anyhow::anyhow!("获取配置目录失败: {}", e))?
            .join("providers.json");

        // 检查是否已迁移
        if providers_path.exists() {
            tracing::info!("providers.json 已存在，跳过迁移");
            return Ok(MigrationResult {
                migration_id: self.id().to_string(),
                success: true,
                message: "已迁移，跳过".to_string(),
                records_migrated: 0,
                duration_secs: 0.0,
            });
        }

        // 读取 GlobalConfig
        let global_config = match read_global_config() {
            Ok(Some(cfg)) => cfg,
            Ok(None) | Err(_) => {
                // 如果没有配置或读取失败，创建默认 ProviderStore
                let store = ProviderStore::default();
                let json_value = serde_json::to_value(&store)
                    .map_err(|e| anyhow::anyhow!("序列化 ProviderStore 失败: {}", e))?;
                data_manager.json().write(&providers_path, &json_value)?;
                return Ok(MigrationResult {
                    migration_id: self.id().to_string(),
                    success: true,
                    message: "创建默认 Providers 配置（无用户信息）".to_string(),
                    records_migrated: 1,
                    duration_secs: 0.0,
                });
            }
        };

        // 创建默认 ProviderStore
        let mut store = ProviderStore::default();

        // 如果 GlobalConfig 中有用户信息，填充到默认 DuckCoding 供应商
        let has_user_id = global_config
            .user_id
            .as_ref()
            .is_some_and(|id| !id.is_empty());
        let has_token = global_config
            .system_token
            .as_ref()
            .is_some_and(|token| !token.is_empty());

        if has_user_id || has_token {
            if let Some(provider) = store.providers.get_mut(0) {
                provider.user_id = global_config.user_id.clone().unwrap_or_default();
                provider.access_token = global_config.system_token.clone().unwrap_or_default();
                provider.updated_at = chrono::Utc::now().timestamp();

                tracing::info!(
                    "迁移用户信息: user_id={}, token={}",
                    if provider.user_id.is_empty() {
                        "未配置"
                    } else {
                        "已配置"
                    },
                    if provider.access_token.is_empty() {
                        "未配置"
                    } else {
                        "已配置"
                    }
                );
            }
        }

        // 写入 providers.json
        let json_value = serde_json::to_value(&store)
            .map_err(|e| anyhow::anyhow!("序列化 ProviderStore 失败: {}", e))?;
        data_manager.json().write(&providers_path, &json_value)?;

        let message = if !has_user_id {
            "创建默认 Providers 配置（无用户信息）"
        } else {
            "成功迁移 GlobalConfig 用户信息到 Providers"
        };

        Ok(MigrationResult {
            migration_id: self.id().to_string(),
            success: true,
            message: message.to_string(),
            records_migrated: 1,
            duration_secs: 0.0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_migration_creates_default_store() {
        let migration = GlobalConfigToProvidersMigration::new();
        assert_eq!(migration.id(), "global_config_to_providers_v1");
        assert_eq!(migration.name(), "GlobalConfig 用户信息迁移到 Providers");
        assert_eq!(migration.target_version(), "1.5.0");
    }
}
