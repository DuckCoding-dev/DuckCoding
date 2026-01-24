// Pricing 默认模板配置迁移
//
// 将 codex 的默认模板从 builtin_claude 迁移到 builtin_openai

use crate::data::DataManager;
use crate::services::migration_manager::migration_trait::{Migration, MigrationResult};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::PathBuf;

/// Pricing 默认模板配置迁移（目标版本 1.5.5）
pub struct PricingDefaultTemplatesMigration;

impl Default for PricingDefaultTemplatesMigration {
    fn default() -> Self {
        Self::new()
    }
}

impl PricingDefaultTemplatesMigration {
    pub fn new() -> Self {
        Self
    }

    /// 获取 default_templates.json 路径
    fn get_default_templates_path() -> Result<PathBuf> {
        let home_dir = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("无法获取用户主目录"))?;
        Ok(home_dir
            .join(".duckcoding")
            .join("pricing")
            .join("default_templates.json"))
    }
}

#[async_trait]
impl Migration for PricingDefaultTemplatesMigration {
    fn id(&self) -> &str {
        "pricing_default_templates_v2"
    }

    fn name(&self) -> &str {
        "Pricing 默认模板配置迁移"
    }

    fn target_version(&self) -> &str {
        "1.5.5"
    }

    async fn execute(&self) -> Result<MigrationResult> {
        tracing::info!("开始执行 Pricing 默认模板配置迁移");

        let config_path = Self::get_default_templates_path()?;

        // 如果配置文件不存在，无需迁移
        if !config_path.exists() {
            return Ok(MigrationResult {
                migration_id: self.id().to_string(),
                success: true,
                message: "配置文件不存在，跳过迁移".to_string(),
                records_migrated: 0,
                duration_secs: 0.0,
            });
        }

        let manager = DataManager::new();
        let mut config_value = manager
            .json_uncached()
            .read(&config_path)
            .context("Failed to read default_templates.json")?;

        let mut migrated = false;

        // 使用 serde_json::Value 手动处理
        if let Some(config_obj) = config_value.as_object_mut() {
            // 检查配置版本号
            let current_version = config_obj
                .get("version")
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as u32;

            // v1 -> v2 迁移：将 codex 的默认模板从 builtin_claude 改为 builtin_openai
            if current_version < 2 {
                if let Some(codex_template) = config_obj.get("codex") {
                    if codex_template.as_str() == Some("builtin_claude") {
                        tracing::info!("迁移 codex 默认模板: builtin_claude -> builtin_openai");
                        config_obj.insert(
                            "codex".to_string(),
                            serde_json::Value::String("builtin_openai".to_string()),
                        );
                        migrated = true;
                    }
                }

                // 更新版本号
                config_obj.insert("version".to_string(), serde_json::Value::Number(2.into()));
                migrated = true;
            }
        }

        if migrated {
            // 保存迁移后的配置
            manager
                .json_uncached()
                .write(&config_path, &config_value)
                .context("Failed to write migrated default_templates.json")?;

            Ok(MigrationResult {
                migration_id: self.id().to_string(),
                success: true,
                message: "成功迁移 codex 默认模板配置".to_string(),
                records_migrated: 1,
                duration_secs: 0.0,
            })
        } else {
            Ok(MigrationResult {
                migration_id: self.id().to_string(),
                success: true,
                message: "配置已是最新版本，无需迁移".to_string(),
                records_migrated: 0,
                duration_secs: 0.0,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_migrate_codex_template() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("default_templates.json");

        // 创建 v1 配置（codex 使用 builtin_claude）
        let old_config = json!({
            "claude-code": "builtin_claude",
            "codex": "builtin_claude",
            "gemini-cli": "builtin_claude"
        });

        fs::write(
            &config_path,
            serde_json::to_string_pretty(&old_config).unwrap(),
        )
        .unwrap();

        // 执行迁移
        let manager = DataManager::new();
        let mut config_value = manager.json_uncached().read(&config_path).unwrap();

        if let Some(config_obj) = config_value.as_object_mut() {
            config_obj.insert(
                "codex".to_string(),
                serde_json::Value::String("builtin_openai".to_string()),
            );
            config_obj.insert("version".to_string(), serde_json::Value::Number(2.into()));
        }

        manager
            .json_uncached()
            .write(&config_path, &config_value)
            .unwrap();

        // 验证结果
        let migrated_value = manager.json_uncached().read(&config_path).unwrap();
        assert_eq!(migrated_value["codex"], "builtin_openai");
        assert_eq!(migrated_value["version"], 2);
        assert_eq!(migrated_value["claude-code"], "builtin_claude");
    }
}
