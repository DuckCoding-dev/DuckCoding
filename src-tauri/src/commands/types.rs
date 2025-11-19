// 命令层数据类型定义

/// 工具状态
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ToolStatus {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
    #[serde(default)]
    pub has_update: bool,
    #[serde(default)]
    pub latest_version: Option<String>,
    #[serde(default)]
    pub mirror_version: Option<String>,
    #[serde(default)]
    pub mirror_is_stale: bool,
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
    pub mirror_version: Option<String>,  // 镜像实际可安装的版本
    pub mirror_is_stale: Option<bool>,   // 镜像是否滞后
    pub tool_id: Option<String>,         // 工具ID，用于批量检查时识别工具
}

/// 活动配置
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ActiveConfig {
    pub api_key: String,
    pub base_url: String,
    pub profile_name: Option<String>,  // 当前配置的名称
}
