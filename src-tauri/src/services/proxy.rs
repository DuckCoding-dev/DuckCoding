use crate::GlobalConfig;
use std::env;

/// 代理服务 - 负责应用代理配置到环境变量
pub struct ProxyService;

impl ProxyService {
    /// 从全局配置应用代理到环境变量
    /// 这会设置 HTTP_PROXY, HTTPS_PROXY, ALL_PROXY 等环境变量
    pub fn apply_proxy_from_config(config: &GlobalConfig) {
        // 清除可能存在的旧代理设置
        Self::clear_proxy();

        if !config.proxy_enabled {
            return;
        }

        // 构建代理 URL
        if let Some(proxy_url) = Self::build_proxy_url(config) {
            // 为了兼容各种库和平台，设置常用的代理环境变量（大写和小写）
            // 一些库只识别 HTTP_PROXY/HTTPS_PROXY，其他库或工具识别 ALL_PROXY
            env::set_var("HTTP_PROXY", &proxy_url);
            env::set_var("http_proxy", &proxy_url);
            env::set_var("HTTPS_PROXY", &proxy_url);
            env::set_var("https_proxy", &proxy_url);
            env::set_var("ALL_PROXY", &proxy_url);
            env::set_var("all_proxy", &proxy_url);

            println!("Proxy enabled: {}", proxy_url);
        }
    }

    /// 构建代理 URL
    fn build_proxy_url(config: &GlobalConfig) -> Option<String> {
        let host = config.proxy_host.as_ref()?;
        let port = config.proxy_port.as_ref()?;

        if host.is_empty() || port.is_empty() {
            return None;
        }

        let proxy_type = config.proxy_type.as_deref().unwrap_or("http");

        // 构建认证部分
        let auth = if let (Some(username), Some(password)) = (
            config.proxy_username.as_ref(),
            config.proxy_password.as_ref(),
        ) {
            if !username.is_empty() && !password.is_empty() {
                format!("{}:{}@", username, password)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // 对于 socks5，使用标准 scheme；其他情形使用 http/https
        let scheme = match proxy_type {
            "socks5" => "socks5",
            "https" => "https",
            _ => "http",
        };

        // 构建完整的代理 URL
        Some(format!("{}://{}{}:{}", scheme, auth, host, port))
    }

    /// 清除代理环境变量
    pub fn clear_proxy() {
        env::remove_var("HTTP_PROXY");
        env::remove_var("http_proxy");
        env::remove_var("HTTPS_PROXY");
        env::remove_var("https_proxy");
        env::remove_var("ALL_PROXY");
        env::remove_var("all_proxy");
    }

    /// 获取当前代理设置（用于调试）
    pub fn get_current_proxy() -> Option<String> {
        env::var("HTTP_PROXY")
            .or_else(|_| env::var("http_proxy"))
            .or_else(|_| env::var("HTTPS_PROXY"))
            .or_else(|_| env::var("https_proxy"))
            .or_else(|_| env::var("ALL_PROXY"))
            .or_else(|_| env::var("all_proxy"))
            .ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_proxy_url_basic() {
        let config = GlobalConfig {
            user_id: String::new(),
            system_token: String::new(),
            proxy_enabled: true,
            proxy_type: Some("http".to_string()),
            proxy_host: Some("127.0.0.1".to_string()),
            proxy_port: Some("7890".to_string()),
            proxy_username: None,
            proxy_password: None,
            transparent_proxy_enabled: false,
            transparent_proxy_port: 8787,
            transparent_proxy_api_key: None,
            transparent_proxy_real_api_key: None,
            transparent_proxy_real_base_url: None,
            transparent_proxy_allow_public: false,
        };

        let url = ProxyService::build_proxy_url(&config);
        assert_eq!(url, Some("http://127.0.0.1:7890".to_string()));
    }

    #[test]
    fn test_build_proxy_url_with_auth() {
        let config = GlobalConfig {
            user_id: String::new(),
            system_token: String::new(),
            proxy_enabled: true,
            proxy_type: Some("http".to_string()),
            proxy_host: Some("proxy.example.com".to_string()),
            proxy_port: Some("8080".to_string()),
            proxy_username: Some("user".to_string()),
            proxy_password: Some("pass".to_string()),
            transparent_proxy_enabled: false,
            transparent_proxy_port: 8787,
            transparent_proxy_api_key: None,
            transparent_proxy_real_api_key: None,
            transparent_proxy_real_base_url: None,
            transparent_proxy_allow_public: false,
        };

        let url = ProxyService::build_proxy_url(&config);
        assert_eq!(
            url,
            Some("http://user:pass@proxy.example.com:8080".to_string())
        );
    }

    #[test]
    fn test_build_proxy_url_socks5() {
        let config = GlobalConfig {
            user_id: String::new(),
            system_token: String::new(),
            proxy_enabled: true,
            proxy_type: Some("socks5".to_string()),
            proxy_host: Some("127.0.0.1".to_string()),
            proxy_port: Some("1080".to_string()),
            proxy_username: None,
            proxy_password: None,
            transparent_proxy_enabled: false,
            transparent_proxy_port: 8787,
            transparent_proxy_api_key: None,
            transparent_proxy_real_api_key: None,
            transparent_proxy_real_base_url: None,
            transparent_proxy_allow_public: false,
        };

        let url = ProxyService::build_proxy_url(&config);
        assert_eq!(url, Some("socks5://127.0.0.1:1080".to_string()));
    }
}
