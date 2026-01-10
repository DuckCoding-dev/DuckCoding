//! 透明代理配置拆分迁移
//!
//! 从 config.json 的 proxy_configs 迁移到独立的 proxy.json
//! 目标版本：1.4.0

use crate::data::DataManager;
use crate::models::proxy_config::{ProxyStore, ToolProxyConfig};
use crate::services::migration_manager::migration_trait::{Migration, MigrationResult};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::time::Instant;

pub struct ProxyConfigSplitMigration;

impl ProxyConfigSplitMigration {
    pub fn new() -> Self {
        Self
    }

    /// 检查是否需要迁移
    fn needs_migration(&self) -> bool {
        let Some(home_dir) = dirs::home_dir() else {
            return false;
        };

        let proxy_path = home_dir.join(".duckcoding/proxy.json");
        let config_path = home_dir.join(".duckcoding/config.json");

        // proxy.json 已存在，无需迁移
        if proxy_path.exists() {
            return false;
        }

        // config.json 存在且包含 proxy_configs，需要迁移
        if config_path.exists() {
            let manager = DataManager::new();
            if let Ok(config) = manager.json().read(&config_path) {
                if config.get("proxy_configs").is_some() {
                    return true;
                }
            }
        }

        false
    }

    /// 从 config.json 读取旧的代理配置
    fn read_old_proxy_configs(&self) -> Result<ProxyStore> {
        let Some(home_dir) = dirs::home_dir() else {
            anyhow::bail!("无法获取用户主目录");
        };

        let config_path = home_dir.join(".duckcoding/config.json");
        let manager = DataManager::new();

        let config_value = manager
            .json()
            .read(&config_path)
            .context("读取 config.json 失败")?;

        let mut proxy_store = ProxyStore::new();

        // 读取 proxy_configs
        if let Some(proxy_configs_obj) = config_value
            .get("proxy_configs")
            .and_then(|v| v.as_object())
        {
            for (tool_id, config_value) in proxy_configs_obj {
                if let Ok(old_config) = parse_old_config(config_value) {
                    proxy_store.update_config(tool_id, old_config);
                    tracing::info!("已迁移 {} 的透明代理配置", tool_id);
                }
            }
        }

        // 兼容旧的单工具字段（claude-code）
        if let Some(enabled) = config_value
            .get("transparent_proxy_enabled")
            .and_then(|v| v.as_bool())
        {
            if enabled {
                let mut claude_config = proxy_store.claude_code.clone();
                claude_config.enabled = true;

                if let Some(port) = config_value
                    .get("transparent_proxy_port")
                    .and_then(|v| v.as_u64())
                {
                    claude_config.port = port as u16;
                }
                if let Some(key) = config_value
                    .get("transparent_proxy_api_key")
                    .and_then(|v| v.as_str())
                {
                    claude_config.local_api_key = Some(key.to_string());
                }
                if let Some(key) = config_value
                    .get("transparent_proxy_real_api_key")
                    .and_then(|v| v.as_str())
                {
                    claude_config.real_api_key = Some(key.to_string());
                }
                if let Some(url) = config_value
                    .get("transparent_proxy_real_base_url")
                    .and_then(|v| v.as_str())
                {
                    claude_config.real_base_url = Some(url.to_string());
                }

                proxy_store.claude_code = claude_config;
                tracing::info!("已从旧的 transparent_proxy_* 字段迁移配置");
            }
        }

        Ok(proxy_store)
    }

    /// 保存到 proxy.json
    fn save_proxy_json(&self, store: &ProxyStore) -> Result<()> {
        let Some(home_dir) = dirs::home_dir() else {
            anyhow::bail!("无法获取用户主目录");
        };

        let proxy_path = home_dir.join(".duckcoding/proxy.json");
        let manager = DataManager::new();

        let value = serde_json::to_value(store).context("序列化 ProxyStore 失败")?;
        manager
            .json()
            .write(&proxy_path, &value)
            .map_err(|e| anyhow::anyhow!("写入 proxy.json 失败: {}", e))
    }

    /// 从 config.json 移除旧字段
    fn cleanup_old_fields(&self) -> Result<()> {
        let Some(home_dir) = dirs::home_dir() else {
            anyhow::bail!("无法获取用户主目录");
        };

        let config_path = home_dir.join(".duckcoding/config.json");
        let manager = DataManager::new();

        let mut config = manager
            .json()
            .read(&config_path)
            .context("读取 config.json 失败")?;

        let obj = config
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("config.json 不是对象"))?;

        // 移除所有透明代理相关字段
        let removed_fields = vec![
            "proxy_configs",
            "transparent_proxy_enabled",
            "transparent_proxy_port",
            "transparent_proxy_api_key",
            "transparent_proxy_real_api_key",
            "transparent_proxy_real_base_url",
            "transparent_proxy_real_model_provider",
            "transparent_proxy_real_profile_name",
        ];

        let mut removed_count = 0;
        for field in removed_fields {
            if obj.remove(field).is_some() {
                removed_count += 1;
            }
        }

        if removed_count > 0 {
            manager
                .json()
                .write(&config_path, &config)
                .map_err(|e| anyhow::anyhow!("写入 config.json 失败: {}", e))?;
            tracing::info!("已从 config.json 移除 {} 个透明代理字段", removed_count);
        }

        Ok(())
    }
}

impl Default for ProxyConfigSplitMigration {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Migration for ProxyConfigSplitMigration {
    fn id(&self) -> &str {
        "proxy_config_split"
    }

    fn name(&self) -> &str {
        "透明代理配置拆分迁移"
    }

    fn target_version(&self) -> &str {
        "1.4.0"
    }

    async fn execute(&self) -> Result<MigrationResult> {
        let start = Instant::now();

        // 检查是否需要迁移
        if !self.needs_migration() {
            return Ok(MigrationResult {
                migration_id: self.id().to_string(),
                success: true,
                message: "无需迁移（proxy.json 已存在或无旧配置）".to_string(),
                records_migrated: 0,
                duration_secs: start.elapsed().as_secs_f64(),
            });
        }

        // 读取旧配置
        let proxy_store = self.read_old_proxy_configs()?;

        // 保存到 proxy.json
        self.save_proxy_json(&proxy_store)?;

        // 清理 config.json 中的旧字段
        self.cleanup_old_fields()?;

        let duration = start.elapsed().as_secs_f64();
        tracing::info!("透明代理配置拆分迁移完成，耗时 {:.2}s", duration);

        Ok(MigrationResult {
            migration_id: self.id().to_string(),
            success: true,
            message: "成功将透明代理配置迁移到 proxy.json".to_string(),
            records_migrated: 3, // 三个工具
            duration_secs: duration,
        })
    }
}

// ==================== 辅助函数 ====================

fn parse_old_config(value: &Value) -> Result<ToolProxyConfig> {
    let obj = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("配置不是对象"))?;

    Ok(ToolProxyConfig {
        enabled: obj
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        port: obj.get("port").and_then(|v| v.as_u64()).unwrap_or(8787) as u16,
        local_api_key: obj
            .get("local_api_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        real_api_key: obj
            .get("real_api_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        real_base_url: obj
            .get("real_base_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        real_profile_name: obj
            .get("real_profile_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        allow_public: obj
            .get("allow_public")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        session_endpoint_config_enabled: obj
            .get("session_endpoint_config_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        auto_start: obj
            .get("auto_start")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        original_active_profile: obj
            .get("original_active_profile")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        original_amp_settings: obj.get("original_amp_settings").cloned(),
        original_amp_secrets: obj.get("original_amp_secrets").cloned(),
        tavily_api_key: obj
            .get("tavily_api_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    })
}
