use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 工具状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatus {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
}

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub id: String,
    pub name: String,
    pub group_name: String,
    pub npm_package: String,
    pub check_command: String,
    pub config_dir: PathBuf,
    pub config_file: String,
    pub env_vars: EnvVars,
    /// 版本检查是否使用代理（某些工具如Claude Code在代理环境下会出错）
    pub use_proxy_for_version_check: bool,
}

/// 环境变量配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVars {
    pub api_key: String,
    pub base_url: String,
}

/// 安装方法
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InstallMethod {
    Official, // 官方脚本
    Npm,      // npm install
    Brew,     // Homebrew (macOS)
}

impl Tool {
    /// 获取所有工具
    pub fn all() -> Vec<Tool> {
        vec![Tool::claude_code(), Tool::codex(), Tool::gemini_cli()]
    }

    /// 根据 ID 获取工具
    pub fn by_id(id: &str) -> Option<Tool> {
        Self::all().into_iter().find(|t| t.id == id)
    }

    /// Claude Code 定义
    pub fn claude_code() -> Tool {
        let home_dir = dirs::home_dir().expect("无法获取用户主目录");

        Tool {
            id: "claude-code".to_string(),
            name: "Claude Code".to_string(),
            group_name: "Claude Code 专用分组".to_string(),
            npm_package: "@anthropic-ai/claude-code".to_string(),
            check_command: "claude --version".to_string(),
            config_dir: home_dir.join(".claude"),
            config_file: "settings.json".to_string(),
            env_vars: EnvVars {
                api_key: "ANTHROPIC_AUTH_TOKEN".to_string(),
                base_url: "ANTHROPIC_BASE_URL".to_string(),
            },
            use_proxy_for_version_check: false, // Claude Code在代理环境下会出现URL协议错误
        }
    }

    /// CodeX 定义
    pub fn codex() -> Tool {
        let home_dir = dirs::home_dir().expect("无法获取用户主目录");

        Tool {
            id: "codex".to_string(),
            name: "CodeX".to_string(),
            group_name: "CodeX 专用分组".to_string(),
            npm_package: "@openai/codex".to_string(),
            check_command: "codex --version".to_string(),
            config_dir: home_dir.join(".codex"),
            config_file: "config.toml".to_string(),
            env_vars: EnvVars {
                api_key: "OPENAI_API_KEY".to_string(),
                base_url: "base_url".to_string(), // TOML key
            },
            use_proxy_for_version_check: true, // CodeX可以使用代理
        }
    }

    /// Gemini CLI 定义
    pub fn gemini_cli() -> Tool {
        let home_dir = dirs::home_dir().expect("无法获取用户主目录");

        Tool {
            id: "gemini-cli".to_string(),
            name: "Gemini CLI".to_string(),
            group_name: "Gemini CLI 专用分组".to_string(),
            npm_package: "@google/gemini-cli".to_string(),
            check_command: "gemini --version".to_string(),
            config_dir: home_dir.join(".gemini"),
            config_file: "settings.json".to_string(),
            env_vars: EnvVars {
                api_key: "GEMINI_API_KEY".to_string(),
                base_url: "GOOGLE_GEMINI_BASE_URL".to_string(),
            },
            use_proxy_for_version_check: true, // Gemini CLI可以使用代理
        }
    }

    /// 获取可用的安装方法
    pub fn available_install_methods(&self) -> Vec<InstallMethod> {
        let mut methods = vec![];

        match self.id.as_str() {
            "claude-code" => {
                methods.push(InstallMethod::Official);
                methods.push(InstallMethod::Npm);
            }
            "codex" => {
                methods.push(InstallMethod::Official);
                if cfg!(target_os = "macos") {
                    methods.push(InstallMethod::Brew);
                }
                methods.push(InstallMethod::Npm);
            }
            "gemini-cli" => {
                methods.push(InstallMethod::Npm);
            }
            _ => {}
        }

        methods
    }

    /// 获取推荐的安装方法
    pub fn recommended_install_method(&self) -> InstallMethod {
        match self.id.as_str() {
            "claude-code" => InstallMethod::Official,
            "codex" => {
                // CodeX 官方安装方法尚未实现，推荐使用 npm
                // 在 macOS 上如果有 Homebrew 也可以使用
                #[cfg(target_os = "macos")]
                {
                    // macOS 优先推荐 Homebrew，但由于需要异步检测，默认使用 npm
                    InstallMethod::Npm
                }
                #[cfg(not(target_os = "macos"))]
                {
                    InstallMethod::Npm
                }
            }
            "gemini-cli" => InstallMethod::Npm,
            _ => InstallMethod::Official,
        }
    }

    /// 获取备份配置路径
    pub fn backup_path(&self, profile_name: &str) -> PathBuf {
        let ext = std::path::Path::new(&self.config_file)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let basename = std::path::Path::new(&self.config_file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("config");

        if ext.is_empty() {
            self.config_dir.join(format!("{basename}.{profile_name}"))
        } else {
            self.config_dir
                .join(format!("{basename}.{profile_name}.{ext}"))
        }
    }
}

/// Provider 配置
pub const DUCKCODING_BASE_URL: &str = "https://jp.duckcoding.com";

// ============================================================================
// 工具管理系统扩展（2025-11-29）
// ============================================================================

/// 工具环境类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ToolType {
    /// 本地环境
    Local,
    /// WSL 环境
    WSL,
    /// SSH 远程环境
    SSH,
}

