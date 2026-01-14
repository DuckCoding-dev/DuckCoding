// filepath: e:\DuckCoding\src-tauri\src\models\config.rs

// 全局配置结构，移动到 models 以便在库和二进制之间共享
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 日志级别
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Default for LogLevel {
    fn default() -> Self {
        if cfg!(debug_assertions) {
            LogLevel::Debug
        } else {
            LogLevel::Info
        }
    }
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }
}

/// 日志输出格式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Json,
    #[default]
    Text,
}

/// 日志输出目标
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogOutput {
    Console,
    File,
    #[default]
    Both,
}

/// 日志系统配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LogConfig {
    #[serde(default)]
    pub level: LogLevel,
    #[serde(default)]
    pub format: LogFormat,
    #[serde(default)]
    pub output: LogOutput,
    #[serde(default)]
    pub file_path: Option<String>,
}

/// 新用户引导状态
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OnboardingStatus {
    /// 已完成的引导版本（例如："v1", "v2"）
    pub completed_version: String,
    /// 跳过的步骤 ID 列表
    #[serde(default)]
    pub skipped_steps: Vec<String>,
    /// 完成时间戳（ISO 8601 格式）
    pub completed_at: Option<String>,
}

/// 配置监听模式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum WatchMode {
    /// 默认模式：仅检测敏感字段（API Key/URL）
    #[default]
    Default,
    /// 全量模式：检测所有变更
    Full,
}

/// 配置监听配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigWatchConfig {
    /// 是否启用配置守护
    #[serde(default = "default_config_guard_enabled")]
    pub enabled: bool,
    /// 监听模式
    #[serde(default)]
    pub mode: WatchMode,
    /// 扫描间隔（秒）
    #[serde(default = "default_scan_interval")]
    pub scan_interval: u64,
    /// 黑名单字段（按工具分组）
    /// 格式：{ "claude-code": ["env.model", "theme"], ... }
    #[serde(default = "default_watch_blacklist")]
    pub blacklist: HashMap<String, Vec<String>>,
    /// 敏感字段（按工具分组）
    /// 格式：{ "claude-code": ["env.ANTHROPIC_AUTH_TOKEN", ...], ... }
    #[serde(default = "default_sensitive_fields")]
    pub sensitive_fields: HashMap<String, Vec<String>>,
}

/// Token统计配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStatsConfig {
    /// 数据保留天数（None表示不限制）
    #[serde(default)]
    pub retention_days: Option<u32>,
    /// 最大日志条数（None表示不限制）
    #[serde(default)]
    pub max_log_count: Option<u32>,
    /// 是否启用自动清理
    #[serde(default = "default_auto_cleanup_enabled")]
    pub auto_cleanup_enabled: bool,
}

impl Default for TokenStatsConfig {
    fn default() -> Self {
        Self {
            retention_days: Some(30),
            max_log_count: Some(10000),
            auto_cleanup_enabled: true,
        }
    }
}

fn default_auto_cleanup_enabled() -> bool {
    true
}

/// 配置文件快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSnapshot {
    /// 工具 ID
    pub tool_id: String,
    /// 配置文件快照（文件名 -> 内容）
    /// 对于 JSON 文件，直接存储 JSON 值
    /// 对于 TOML/ENV 文件，转换为 JSON 对象存储
    pub files: HashMap<String, serde_json::Value>,
    /// 最后更新时间
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl Default for ConfigWatchConfig {
    fn default() -> Self {
        Self {
            enabled: default_config_guard_enabled(),
            mode: WatchMode::Default,
            scan_interval: default_scan_interval(),
            blacklist: default_watch_blacklist(),
            sensitive_fields: default_sensitive_fields(),
        }
    }
}

impl LogConfig {
    /// 检查新配置是否可以热重载（无需重启应用）
    /// 只有日志级别变更可以热重载，其他配置需要重启
    pub fn can_hot_reload(&self, other: &LogConfig) -> bool {
        self.format == other.format
            && self.output == other.output
            && self.file_path == other.file_path
    }
}

