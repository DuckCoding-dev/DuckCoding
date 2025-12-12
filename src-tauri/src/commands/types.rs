// 命令层数据类型定义

// 重新导出 models 层的类型
pub use duckcoding::models::{ToolStatus, UpdateResult};

/// Node 环境信息
#[derive(serde::Serialize, serde::Deserialize)]
pub struct NodeEnvironment {
    pub node_available: bool,
    pub node_version: Option<String>,
    pub npm_available: bool,
    pub npm_version: Option<String>,
}

/// 安装结果
#[derive(serde::Serialize, serde::Deserialize)]
pub struct InstallResult {
    pub success: bool,
    pub message: String,
    pub output: String,
}