impl ToolType {
    /// 转换为字符串（用于数据库存储）
    pub fn as_str(&self) -> &'static str {
        match self {
            ToolType::Local => "Local",
            ToolType::WSL => "WSL",
            ToolType::SSH => "SSH",
        }
    }

    /// 从字符串解析（避免与 std::str::FromStr 混淆）
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Local" => Some(ToolType::Local),
            "WSL" => Some(ToolType::WSL),
            "SSH" => Some(ToolType::SSH),
            _ => None,
        }
    }
}

/// 工具安装来源
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToolSource {
    /// DuckCoding 管理（安装在 ~/.duckcoding/tool/bin/）
    DuckCodingManaged,
    /// 外部安装（npm、官方脚本等）
    External,
}

impl ToolSource {
    /// 转换为字符串（用于数据库存储）
    pub fn as_str(&self) -> &'static str {
        match self {
            ToolSource::DuckCodingManaged => "DuckCodingManaged",
            ToolSource::External => "External",
        }
    }

    /// 从字符串解析（避免与 std::str::FromStr 混淆）
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "DuckCodingManaged" => Some(ToolSource::DuckCodingManaged),
            "External" => Some(ToolSource::External),
            _ => None,
        }
    }

    /// 根据安装路径判断来源
    pub fn from_install_path(path: &str) -> Self {
        if path.contains("/.duckcoding/tool/bin/") || path.contains("\\.duckcoding\\tool\\bin\\") {
            ToolSource::DuckCodingManaged
        } else {
            ToolSource::External
        }
    }
}

/// SSH 连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSHConfig {
    /// 显示名称（如"开发服务器"、"生产环境"）
    pub display_name: String,
    /// 主机地址
    pub host: String,
    /// 端口
    pub port: u16,
    /// 用户名
    pub user: String,
    /// SSH 密钥路径（可选）
    pub key_path: Option<String>,
}

/// 工具实例（具体环境中的安装）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInstance {
    /// 实例唯一标识（如"claude-code-local", "codex-wsl-Ubuntu", "gemini-ssh-dev"）
    pub instance_id: String,
    /// 基础工具ID（claude-code, codex, gemini-cli）
    pub base_id: String,
    /// 工具名称（用于显示）
    pub tool_name: String,
    /// 环境类型
    pub tool_type: ToolType,
    /// 安装来源
    pub tool_source: ToolSource,
    /// 是否已安装
    pub installed: bool,
    /// 版本号
    pub version: Option<String>,
    /// 实际安装路径
    pub install_path: Option<String>,
    /// WSL发行版名称（仅WSL类型使用）
    pub wsl_distro: Option<String>,
    /// SSH配置（仅SSH类型使用）
    pub ssh_config: Option<SSHConfig>,
    /// 是否为内置实例（内置的本地工具实例）
    pub is_builtin: bool,
    /// 创建时间（Unix timestamp）
    pub created_at: i64,
    /// 更新时间（Unix timestamp）
    pub updated_at: i64,
}

impl ToolInstance {
    /// 从基础工具创建本地实例
    pub fn from_tool_local(
        tool: &Tool,
        installed: bool,
        version: Option<String>,
        install_path: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();
        let tool_source = install_path
            .as_ref()
            .map(|p| ToolSource::from_install_path(p))
            .unwrap_or(ToolSource::External);

        ToolInstance {
            instance_id: format!("{}-local", tool.id),
            base_id: tool.id.clone(),
            tool_name: tool.name.clone(),
            tool_type: ToolType::Local,
            tool_source,
            installed,
            version,
            install_path,
            wsl_distro: None,
            ssh_config: None,
            is_builtin: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// 创建WSL实例
    pub fn create_wsl_instance(
        base_id: String,
        tool_name: String,
        distro_name: String,
        installed: bool,
        version: Option<String>,
        install_path: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();
        let tool_source = install_path
            .as_ref()
            .map(|p| ToolSource::from_install_path(p))
            .unwrap_or(ToolSource::External);

        // instance_id 格式: {base_id}-wsl-{distro_name}
        let sanitized_distro = distro_name.to_lowercase().replace(' ', "-");

        ToolInstance {
            instance_id: format!("{}-wsl-{}", base_id, sanitized_distro),
            base_id,
            tool_name,
            tool_type: ToolType::WSL,
            tool_source,
            installed,
            version,
            install_path,
            wsl_distro: Some(distro_name),
            ssh_config: None,
            is_builtin: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// 创建SSH实例
    pub fn create_ssh_instance(
        base_id: String,
        tool_name: String,
        ssh_config: SSHConfig,
        installed: bool,
        version: Option<String>,
        install_path: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();
        let ssh_display_name = ssh_config.display_name.clone();
        let tool_source = install_path
            .as_ref()
            .map(|p| ToolSource::from_install_path(p))
            .unwrap_or(ToolSource::External);

        ToolInstance {
            instance_id: format!(
                "{}-ssh-{}",
                base_id,
                ssh_display_name.to_lowercase().replace(' ', "-")
            ),
            base_id,
            tool_name,
            tool_type: ToolType::SSH,
            tool_source,
            installed,
            version,
            install_path,
            wsl_distro: None,
            ssh_config: Some(ssh_config),
            is_builtin: false,
            created_at: now,
            updated_at: now,
        }
    }
}