/// 单个工具的透明代理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProxyConfig {
    pub enabled: bool,
    pub port: u16,
    pub local_api_key: Option<String>,       // 保护密钥
    pub real_api_key: Option<String>,        // 备份的真实 API Key
    pub real_base_url: Option<String>,       // 备份的真实 Base URL
    pub real_model_provider: Option<String>, // 备份的 model_provider (Codex 专用)
    #[serde(default)]
    pub real_profile_name: Option<String>, // 备份的配置名称
    pub allow_public: bool,
    #[serde(default)]
    pub session_endpoint_config_enabled: bool, // 工具级：是否允许会话自定义端点
    #[serde(default)]
    pub auto_start: bool, // 应用启动时自动运行代理（默认关闭）
    /// 启动代理前激活的 Profile 名称（用于关闭时还原）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_active_profile: Option<String>,
    /// 价格模板 ID（用于成本计算）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pricing_template_id: Option<String>,
}

impl Default for ToolProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 8787,
            local_api_key: None,
            real_api_key: None,
            real_base_url: None,
            real_model_provider: None,
            real_profile_name: None,
            allow_public: false,
            session_endpoint_config_enabled: false,
            auto_start: false,
            original_active_profile: None,
            pricing_template_id: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GlobalConfig {
    /// 配置文件版本（用于迁移管理）
    /// 默认值为 "0.0.0"，迁移后更新为对应的应用版本号
    #[serde(default)]
    pub version: Option<String>,
    /// 已废弃，由供应商系统管理
    #[serde(default)]
    pub user_id: Option<String>,
    /// 已废弃，由供应商系统管理
    #[serde(default)]
    pub system_token: Option<String>,
    #[serde(default)]
    pub proxy_enabled: bool,
    #[serde(default)]
    pub proxy_type: Option<String>, // "http", "https", "socks5"
    #[serde(default)]
    pub proxy_host: Option<String>,
    #[serde(default)]
    pub proxy_port: Option<String>,
    #[serde(default)]
    pub proxy_username: Option<String>,
    #[serde(default)]
    pub proxy_password: Option<String>,
    #[serde(default)]
    pub proxy_bypass_urls: Vec<String>, // 代理过滤URL列表
    // 多工具透明代理配置（新架构）
    #[serde(default = "default_proxy_configs")]
    pub proxy_configs: HashMap<String, ToolProxyConfig>,
    // 会话级端点配置开关（默认关闭）
    #[serde(default)]
    pub session_endpoint_config_enabled: bool,
    // 是否隐藏透明代理推荐提示（默认显示）
    #[serde(default)]
    pub hide_transparent_proxy_tip: bool,
    // 是否隐藏会话级端点配置提示（默认显示）
    #[serde(default)]
    pub hide_session_config_hint: bool,
    // 日志系统配置
    #[serde(default)]
    pub log_config: LogConfig,
    // 新用户引导状态
    #[serde(default)]
    pub onboarding_status: Option<OnboardingStatus>,
    /// 外部改动监听是否开启（notify + 轮询）
    #[serde(default = "default_external_watch_enabled")]
    pub external_watch_enabled: bool,
    /// 外部改动轮询间隔（毫秒），用于前端补偿刷新
    #[serde(default = "default_external_poll_interval_ms")]
    pub external_poll_interval_ms: u64,
    /// 单实例模式开关（默认启用，仅生产环境生效）
    #[serde(default = "default_single_instance_enabled")]
    pub single_instance_enabled: bool,
    /// 开机自启动开关（默认关闭）
    #[serde(default)]
    pub startup_enabled: bool,
    /// 配置监听配置
    #[serde(default)]
    pub config_watch: ConfigWatchConfig,
    /// Token统计配置
    #[serde(default)]
    pub token_stats_config: TokenStatsConfig,
}

fn default_proxy_configs() -> HashMap<String, ToolProxyConfig> {
    let mut configs = HashMap::new();

    configs.insert(
        "claude-code".to_string(),
        ToolProxyConfig {
            enabled: false,
            port: 8787,
            local_api_key: None,
            real_api_key: None,
            real_base_url: None,
            real_model_provider: None,
            real_profile_name: None,
            allow_public: false,
            session_endpoint_config_enabled: false,
            auto_start: false,
            original_active_profile: None,
            pricing_template_id: None,
        },
    );

    configs.insert(
        "codex".to_string(),
        ToolProxyConfig {
            enabled: false,
            port: 8788,
            local_api_key: None,
            real_api_key: None,
            real_base_url: None,
            real_model_provider: None,
            real_profile_name: None,
            allow_public: false,
            session_endpoint_config_enabled: false,
            auto_start: false,
            original_active_profile: None,
            pricing_template_id: None,
        },
    );

    configs.insert(
        "gemini-cli".to_string(),
        ToolProxyConfig {
            enabled: false,
            port: 8789,
            local_api_key: None,
            real_api_key: None,
            real_base_url: None,
            real_model_provider: None,
            real_profile_name: None,
            allow_public: false,
            session_endpoint_config_enabled: false,
            auto_start: false,
            original_active_profile: None,
            pricing_template_id: None,
        },
    );

    configs.insert(
        "amp-code".to_string(),
        ToolProxyConfig {
            enabled: false,
            port: 8790,
            local_api_key: None,
            real_api_key: None,
            real_base_url: None,
            real_model_provider: None,
            real_profile_name: None,
            allow_public: false,
            session_endpoint_config_enabled: false,
            auto_start: false,
            original_active_profile: None,
        },
    );

    configs
}

