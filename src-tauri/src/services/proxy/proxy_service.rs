use crate::GlobalConfig;
use std::env;
use url::Url;

/// 代理服务 - 负责应用代理配置到环境变量
pub struct ProxyService;

impl ProxyService {
    /// 检查给定的URL是否应该绕过代理
    pub fn should_bypass_proxy(url: &str, bypass_list: &[String]) -> bool {
        // 如果没有过滤规则，不绕过
        if bypass_list.is_empty() {
            return false;
        }

        let parsed_url = match Url::parse(url) {
            Ok(url) => url,
            Err(_) => {
                // 如果URL解析失败，尝试作为主机名处理
                return Self::should_bypass_host(url, bypass_list);
            }
        };

        // 获取主机名进行匹配
        let host = parsed_url.host_str().unwrap_or("");

        Self::should_bypass_host(host, bypass_list)
    }

    /// 检查主机名是否应该绕过代理
    fn should_bypass_host(host: &str, bypass_list: &[String]) -> bool {
        for pattern in bypass_list {
            if Self::matches_pattern(host, pattern) {
                return true;
            }
        }
        false
    }

    /// 检查主机名是否匹配给定的模式
    fn matches_pattern(host: &str, pattern: &str) -> bool {
        let pattern = pattern.trim();
        let host = host.trim().to_lowercase();
        let pattern = pattern.to_lowercase();

        // 精确匹配
        if host == pattern {
            return true;
        }

        // 通配符匹配
        if let Some(domain) = pattern.strip_prefix("*.") {
            if host.ends_with(domain) || host == domain {
                return true;
            }
        }

        // 简单的通配符匹配（支持 *）
        if pattern.contains('*') {
            return Self::wildcard_match(&host, &pattern);
        }

        // IP段匹配（例如 192.168.*）
        if pattern.contains('*') && !pattern.starts_with("*.") {
            let parts: Vec<&str> = pattern.split('.').collect();
            let host_parts: Vec<&str> = host.split('.').collect();

            if parts.len() == host_parts.len() {
                for (i, part) in parts.iter().enumerate() {
                    let host_part = host_parts.get(i).unwrap_or(&"");
                    if *part != "*" && *part != *host_part {
                        return false;
                    }
                }
                return true;
            }
        }

        false
    }

    /// 简单的通配符匹配
    fn wildcard_match(text: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if !pattern.contains('*') {
            return text == pattern;
        }

        // 将模式按 * 分割
        let parts: Vec<&str> = pattern.split('*').collect();

        if parts.len() == 2 {
            let prefix = parts[0];
            let suffix = parts[1];

            return text.starts_with(prefix) && text.ends_with(suffix);
        }

        // 更复杂的模式，简化处理
        text.contains(&pattern.replace('*', ""))
    }

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

            // 设置绕过代理的环境变量
            let bypass_urls: Vec<String> = config
                .proxy_bypass_urls
                .iter()
                .map(|url| url.trim().to_string())
                .filter(|url| !url.is_empty())
                .collect();

            if !bypass_urls.is_empty() {
                let no_proxy = bypass_urls.join(",");
                env::set_var("NO_PROXY", &no_proxy);
                env::set_var("no_proxy", &no_proxy);
                println!("Proxy bypass list: {}", no_proxy);
            }

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
        env::remove_var("NO_PROXY");
        env::remove_var("no_proxy");
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
            proxy_bypass_urls: vec![],
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
            proxy_bypass_urls: vec![],
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
            proxy_bypass_urls: vec![],
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

    #[test]
    fn test_should_bypass_proxy_exact_match() {
        let bypass_list = vec!["localhost".to_string(), "127.0.0.1".to_string()];

        assert!(ProxyService::should_bypass_proxy("localhost", &bypass_list));
        assert!(ProxyService::should_bypass_proxy("127.0.0.1", &bypass_list));
        assert!(!ProxyService::should_bypass_proxy(
            "example.com",
            &bypass_list
        ));
    }

    #[test]
    fn test_should_bypass_proxy_wildcard_match() {
        let bypass_list = vec![
            "*.local".to_string(),
            "*.lan".to_string(),
            "192.168.*".to_string(),
        ];

        assert!(ProxyService::should_bypass_proxy(
            "test.local",
            &bypass_list
        ));
        assert!(ProxyService::should_bypass_proxy("home.lan", &bypass_list));
        assert!(ProxyService::should_bypass_proxy(
            "192.168.1.1",
            &bypass_list
        ));
        assert!(ProxyService::should_bypass_proxy(
            "192.168.100.50",
            &bypass_list
        ));
        assert!(!ProxyService::should_bypass_proxy(
            "192.167.1.1",
            &bypass_list
        ));
        assert!(!ProxyService::should_bypass_proxy("test.com", &bypass_list));
    }

    #[test]
    fn test_should_bypass_proxy_with_url() {
        let bypass_list = vec!["localhost".to_string(), "192.168.*".to_string()];

        assert!(ProxyService::should_bypass_proxy(
            "http://localhost:3000",
            &bypass_list
        ));
        assert!(ProxyService::should_bypass_proxy(
            "https://192.168.1.100/api",
            &bypass_list
        ));
        assert!(!ProxyService::should_bypass_proxy(
            "https://example.com",
            &bypass_list
        ));
    }

    #[test]
    fn test_should_bypass_proxy_case_insensitive() {
        let bypass_list = vec!["LOCALHOST".to_string(), "Example.Com".to_string()];

        assert!(ProxyService::should_bypass_proxy("localhost", &bypass_list));
        assert!(ProxyService::should_bypass_proxy(
            "example.com",
            &bypass_list
        ));
    }
}
