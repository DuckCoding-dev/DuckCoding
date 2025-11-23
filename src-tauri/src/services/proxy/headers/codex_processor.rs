// Codex 请求处理器

use super::{ProcessedRequest, RequestProcessor};
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use hyper::HeaderMap as HyperHeaderMap;
use reqwest::header::HeaderMap as ReqwestHeaderMap;

/// Codex 专用请求处理器
///
/// 处理 OpenAI API 的请求转换：
/// - URL 构建：特殊的 /v1 路径调整逻辑
///   - Codex 配置要求 base_url 包含 /v1（如 https://api.openai.com/v1）
///   - 但 Codex 发送请求时也会带上 /v1 前缀（如 /v1/chat/completions）
///   - 为避免重复，当 base_url 以 /v1 结尾且 path 以 /v1 开头时，去掉 path 中的 /v1
/// - 认证方式：Bearer Token
/// - Authorization header 格式：`Bearer sk-xxx`
///
/// # TODO
/// 根据实际需求添加：
/// - OpenAI-Organization header 处理
/// - OpenAI-Project header 处理
pub struct CodexHeadersProcessor;

#[async_trait]
impl RequestProcessor for CodexHeadersProcessor {
    fn tool_id(&self) -> &str {
        "codex"
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
        // 1. 构建目标 URL（Codex 特殊逻辑：避免 /v1 路径重复）
        let base = base_url.trim_end_matches('/');

        // Codex 特殊逻辑：避免 /v1 路径重复
        let adjusted_path = if base.ends_with("/v1") && path.starts_with("/v1") {
            &path[3..] // 去掉 "/v1"
        } else {
            path
        };

        let query_str = query.map(|q| format!("?{q}")).unwrap_or_default();
        let target_url = format!("{base}{adjusted_path}{query_str}");

        // 2. 处理 headers（复制非认证 headers）
        let mut headers = ReqwestHeaderMap::new();
        for (name, value) in original_headers.iter() {
            let name_str = name.as_str();
            // 跳过认证相关和 Host headers
            if name_str.eq_ignore_ascii_case("host")
                || name_str.eq_ignore_ascii_case("authorization")
                || name_str.eq_ignore_ascii_case("x-api-key")
            {
                continue;
            }
            headers.insert(name.clone(), value.clone());
        }

        // 3. 添加真实的 OpenAI API Key（Bearer Token 格式）
        headers.insert(
            "authorization",
            format!("Bearer {api_key}")
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid authorization header: {e}"))?,
        );

        // TODO: 根据需要添加其他 OpenAI 特定的 headers
        // 例如：
        // if let Some(org_id) = get_organization_id() {
        //     headers.insert("OpenAI-Organization", org_id.parse()?);
        // }

        // 4. 返回处理后的请求
        Ok(ProcessedRequest {
            target_url,
            headers,
            body: Bytes::copy_from_slice(body),
        })
    }

    // Codex 当前不需要特殊的响应处理
    // 如果未来需要（例如处理速率限制信息），可以在此实现
}
