// Migration Manager Module
//
// 统一迁移管理系统

mod manager;
mod migration_trait;
mod migrations;

pub use manager::MigrationManager;
pub use migration_trait::{Migration, MigrationResult};
pub use migrations::{
    BalanceLocalstorageToJsonMigration, GlobalConfigToProvidersMigration, ProfileV2Migration,
    ProxyConfigMigration, ProxyConfigSplitMigration, SessionConfigMigration, SqliteToJsonMigration,
};

use std::sync::Arc;

/// 创建并初始化迁移管理器
///
/// 自动注册所有迁移（按版本号执行）：
/// - SqliteToJsonMigration (1.4.0) - SQLite → JSON 迁移
/// - ProxyConfigMigration (1.4.0) - Proxy 配置重构
/// - SessionConfigMigration (1.4.0) - Session 配置拆分
/// - ProfileV2Migration (1.4.0) - Profile v2.0 双文件系统迁移
/// - ProxyConfigSplitMigration (1.4.0) - 透明代理配置拆分到 proxy.json
/// - BalanceLocalstorageToJsonMigration (1.4.1) - 余额监控 LocalStorage → JSON 迁移
/// - GlobalConfigToProvidersMigration (1.5.0) - GlobalConfig 用户信息迁移到 Providers
pub fn create_migration_manager() -> MigrationManager {
    let mut manager = MigrationManager::new();

    // 注册所有迁移（按目标版本号自动排序执行）
    manager.register(Arc::new(SqliteToJsonMigration::new()));
    manager.register(Arc::new(ProxyConfigMigration::new()));
    manager.register(Arc::new(SessionConfigMigration::new()));
    manager.register(Arc::new(ProfileV2Migration::new()));
    manager.register(Arc::new(ProxyConfigSplitMigration::new()));
    manager.register(Arc::new(BalanceLocalstorageToJsonMigration::new()));
    manager.register(Arc::new(GlobalConfigToProvidersMigration::new()));

    tracing::debug!(
        "迁移管理器初始化完成，已注册 {} 个迁移",
        manager.list_migrations().len()
    );

    manager
}
