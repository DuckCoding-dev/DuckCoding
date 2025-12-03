//! 统一数据管理模块
//!
//! 提供 JSON/TOML/ENV/SQLite 配置文件的统一管理接口，支持缓存、校验和事务。
//!
//! # 模块组织
//!
//! - `error`: 统一错误类型定义
//! - `cache`: 缓存层实现（LRU + 文件校验和 + SQL 查询缓存）
//! - `managers`: 各格式管理器（JSON/TOML/ENV/SQLite）
//! - `manager`: 统一入口 `DataManager`
//!
//! # 使用示例
//!
//! ```rust
//! use crate::data::DataManager;
//! use std::path::Path;
//!
//! // 创建统一管理器
//! let manager = DataManager::new();
//!
//! // 读取全局配置（带缓存）
//! let config = manager.json().read(Path::new("config.json"))?;
//!
//! // 读取工具原生配置（无缓存）
//! let settings = manager.json_uncached().read(Path::new("~/.claude/settings.json"))?;
//! ```

pub mod cache;
pub mod error;
pub mod manager;
pub mod managers;

pub use error::{DataError, Result};
pub use manager::{CacheConfig, DataManager};
