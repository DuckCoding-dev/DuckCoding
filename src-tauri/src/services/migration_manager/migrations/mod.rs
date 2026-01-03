// Migrations - 所有迁移实现
//
// 每个迁移定义目标版本号，按版本号顺序执行

mod balance_localstorage_to_json;
mod global_to_providers;
mod profile_v2;
mod proxy_config;
mod proxy_config_split;
mod session_config;
mod sqlite_to_json;

pub use balance_localstorage_to_json::BalanceLocalstorageToJsonMigration;
pub use global_to_providers::GlobalConfigToProvidersMigration;
pub use profile_v2::ProfileV2Migration;
pub use proxy_config::ProxyConfigMigration;
pub use proxy_config_split::ProxyConfigSplitMigration;
pub use session_config::SessionConfigMigration;
pub use sqlite_to_json::SqliteToJsonMigration;
