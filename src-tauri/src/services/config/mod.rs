//! 配置服务模块
//!
//! 提供统一的工具配置管理接口，支持 Claude Code、Codex、Gemini CLI 三个工具。
//!
//! ## 模块结构
//!
//! - `types`: 共享类型定义
//! - `utils`: 工具函数（TOML 合并等）
//! - `claude`: Claude Code 配置管理
//! - `codex`: Codex 配置管理
//! - `gemini`: Gemini CLI 配置管理
//! - `watcher`: 外部变更检测与文件监听

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// 模块声明
pub mod claude;
pub mod codex;
pub mod gemini;
pub mod types;
pub mod utils;

// 重导出类型
pub use types::*;

/// 统一的工具配置管理接口
///
/// 所有工具配置管理器都应该实现此 trait，以提供一致的 API。
///
/// # 类型参数
///
/// - `Settings`: 配置的结构类型
/// - `Payload`: 保存配置时的载荷类型
///
/// # 示例
///
/// ```ignore
/// impl ToolConfigManager for ClaudeConfigManager {
///     type Settings = Value;
///     type Payload = ClaudeSettingsPayload;
///
///     fn read_settings() -> Result<Self::Settings> {
///         // 读取配置实现
///     }
///
///     fn save_settings(payload: &Self::Payload) -> Result<()> {
///         // 保存配置实现
///     }
///
///     fn get_schema() -> Result<Value> {
///         // 获取 Schema 实现
///     }
/// }
/// ```
pub trait ToolConfigManager {
    /// 配置的结构类型
    type Settings: Serialize + for<'de> Deserialize<'de>;

    /// 保存配置时的载荷类型
    type Payload: Serialize + for<'de> Deserialize<'de>;

    /// 读取工具配置
    ///
    /// # Errors
    ///
    /// 当配置文件不存在、格式错误或读取失败时返回错误
    fn read_settings() -> Result<Self::Settings>;

    /// 保存工具配置
    ///
    /// # Arguments
    ///
    /// * `payload` - 配置载荷
    ///
    /// # Errors
    ///
    /// 当写入文件失败或配置无效时返回错误
    fn save_settings(payload: &Self::Payload) -> Result<()>;

    /// 获取配置 Schema
    ///
    /// # Errors
    ///
    /// 当 Schema 文件不存在或读取失败时返回错误
    fn get_schema() -> Result<Value>;
}
