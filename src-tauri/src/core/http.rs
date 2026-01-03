use crate::core::error::{AppError, AppResult};
use crate::models::GlobalConfig;
use reqwest::Client;

const USER_AGENT: &str = concat!("DuckCoding/", env!("CARGO_PKG_VERSION"));

/// 构建带代理配置的 HTTP 客户端
///
/// # 参数
/// - `config`: 可选的全局配置（包含代理设置）
///
/// # 返回
/// - 配置好的 reqwest::Client
pub fn build_http_client(config: Option<&GlobalConfig>) -> AppResult<Client> {
    let mut builder = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(300)) // 5分钟超时
        .redirect(reqwest::redirect::Policy::limited(10)); // 支持重定向

    // 应用代理配置
    if let Some(cfg) = config {
        if cfg.proxy_enabled {
            let proxy_url = build_proxy_url(cfg)?;
            let proxy =
                reqwest::Proxy::all(&proxy_url).map_err(|e| AppError::ProxyConfigError {
                    reason: format!("代理 URL 无效: {e}"),
                })?;

            builder = builder.proxy(proxy);
        }
    }

    builder.build().map_err(|e| AppError::Other(e.into()))
}

/// 获取全局 HTTP 客户端（应用当前代理配置）
///
/// 优先读取由 ProxyService 写入的环境变量
pub fn get_global_client() -> AppResult<Client> {
    crate::http_client::build_client().map_err(|e| AppError::ProxyConfigError { reason: e })
}

/// 构建代理 URL
fn build_proxy_url(config: &GlobalConfig) -> AppResult<String> {
    let proxy_type = config
        .proxy_type
        .as_ref()
        .ok_or_else(|| AppError::ProxyConfigError {
            reason: "代理类型未设置".to_string(),
        })?;

    let host = config
        .proxy_host
        .as_ref()
        .ok_or_else(|| AppError::ProxyConfigError {
            reason: "代理主机未设置".to_string(),
        })?;

    let port = config
        .proxy_port
        .as_ref()
        .ok_or_else(|| AppError::ProxyConfigError {
            reason: "代理端口未设置".to_string(),
        })?;

    // 构建认证部分
    let auth = if let (Some(username), Some(password)) =
        (&config.proxy_username, &config.proxy_password)
    {
        if !username.is_empty() && !password.is_empty() {
            format!("{username}:{password}@")
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // 构建完整 URL
    let scheme = match proxy_type.as_str() {
        "socks5" => "socks5",
        "https" => "https",
        "http" => "http",
        _ => "http",
    };

    Ok(format!("{scheme}://{auth}{host}:{port}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_proxy_url_http() {
        let config = GlobalConfig {
            version: None,
            user_id: "test".to_string(),
            system_token: "test".to_string(),
            proxy_enabled: true,
            proxy_type: Some("http".to_string()),
            proxy_host: Some("127.0.0.1".to_string()),
            proxy_port: Some("8080".to_string()),
            proxy_username: None,
            proxy_password: None,
            proxy_bypass_urls: vec![],
            proxy_configs: std::collections::HashMap::new(),
            session_endpoint_config_enabled: false,
            hide_transparent_proxy_tip: false,
            hide_session_config_hint: false,
            log_config: crate::models::config::LogConfig::default(),
            onboarding_status: None,
            external_watch_enabled: true,
            external_poll_interval_ms: 5000,
            single_instance_enabled: true,
            startup_enabled: false,
        };

        let url = build_proxy_url(&config).unwrap();
        assert_eq!(url, "http://127.0.0.1:8080");
    }

    #[test]
    fn test_build_proxy_url_with_auth() {
        let config = GlobalConfig {
            version: None,
            user_id: "test".to_string(),
            system_token: "test".to_string(),
            proxy_enabled: true,
            proxy_type: Some("socks5".to_string()),
            proxy_host: Some("proxy.example.com".to_string()),
            proxy_port: Some("1080".to_string()),
            proxy_username: Some("user".to_string()),
            proxy_password: Some("pass".to_string()),
            proxy_bypass_urls: vec![],
            proxy_configs: std::collections::HashMap::new(),
            session_endpoint_config_enabled: false,
            hide_transparent_proxy_tip: false,
            hide_session_config_hint: false,
            log_config: crate::models::config::LogConfig::default(),
            onboarding_status: None,
            external_watch_enabled: true,
            external_poll_interval_ms: 5000,
            single_instance_enabled: true,
            startup_enabled: false,
        };

        let url = build_proxy_url(&config).unwrap();
        assert_eq!(url, "socks5://user:pass@proxy.example.com:1080");
    }
}