impl GlobalConfig {
    /// 获取指定工具的代理配置
    pub fn get_proxy_config(&self, tool_id: &str) -> Option<&ToolProxyConfig> {
        self.proxy_configs.get(tool_id)
    }

    /// 获取指定工具的可变代理配置
    pub fn get_proxy_config_mut(&mut self, tool_id: &str) -> Option<&mut ToolProxyConfig> {
        self.proxy_configs.get_mut(tool_id)
    }

    /// 确保工具的代理配置存在（如果不存在则创建默认配置）
    pub fn ensure_proxy_config(&mut self, tool_id: &str, default_port: u16) {
        self.proxy_configs
            .entry(tool_id.to_string())
            .or_insert_with(|| ToolProxyConfig {
                enabled: false,
                port: default_port,
                local_api_key: None,
                real_api_key: None,
                real_base_url: None,
                real_model_provider: None,
                real_profile_name: None,
                allow_public: false,
                session_endpoint_config_enabled: false,
                auto_start: false,
                original_active_profile: None,
                pricing_template_id: None,
            });
    }

    /// 自动迁移旧的全局会话开关到工具级
    /// 如果全局开关已启用，则将其值迁移到每个工具的配置中
    pub fn migrate_session_config(&mut self) {
        // 仅在全局开关为 true 时进行迁移
        if self.session_endpoint_config_enabled {
            for config in self.proxy_configs.values_mut() {
                // 仅迁移尚未设置的工具
                if !config.session_endpoint_config_enabled {
                    config.session_endpoint_config_enabled = true;
                }
            }
            // 迁移完成后，保留旧字段但不再使用（向后兼容）
            // 可选：self.session_endpoint_config_enabled = false;
        }
    }
}

fn default_external_watch_enabled() -> bool {
    true
}

fn default_external_poll_interval_ms() -> u64 {
    5000
}

fn default_single_instance_enabled() -> bool {
    true
}

/// 默认配置守护启用状态
fn default_config_guard_enabled() -> bool {
    true
}

/// 默认扫描间隔（秒）
fn default_scan_interval() -> u64 {
    2
}

pub fn default_watch_blacklist() -> HashMap<String, Vec<String>> {
    let mut map = HashMap::new();
    map.insert(
        "claude-code".to_string(),
        vec![
            "theme".to_string(),
            "keyBindings".to_string(),
            "ui.*".to_string(),
            "editor.*".to_string(),
        ],
    );
    map.insert(
        "codex".to_string(),
        vec![
            "network_access".to_string(),
            "ui.*".to_string(),
            "editor.*".to_string(),
        ],
    );
    map.insert(
        "gemini-cli".to_string(),
        vec![
            "ui.*".to_string(),
            "cache.*".to_string(),
            "log_level".to_string(),
        ],
    );
    map
}

pub fn default_sensitive_fields() -> HashMap<String, Vec<String>> {
    let mut map = HashMap::new();
    map.insert(
        "claude-code".to_string(),
        vec![
            "env.ANTHROPIC_AUTH_TOKEN".to_string(),
            "env.ANTHROPIC_BASE_URL".to_string(),
        ],
    );
    map.insert(
        "codex".to_string(),
        vec![
            "auth.json:OPENAI_API_KEY".to_string(),
            "model_provider".to_string(),
            "model_providers.*.base_url".to_string(),
            "model_providers.*.wire_api".to_string(),
        ],
    );
    map.insert(
        "gemini-cli".to_string(),
        vec![
            ".env:GEMINI_API_KEY".to_string(),
            ".env:GOOGLE_GEMINI_BASE_URL".to_string(),
        ],
    );
    map
}
