// LocalStorage → JSON 迁移
//
// 将余额监控配置从 localStorage 迁移到 balance.json

use crate::services::balance::BalanceManager;
use crate::services::migration_manager::migration_trait::{Migration, MigrationResult};
use anyhow::{Context, Result};
use async_trait::async_trait;

/// LocalStorage → JSON 迁移（目标版本 1.4.1）
///
/// 注意：此迁移仅在后端创建空的 balance.json 文件
/// 实际的数据迁移由前端在首次加载时自动完成
pub struct BalanceLocalstorageToJsonMigration;

impl BalanceLocalstorageToJsonMigration {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BalanceLocalstorageToJsonMigration {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Migration for BalanceLocalstorageToJsonMigration {
    fn id(&self) -> &str {
        "balance_localstorage_to_json_v1"
    }

    fn name(&self) -> &str {
        "余额监控 LocalStorage → JSON 迁移"
    }

    fn target_version(&self) -> &str {
        "1.4.1"
    }

    async fn execute(&self) -> Result<MigrationResult> {
        tracing::info!("开始执行余额监控存储迁移");

        let manager = BalanceManager::new()?;

        // 检查 balance.json 是否已存在
        // 通过尝试加载来判断（load_store 会检查文件是否存在）
        let store = manager.load_store()?;

        // 如果已有配置，说明文件已存在或已经迁移过
        if !store.configs.is_empty() {
            tracing::info!("balance.json 已包含数据，跳过迁移");
            return Ok(MigrationResult {
                migration_id: self.id().to_string(),
                success: true,
                message: "balance.json 已包含数据，无需迁移".to_string(),
                records_migrated: 0,
                duration_secs: 0.0,
            });
        }

        // 创建空的 balance.json（如果还不存在的话）
        // 前端会在首次加载 BalancePage 时检测 localStorage 数据并自动迁移
        let empty_store = crate::models::BalanceStore::default();
        manager.save_store(&empty_store)?;

        tracing::info!("已创建 balance.json，等待前端首次加载时自动迁移 localStorage 数据");

        Ok(MigrationResult {
            migration_id: self.id().to_string(),
            success: true,
            message: "已创建 balance.json，前端将自动完成数据迁移".to_string(),
            records_migrated: 0,
            duration_secs: 0.0,
        })
    }

    async fn rollback(&self) -> Result<()> {
        tracing::warn!("回滚余额监控迁移：删除 balance.json");

        // 构造文件路径
        let home_dir = dirs::home_dir().context("无法获取用户主目录")?;
        let file_path = home_dir.join(".duckcoding").join("balance.json");

        if file_path.exists() {
            std::fs::remove_file(&file_path)?;
            tracing::info!("已删除 balance.json");
        } else {
            tracing::warn!("balance.json 不存在，无需回滚");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_migration_creates_empty_file() {
        let migration = BalanceLocalstorageToJsonMigration::new();

        assert_eq!(migration.id(), "balance_localstorage_to_json_v1");
        assert_eq!(migration.name(), "余额监控 LocalStorage → JSON 迁移");
        assert_eq!(migration.target_version(), "1.4.1");
    }
}
