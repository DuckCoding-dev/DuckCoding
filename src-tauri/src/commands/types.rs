// 命令层数据类型定义

// 重新导出 ToolStatus（定义在 models 层）
pub use duckcoding::models::ToolStatus;

/// 工具扫描结果（用于添加实例预览，包含路径信息）
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ToolScanResult {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
    pub install_path: Option<String>,
}

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

/// 更新结果
#[derive(serde::Serialize, serde::Deserialize)]
pub struct UpdateResult {
    pub success: bool,
    pub message: String,
    pub has_update: bool,
    pub current_version: Option<String>,
    pub latest_version: Option<String>,
    pub mirror_version: Option<String>, // 镜像实际可安装的版本
    pub mirror_is_stale: Option<bool>,  // 镜像是否滞后
    pub tool_id: Option<String>,        // 工具ID，用于批量检查时识别工具
}
