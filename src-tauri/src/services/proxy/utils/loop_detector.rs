//! 代理回环检测工具
//!
//! 防止代理配置指向自身导致无限循环

/// 检查目标 URL 是否指向自身代理端口
///
/// # 参数
/// - `target_url`: 目标 URL
/// - `own_port`: 当前代理监听的端口
///
/// # 返回
/// - `true`: 检测到回环
/// - `false`: 未检测到回环
pub fn is_proxy_loop(target_url: &str, own_port: u16) -> bool {
    let loop_urls = vec![
        format!("http://127.0.0.1:{}", own_port),
        format!("https://127.0.0.1:{}", own_port),
        format!("http://localhost:{}", own_port),
        format!("https://localhost:{}", own_port),
    ];

    for loop_url in &loop_urls {
        if target_url.starts_with(loop_url) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loop_detection() {
        assert!(is_proxy_loop("http://127.0.0.1:8787/v1/messages", 8787));
        assert!(is_proxy_loop("https://localhost:8787/api", 8787));
        assert!(!is_proxy_loop(
            "https://api.anthropic.com/v1/messages",
            8787
        ));
        assert!(!is_proxy_loop("http://127.0.0.1:8788/v1/messages", 8787));
    }
}
