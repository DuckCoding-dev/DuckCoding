// filepath: e:\DuckCoding\src-tauri\src\models\config.rs

// 全局配置结构，移动到 models 以便在库和二进制之间共享
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct GlobalConfig {
    pub user_id: String,
    pub system_token: String,
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
    // 透明代理功能 (实验性)
    #[serde(default)]
    pub transparent_proxy_enabled: bool,
    #[serde(default = "default_transparent_proxy_port")]
    pub transparent_proxy_port: u16,
    #[serde(default)]
    pub transparent_proxy_api_key: Option<String>, // 用于保护本地代理的 API Key
    // 保存真实的 ClaudeCode API 配置（透明代理启用时使用）
    #[serde(default)]
    pub transparent_proxy_real_api_key: Option<String>,
    #[serde(default)]
    pub transparent_proxy_real_base_url: Option<String>,
    // 允许局域网访问透明代理（默认仅本地访问）
    #[serde(default)]
    pub transparent_proxy_allow_public: bool,
}

fn default_transparent_proxy_port() -> u16 {
    8787
}
