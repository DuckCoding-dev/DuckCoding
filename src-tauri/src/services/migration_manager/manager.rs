// Migration Manager - 迁移管理器核心
//
// 统一管理所有数据迁移操作

use super::migration_trait::{compare_versions, Migration, MigrationResult};
use crate::models::GlobalConfig;
use crate::utils::config::{read_global_config, write_global_config};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::sync::Arc;

/// 当前应用版本（从 Cargo.toml 读取）
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 迁移管理器
pub struct MigrationManager {
    migrations: Vec<Arc<dyn Migration>>,
}

impl MigrationManager {
    /// 创建新的迁移管理器
    pub fn new() -> Self {
        Self {
            migrations: Vec::new(),
        }
    }

    /// 注册迁移
    pub fn register(&mut self, migration: Arc<dyn Migration>) {
        tracing::debug!(
            "注册迁移: {} (目标版本: {})",
            migration.id(),
            migration.target_version()
        );
        self.migrations.push(migration);
    }

    /// 执行所有需要的迁移
    ///
    /// 流程：
    /// 1. 读取 GlobalConfig.version（当前版本）
    /// 2. 筛选需要执行的迁移（target_version > current_version）
    /// 3. 按 target_version 排序（从低到高）
    /// 4. 依次执行迁移
    /// 5. 每个迁移成功后更新 config.version = target_version
    /// 6. **最后强制更新 config.version = APP_VERSION**（解决无新迁移时版本不更新问题）
    pub async fn run_all(&self) -> Result<Vec<MigrationResult>> {
        tracing::info!("开始执行迁移检查（应用版本: {}）", APP_VERSION);

        // 1. 读取当前配置版本
        let current_version = self.get_current_version()?;
        tracing::info!("当前配置版本: {}", current_version);

        // 2. 筛选需要执行的迁移
        let mut pending_migrations: Vec<_> = self
            .migrations
            .iter()
            .filter(|m| {
                let needs =
                    compare_versions(&current_version, m.target_version()) == Ordering::Less;
                if needs {
                    tracing::info!(
                        "需要执行迁移: {} ({} → {})",
                        m.name(),
                        current_version,
                        m.target_version()
                    );
                }
                needs
            })
            .collect();

        if pending_migrations.is_empty() {
            tracing::info!("无需执行迁移");
        } else {
            // 3. 按 target_version 排序（从低到高）
            pending_migrations
                .sort_by(|a, b| compare_versions(a.target_version(), b.target_version()));

            tracing::info!("共 {} 个迁移需要执行", pending_migrations.len());
        }

        // 4. 依次执行迁移
        let mut results = Vec::new();

        for migration in pending_migrations {
            tracing::info!(
                "执行迁移: {} (目标版本: {})",
                migration.name(),
                migration.target_version()
            );

            let start_time = std::time::Instant::now();
            let result = migration.execute().await;

            match result {
                Ok(mut migration_result) => {
                    migration_result.duration_secs = start_time.elapsed().as_secs_f64();

                    tracing::info!(
                        "迁移 {} 成功: {}（耗时 {:.2}s）",
                        migration.name(),
                        migration_result.message,
                        migration_result.duration_secs
                    );

                    // 5. 更新配置版本到迁移目标版本
                    if let Err(e) = self.update_config_version(migration.target_version()) {
                        tracing::error!("更新配置版本失败: {}", e);
                        // 不中断后续迁移
                    }

                    results.push(migration_result);
                }
                Err(e) => {
                    let error_result = MigrationResult {
                        migration_id: migration.id().to_string(),
                        success: false,
                        message: format!("迁移失败: {}", e),
                        records_migrated: 0,
                        duration_secs: start_time.elapsed().as_secs_f64(),
                    };

                    tracing::error!(
                        "迁移 {} 失败: {}（耗时 {:.2}s）",
                        migration.name(),
                        e,
                        error_result.duration_secs
                    );

                    results.push(error_result);

                    // 6. 迁移失败，继续执行后续迁移（不中断）
                    tracing::warn!("迁移失败，继续执行后续迁移");
                }
            }
        }

        // 7. 强制更新配置版本为当前应用版本
        //    解决问题：如果新版本没有新迁移，config.version 仍会更新
        if compare_versions(&current_version, APP_VERSION) == Ordering::Less {
            tracing::info!("更新配置版本: {} → {}", current_version, APP_VERSION);
            if let Err(e) = self.update_config_version(APP_VERSION) {
                tracing::error!("更新配置版本失败: {}", e);
            }
        }

        if !results.is_empty() {
            tracing::info!(
                "所有迁移执行完成，成功 {} 个，失败 {} 个",
                results.iter().filter(|r| r.success).count(),
                results.iter().filter(|r| !r.success).count()
            );
        }

        Ok(results)
    }

