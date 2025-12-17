// Proxy 配置重构迁移
//
// 将旧的 transparent_proxy_* 字段迁移到 proxy_configs["claude-code"]

use crate::data::DataManager;
use crate::services::migration_manager::migration_trait::{Migration, MigrationResult};
use crate::utils::config::global_config_path;
use anyhow::Result;
use async_trait::async_trait;

/// Proxy 配置重构迁移（目标版本 1.3.9）
pub struct ProxyConfigMigration;

impl Default for ProxyConfigMigration {
    fn default() -> Self {
        Self::new()
    }
}

impl ProxyConfigMigration {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Migration for ProxyConfigMigration {
    fn id(&self) -> &str {
        "proxy_config_refactor_v1"
    }

    fn name(&self) -> &str {
        "Proxy 配置重构迁移"
    }

    fn target_version(&self) -> &str {
        "1.4.0"
    }

    async fn execute(&self) -> Result<MigrationResult> {
        tracing::info!("开始执行 Proxy 配置重构迁移");

        // 读取配置
        let config_path = global_config_path().map_err(|e| anyhow::anyhow!(e))?;
        let manager = DataManager::new();

        let mut config_value = manager.json_uncached().read(&config_path)?;

        let mut migrated = false;

        // 使用 serde_json::Value 手动处理，避免结构体字段不匹配
        if let Some(config_obj) = config_value.as_object_mut() {
            // 检查是否需要迁移（检查旧字段是否存在）
            let has_old_fields = config_obj.get("transparent_proxy_enabled").is_some()
                || config_obj.get("transparent_proxy_api_key").is_some()
                || config_obj.get("transparent_proxy_real_api_key").is_some();

            if has_old_fields {
                // 读取旧字段值
                let old_enabled = config_obj
                    .get("transparent_proxy_enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let old_port = config_obj
                    .get("transparent_proxy_port")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(8787) as u16;
                let old_local_key = config_obj
                    .get("transparent_proxy_api_key")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let old_real_key = config_obj
                    .get("transparent_proxy_real_api_key")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let old_real_url = config_obj
                    .get("transparent_proxy_real_base_url")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let old_allow_public = config_obj
                    .get("transparent_proxy_allow_public")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // 获取或创建 proxy_configs
                let proxy_configs = config_obj
                    .entry("proxy_configs".to_string())
                    .or_insert_with(|| serde_json::json!({}));

                if let Some(proxy_configs_obj) = proxy_configs.as_object_mut() {
                    // 获取或创建 claude-code 配置
                    let claude_config = proxy_configs_obj
                        .entry("claude-code".to_string())
                        .or_insert_with(|| {
                            serde_json::json!({
                                "enabled": false,
                                "port": 8787,
                                "local_api_key": null,
                                "real_api_key": null,
                                "real_base_url": null,
                                "real_model_provider": null,
                                "real_profile_name": null,
                                "allow_public": false,
                                "session_endpoint_config_enabled": false,
                                "auto_start": false,
                                "original_active_profile": null
                            })
                        });

                    // 只有当新配置还是默认值时才迁移
                    if let Some(claude_obj) = claude_config.as_object_mut() {
                        let is_default = !claude_obj
                            .get("enabled")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false)
                            && claude_obj
                                .get("real_api_key")
                                .and_then(|v| v.as_str())
                                .is_none();

                        if is_default {
                            claude_obj
                                .insert("enabled".to_string(), serde_json::json!(old_enabled));
                            claude_obj.insert("port".to_string(), serde_json::json!(old_port));
                            if let Some(key) = old_local_key {
                                claude_obj
                                    .insert("local_api_key".to_string(), serde_json::json!(key));
                            }
                            if let Some(key) = old_real_key {
                                claude_obj
                                    .insert("real_api_key".to_string(), serde_json::json!(key));
                            }
                            if let Some(url) = old_real_url {
                                claude_obj
                                    .insert("real_base_url".to_string(), serde_json::json!(url));
                            }
                            claude_obj.insert(
                                "allow_public".to_string(),
                                serde_json::json!(old_allow_public),
                            );

                            migrated = true;
                        }
                    }
                }

                // 删除旧字段
                config_obj.remove("transparent_proxy_enabled");
                config_obj.remove("transparent_proxy_port");
                config_obj.remove("transparent_proxy_api_key");
                config_obj.remove("transparent_proxy_real_api_key");
                config_obj.remove("transparent_proxy_real_base_url");
                config_obj.remove("transparent_proxy_allow_public");

                // 保存配置
                manager.json_uncached().write(&config_path, &config_value)?;
            }
        }

        let message = if migrated {
            "成功迁移 Proxy 配置到新架构"
        } else {
            "无需迁移（已迁移或无旧配置）"
        };

        Ok(MigrationResult {
            migration_id: self.id().to_string(),
            success: true,
            message: message.to_string(),
            records_migrated: if migrated { 1 } else { 0 },
            duration_secs: 0.0,
        })
    }
}
