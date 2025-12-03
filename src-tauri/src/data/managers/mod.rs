//! 数据管理器实现
//!
//! 提供各种格式的配置文件管理器：
//! - `json`: JSON 管理器（支持缓存和无缓存两种模式）
//! - `toml`: TOML 管理器（保留注释和格式）
//! - `env`: ENV 文件管理器（保留注释）
//! - `sqlite`: SQLite 数据库管理器（支持缓存和事务）

pub mod env;
pub mod json;
pub mod sqlite;
pub mod toml;

pub use env::EnvManager;
pub use json::JsonManager;
pub use sqlite::SqliteManager;
pub use toml::TomlManager;