    /// 获取当前配置版本
    fn get_current_version(&self) -> Result<String> {
        match read_global_config().map_err(|e| anyhow::anyhow!(e))? {
            Some(config) => Ok(config.version.unwrap_or_else(|| "0.0.0".to_string())),
            None => Ok("0.0.0".to_string()), // 无配置文件，视为初始版本
        }
    }

    /// 更新配置版本
    fn update_config_version(&self, new_version: &str) -> Result<()> {
        // 读取现有配置，如果不存在则创建新配置
        let mut config = read_global_config()
            .map_err(|e| anyhow::anyhow!(e))?
            .unwrap_or_else(|| GlobalConfig {
                version: Some("0.0.0".to_string()),
                user_id: Some(String::new()),
                system_token: Some(String::new()),
                proxy_enabled: false,
                proxy_type: None,
                proxy_host: None,
                proxy_port: None,
                proxy_username: None,
                proxy_password: None,
                proxy_bypass_urls: vec![],
                proxy_configs: std::collections::HashMap::new(),
                session_endpoint_config_enabled: false,
                hide_transparent_proxy_tip: false,
                hide_session_config_hint: false,
                log_config: crate::models::LogConfig::default(),
                onboarding_status: None,
                external_watch_enabled: true,
                external_poll_interval_ms: 5000,
                single_instance_enabled: true,
                startup_enabled: false,
            });

        config.version = Some(new_version.to_string());
        write_global_config(&config).map_err(|e| anyhow::anyhow!(e))?;

        tracing::info!("配置版本已更新: {}", new_version);
        Ok(())
    }

    /// 执行单个迁移（用于测试或手动触发）
    pub async fn run_single(&self, migration_id: &str) -> Result<MigrationResult> {
        let migration = self
            .migrations
            .iter()
            .find(|m| m.id() == migration_id)
            .ok_or_else(|| anyhow::anyhow!("未找到迁移: {}", migration_id))?;

        tracing::info!("手动执行迁移: {}", migration.name());
        migration.execute().await
    }

    /// 获取所有已注册的迁移
    pub fn list_migrations(&self) -> Vec<MigrationInfo> {
        self.migrations
            .iter()
            .map(|m| MigrationInfo {
                id: m.id().to_string(),
                name: m.name().to_string(),
                target_version: m.target_version().to_string(),
            })
            .collect()
    }
}

impl Default for MigrationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 迁移信息（用于列表展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationInfo {
    pub id: String,
    pub name: String,
    pub target_version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock 迁移用于测试
    struct MockMigration {
        id: String,
        target_version: String,
        should_fail: bool,
    }

    #[async_trait::async_trait]
    impl Migration for MockMigration {
        fn id(&self) -> &str {
            &self.id
        }

        fn name(&self) -> &str {
            &self.id
        }

        fn target_version(&self) -> &str {
            &self.target_version
        }

        async fn execute(&self) -> Result<MigrationResult> {
            if self.should_fail {
                anyhow::bail!("模拟失败");
            }

            Ok(MigrationResult {
                migration_id: self.id.clone(),
                success: true,
                message: "成功".to_string(),
                records_migrated: 10,
                duration_secs: 0.1,
            })
        }
    }

    #[tokio::test]
    async fn test_migration_sorting() {
        let mut manager = MigrationManager::new();

        // 注册乱序的迁移
        manager.register(Arc::new(MockMigration {
            id: "migration3".to_string(),
            target_version: "1.4.0".to_string(),
            should_fail: false,
        }));
        manager.register(Arc::new(MockMigration {
            id: "migration1".to_string(),
            target_version: "1.3.9".to_string(),
            should_fail: false,
        }));
        manager.register(Arc::new(MockMigration {
            id: "migration2".to_string(),
            target_version: "1.3.10".to_string(),
            should_fail: false,
        }));

        // 迁移应该按版本号排序执行
        // 实际执行需要配置环境，这里只测试注册
        assert_eq!(manager.migrations.len(), 3);
    }
}
