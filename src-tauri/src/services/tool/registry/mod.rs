//! Tool Registry Module
//!
//! 工具注册表模块，按职责拆分为多个子模块

mod detection;
mod instance;
mod query;
mod version_ops;

use crate::services::tool::{DetectorRegistry, ToolInstanceDB};
use crate::utils::{CommandExecutor, WSLExecutor};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 工具检测进度（用于前端显示）
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolDetectionProgress {
    pub tool_id: String,
    pub tool_name: String,
    pub status: String, // "pending", "detecting", "done"
    pub installed: Option<bool>,
    pub version: Option<String>,
}

/// 工具注册表 - 统一管理所有工具实例
pub struct ToolRegistry {
    pub(super) db: Arc<Mutex<ToolInstanceDB>>,
    pub(super) detector_registry: DetectorRegistry,
    pub(super) command_executor: CommandExecutor,
    pub(super) wsl_executor: WSLExecutor,
}

impl ToolRegistry {
    /// 创建新的工具注册表
    pub async fn new() -> Result<Self> {
        let db = ToolInstanceDB::new()?;

        // 初始化配置文件（如果不存在）
        // 注意：迁移逻辑已移到 MigrationManager，这里仅初始化
        db.init_tables()?;

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            detector_registry: DetectorRegistry::new(),
            command_executor: CommandExecutor::new(),
            wsl_executor: WSLExecutor::new(),
        })
    }

    /// 检查数据库中是否已有本地工具数据
    pub async fn has_local_tools_in_db(&self) -> Result<bool> {
        let db = self.db.lock().await;
        db.has_local_tools()
    }
}
