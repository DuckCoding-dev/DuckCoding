//! 透明代理配置数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 单个工具的透明代理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProxyConfig {
    pub enabled: bool,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub real_api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub real_base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub real_profile_name: Option<String>,
    #[serde(default)]
    pub allow_public: bool,
    #[serde(default)]
    pub session_endpoint_config_enabled: bool,
    #[serde(default)]
    pub auto_start: bool,
    /// 启动代理前激活的 Profile 名称（用于关闭时还原）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_active_profile: Option<String>,
    /// AMP Code 原始 settings.json 完整内容（用于关闭时还原，语义备份）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_amp_settings: Option<Value>,
    /// AMP Code 原始 secrets.json 完整内容（用于关闭时还原，语义备份）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_amp_secrets: Option<Value>,
    /// Tavily API Key（用于本地搜索，可选，无则降级 DuckDuckGo）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tavily_api_key: Option<String>,
}

impl ToolProxyConfig {
    /// 创建默认配置
    pub fn new(port: u16) -> Self {
        Self {
            enabled: false,
            port,
            local_api_key: None,
            real_api_key: None,
            real_base_url: None,
            real_profile_name: None,
            allow_public: false,
            session_endpoint_config_enabled: false,
            auto_start: false,
            original_active_profile: None,
            original_amp_settings: None,
            original_amp_secrets: None,
            tavily_api_key: None,
        }
    }

    /// 默认端口配置
    pub fn default_port(tool_id: &str) -> u16 {
        match tool_id {
            "claude-code" => 8787,
            "codex" => 8788,
            "gemini-cli" => 8789,
            "amp-code" => 8790,
            _ => 8787,
        }
    }
}

/// proxy.json 顶层结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyStore {
    pub version: String,
    #[serde(rename = "claude-code")]
    pub claude_code: ToolProxyConfig,
    pub codex: ToolProxyConfig,
    #[serde(rename = "gemini-cli")]
    pub gemini_cli: ToolProxyConfig,
    #[serde(rename = "amp-code", default = "default_amp_config")]
    pub amp_code: ToolProxyConfig,
    pub metadata: ProxyMetadata,
}

fn default_amp_config() -> ToolProxyConfig {
    ToolProxyConfig::new(8790)
}

impl ProxyStore {
    pub fn new() -> Self {
        Self {
            version: "2.1.0".to_string(),
            claude_code: ToolProxyConfig::new(8787),
            codex: ToolProxyConfig::new(8788),
            gemini_cli: ToolProxyConfig::new(8789),
            amp_code: ToolProxyConfig::new(8790),
            metadata: ProxyMetadata {
                last_updated: Utc::now(),
            },
        }
    }

    /// 获取指定工具的配置
    pub fn get_config(&self, tool_id: &str) -> Option<&ToolProxyConfig> {
        match tool_id {
            "claude-code" => Some(&self.claude_code),
            "codex" => Some(&self.codex),
            "gemini-cli" => Some(&self.gemini_cli),
            "amp-code" => Some(&self.amp_code),
            _ => None,
        }
    }

    /// 获取指定工具的可变配置
    pub fn get_config_mut(&mut self, tool_id: &str) -> Option<&mut ToolProxyConfig> {
        match tool_id {
            "claude-code" => Some(&mut self.claude_code),
            "codex" => Some(&mut self.codex),
            "gemini-cli" => Some(&mut self.gemini_cli),
            "amp-code" => Some(&mut self.amp_code),
            _ => None,
        }
    }

    /// 更新指定工具的配置
    pub fn update_config(&mut self, tool_id: &str, config: ToolProxyConfig) {
        match tool_id {
            "claude-code" => self.claude_code = config,
            "codex" => self.codex = config,
            "gemini-cli" => self.gemini_cli = config,
            "amp-code" => self.amp_code = config,
            _ => {}
        }
        self.metadata.last_updated = Utc::now();
    }
}

impl Default for ProxyStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyMetadata {
    pub last_updated: DateTime<Utc>,
}
