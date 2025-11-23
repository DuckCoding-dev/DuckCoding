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
