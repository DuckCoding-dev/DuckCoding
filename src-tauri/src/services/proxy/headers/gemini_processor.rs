// Gemini CLI 请求处理器

use super::{ProcessedRequest, RequestProcessor};
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use hyper::HeaderMap as HyperHeaderMap;
use reqwest::header::HeaderMap as ReqwestHeaderMap;

/// Gemini CLI 专用请求处理器
///
/// 处理 Google Gemini API 的请求转换：
/// - URL 构建：使用标准拼接（无特殊逻辑）
/// - 认证方式：x-goog-api-key header
/// - API Key 格式：直接的 key 字符串（不需要 Bearer 前缀）
///
/// # TODO
/// 根据实际需求添加：
/// - x-goog-user-project header 处理（计费项目）
/// - OAuth 2.0 令牌支持（如果 Gemini CLI 使用 OAuth）
#[derive(Debug)]
pub struct GeminiHeadersProcessor;

#[async_trait]
impl RequestProcessor for GeminiHeadersProcessor {
    fn tool_id(&self) -> &str {
        "gemini-cli"
    }

    async fn process_outgoing_request(
        &self,
        base_url: &str,
        api_key: &str,
        path: &str,
        query: Option<&str>,
        original_headers: &HyperHeaderMap,
        body: &[u8],
    ) -> Result<ProcessedRequest> {
        // 1. 构建目标 URL（标准拼接）
        let base = base_url.trim_end_matches('/');
        let query_str = query.map(|q| format!("?{q}")).unwrap_or_default();
        let target_url = format!("{base}{path}{query_str}");

        // 2. 处理 headers（复制非认证 headers）
        let mut headers = ReqwestHeaderMap::new();
        for (name, value) in original_headers.iter() {
            let name_str = name.as_str();
            // 跳过认证相关和 Host headers
            if name_str.eq_ignore_ascii_case("host")
                || name_str.eq_ignore_ascii_case("x-goog-api-key")
                || name_str.eq_ignore_ascii_case("authorization")
                || name_str.eq_ignore_ascii_case("x-api-key")
            {
                continue;
            }
            headers.insert(name.clone(), value.clone());
        }

        // 3. 添加真实的 Google API Key
        // Google APIs 通常使用 x-goog-api-key header
        headers.insert(
            "x-goog-api-key",
            api_key
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid x-goog-api-key header: {e}"))?,
        );

        // TODO: 根据需要添加其他 Google 特定的 headers
        // 例如：
        // if let Some(project_id) = get_project_id() {
        //     headers.insert("x-goog-user-project", project_id.parse()?);
        // }

        // 4. 返回处理后的请求
        Ok(ProcessedRequest {
            target_url,
            headers,
            body: Bytes::copy_from_slice(body),
        })
    }

    // Gemini CLI 当前不需要特殊的响应处理
    // 如果未来需要（例如处理配额信息），可以在此实现
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::HeaderMap as HyperHeaderMap;

    #[tokio::test]
    async fn test_x_goog_api_key_header_added() {
        let processor = GeminiHeadersProcessor;
        let headers = HyperHeaderMap::new();
        let api_key = "test-api-key-12345";

        let result = processor
            .process_outgoing_request(
                "https://generativelanguage.googleapis.com",
                api_key,
                "/v1beta/models/gemini-2.0-flash:generateContent",
                None,
                &headers,
                b"{}",
            )
            .await;

        assert!(result.is_ok());
        let processed = result.unwrap();

        // 验证 x-goog-api-key header 存在且值正确
        assert_eq!(
            processed
                .headers
                .get("x-goog-api-key")
                .and_then(|v| v.to_str().ok()),
            Some(api_key)
        );
    }

    #[tokio::test]
    async fn test_old_headers_removed() {
        let processor = GeminiHeadersProcessor;
        let mut headers = HyperHeaderMap::new();
        headers.insert("authorization", "Bearer sk-ant-xxx".parse().unwrap());
        headers.insert("x-api-key", "old-key".parse().unwrap());
        headers.insert("x-goog-api-key", "old-goog-key".parse().unwrap());
        headers.insert("content-type", "application/json".parse().unwrap());

        let result = processor
            .process_outgoing_request(
                "https://generativelanguage.googleapis.com",
                "new-api-key",
                "/v1beta/models/gemini-2.0-flash:generateContent",
                None,
                &headers,
                b"{}",
            )
            .await;

        assert!(result.is_ok());
        let processed = result.unwrap();

        // 验证旧 header 被移除
        assert!(processed.headers.get("authorization").is_none());
        assert!(processed.headers.get("x-api-key").is_none());

        // 验证新的 x-goog-api-key 被添加（旧值被覆盖）
        assert_eq!(
            processed
                .headers
                .get("x-goog-api-key")
                .and_then(|v| v.to_str().ok()),
            Some("new-api-key")
        );

        // 验证其他 header 保留
        assert_eq!(
            processed
                .headers
                .get("content-type")
                .and_then(|v| v.to_str().ok()),
            Some("application/json")
        );
    }

    #[tokio::test]
    async fn test_api_key_format_no_bearer() {
        let processor = GeminiHeadersProcessor;
        let headers = HyperHeaderMap::new();
        let api_key = "AIzaSyDl3-some-long-api-key-string";

        let result = processor
            .process_outgoing_request(
                "https://generativelanguage.googleapis.com",
                api_key,
                "/v1beta/models/gemini-2.0-flash:generateContent",
                None,
                &headers,
                b"{}",
            )
            .await;

        assert!(result.is_ok());
        let processed = result.unwrap();

        // 验证 API Key 没有 Bearer 前缀
        let api_key_value = processed
            .headers
            .get("x-goog-api-key")
            .and_then(|v| v.to_str().ok())
            .unwrap();
        assert!(!api_key_value.starts_with("Bearer "));
        assert_eq!(api_key_value, api_key);
    }

    #[tokio::test]
    async fn test_url_construction() {
        let processor = GeminiHeadersProcessor;
        let headers = HyperHeaderMap::new();

        let result = processor
            .process_outgoing_request(
                "https://generativelanguage.googleapis.com/",
                "test-key",
                "/v1beta/models/gemini-2.0-flash:generateContent",
                None,
                &headers,
                b"{}",
            )
            .await;

        assert!(result.is_ok());
        let processed = result.unwrap();

        // 验证 URL 正确拼接（base_url 末尾 / 被移除，path 直接拼接）
        assert_eq!(
            processed.target_url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent"
        );
    }

    #[tokio::test]
    async fn test_query_string_preserved() {
        let processor = GeminiHeadersProcessor;
        let headers = HyperHeaderMap::new();

        let result = processor
            .process_outgoing_request(
                "https://generativelanguage.googleapis.com",
                "test-key",
                "/v1beta/models/gemini-2.0-flash:generateContent",
                Some("key=value&foo=bar"),
                &headers,
                b"{}",
            )
            .await;

        assert!(result.is_ok());
        let processed = result.unwrap();

        // 验证 query string 被正确保留
        assert_eq!(
            processed.target_url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key=value&foo=bar"
        );
    }
}
