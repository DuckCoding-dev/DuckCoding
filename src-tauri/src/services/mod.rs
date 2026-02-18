// 服务层模块
//
// 重组后的目录结构：
// - config: 配置管理（待拆分优化）
// - tool: 工具安装、版本检查、下载
// - proxy: 代理配置和透明代理
// - update: 应用自身更新
// - session: 会话管理（透明代理请求追踪）
// - migration_manager: 统一迁移管理（新）
// - balance: 余额监控配置管理
// - provider_manager: 供应商配置管理
// - new_api: NEW API 客户端服务
// - token_stats: Token统计和请求记录
// - checkin: 签到服务

pub mod amp_native_config; // AMP Code 原生配置管理
pub mod balance;
pub mod checkin; // 签到服务
pub mod checkin_scheduler; // 签到调度器
pub mod config;
pub mod dashboard_manager; // 仪表板状态管理
pub mod migration_manager;
pub mod new_api; // NEW API 客户端
pub mod pricing; // 价格配置管理
pub mod profile_manager; // Profile管理（v2.1）
pub mod provider_manager; // 供应商配置管理
pub mod proxy;
pub mod proxy_config_manager; // 透明代理配置管理（v2.1）
pub mod session;
pub mod token_stats; // Token统计服务
pub mod tool;
pub mod update;

// 重新导出服务
pub use balance::*;
pub use checkin::*;
pub use checkin_scheduler::CheckinScheduler;
pub use config::types::*; // 仅导出类型
pub use dashboard_manager::DashboardManager;
pub use migration_manager::{create_migration_manager, MigrationManager};
pub use new_api::NewApiClient;
pub use profile_manager::{
    ActiveStore, ClaudeProfile, CodexProfile, GeminiProfile, ProfileDescriptor, ProfileManager,
    ProfileSource, ProfilesStore,
}; // Profile管理（v2.0）
pub use provider_manager::ProviderManager;
pub use proxy::*;
// session 模块：明确导出避免 db 名称冲突
pub use session::{manager::SESSION_MANAGER, models::*};
// token_stats 模块：导出管理器和提取器
pub use token_stats::{TokenStatsDb, TokenStatsManager};
// tool 模块：导出主要服务类和子模块
pub use tool::{
    db::ToolInstanceDB, downloader, downloader::FileDownloader, installer,
    installer::InstallerService, registry::ToolRegistry, version, version::VersionService,
};
pub use update::*;
