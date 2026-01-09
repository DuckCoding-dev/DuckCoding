// AMP Code 请求处理器
//
// 路由逻辑：
// 1. /api/provider/anthropic/* → Claude Profile（提取 /v1/messages）
// 2. /api/provider/openai/* → Codex Profile（提取 /v1/responses 或 /v1/chat/completions）
// 3. /api/provider/google/* → Gemini Profile（提取 /v1beta/...）
// 4. 其他 /api/* → ampcode.com（使用 AMP Access Token）
// 5. 直接 LLM 路径 → 按路径/headers/model 判断

use super::{
    ClaudeHeadersProcessor, CodexHeadersProcessor, GeminiHeadersProcessor, ProcessedRequest,
    RequestProcessor,
};
use crate::services::profile_manager::ProfileManager;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use hyper::HeaderMap as HyperHeaderMap;

#[derive(Debug)]
pub struct AmpHeadersProcessor;

#[derive(Debug, Clone, Copy, PartialEq)]
enum ApiType {
    AmpInternal,
    Claude,
    Codex,
    Gemini,
}

const TOOL_PREFIX: &str = "dc_";

impl AmpHeadersProcessor {
    fn detect_api_type(path: &str, headers: &HyperHeaderMap, body: &[u8]) -> ApiType {
        let path_lower = path.to_lowercase();

        // 1. /api/provider/{provider}/* → LLM 端点
        if path_lower.starts_with("/api/provider/anthropic") {
            return ApiType::Claude;
        }
        if path_lower.starts_with("/api/provider/openai") {
            return ApiType::Codex;
        }
        if path_lower.starts_with("/api/provider/google") {
            return ApiType::Gemini;
        }

        // 2. 其他 /api/* → ampcode.com
        if path_lower.starts_with("/api/") {
            return ApiType::AmpInternal;
        }

        // 3. 直接 LLM 路径
        if path_lower.contains("/messages") && !path_lower.contains("/chat/completions") {
            return ApiType::Claude;
        }
        if path_lower.contains("/chat/completions")
            || path_lower.contains("/responses")
            || path_lower.ends_with("/completions")
        {
            return ApiType::Codex;
        }
        if path_lower.contains("/v1beta")
            || path_lower.contains(":generatecontent")
            || path_lower.contains(":streamgeneratecontent")
        {
            return ApiType::Gemini;
        }

        // 4. 按 headers
        if headers.contains_key("anthropic-version") {
            return ApiType::Claude;
        }

        // 5. 按 body.model
        if let Some(api_type) = Self::detect_by_model(body) {
            return api_type;
        }

        ApiType::Claude
    }

    fn detect_by_model(body: &[u8]) -> Option<ApiType> {
        if body.is_empty() {
            return None;
        }
        let json: serde_json::Value = serde_json::from_slice(body).ok()?;
        let model = json.get("model")?.as_str()?.to_lowercase();

        if model.contains("gemini") {
            Some(ApiType::Gemini)
        } else if model.contains("claude") {
            Some(ApiType::Claude)
        } else if model.contains("gpt") {
            Some(ApiType::Codex)
        } else {
            None
        }
    }

    fn extract_model_name(path: &str, body: &[u8]) -> String {
        // 1. 从路径提取：/v1beta/models/{model}:xxx
        if let Some(start) = path.find("/models/") {
            let after = &path[start + 8..];
            if let Some(end) = after.find(':') {
                return after[..end].to_string();
            }
            if let Some(end) = after.find('/') {
                return after[..end].to_string();
            }
            return after.to_string();
        }

        // 2. 从请求体提取
        if !body.is_empty() {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(body) {
                if let Some(model) = json.get("model").and_then(|m| m.as_str()) {
                    return model.to_string();
                }
            }
        }

        "gemini-2.0-flash".to_string()
    }

    /// 提取 LLM API 路径：/api/provider/xxx/v1/... → /v1/...
    /// Gemini 特殊处理：/v1beta1/publishers/google/models/xxx → /v1beta/models/xxx
    fn extract_llm_path(path: &str) -> String {
        // Gemini 路径转换：v1beta1/publishers/google/models/xxx → v1beta/models/xxx
        if let Some(pos) = path.find("/v1beta1/publishers/google/models/") {
            let model_part = &path[pos + "/v1beta1/publishers/google/models/".len()..];
            return format!("/v1beta/models/{}", model_part);
        }

        // 标准路径提取
        if let Some(pos) = path.find("/v1beta") {
            return path[pos..].to_string();
        }
        if let Some(pos) = path.find("/v1") {
            return path[pos..].to_string();
        }

        path.to_string()
    }

    fn get_user_agent(api_type: ApiType, path: &str, body: &[u8]) -> String {
        match api_type {
            ApiType::Claude => "claude-cli/2.0.72 (external, cli)".to_string(),
            ApiType::Codex => {
                "codex_cli_rs/0.77.0 (Mac OS 15.7.2; arm64) Apple_Terminal/455.1".to_string()
            }
            ApiType::Gemini => {
                let model = Self::extract_model_name(path, body);
                format!("GeminiCLI/0.22.5/{} (darwin; arm64)", model)
            }
            ApiType::AmpInternal => unreachable!(),
        }
    }

        fn add_tool_prefix(body: &[u8]) -> Vec<u8> {
        if body.is_empty() {
            return body.to_vec();
        }

        let Ok(mut json) = serde_json::from_slice::<serde_json::Value>(body) else {
            return body.to_vec();
        };

        if let Some(tools) = json.get_mut("tools").and_then(|t| t.as_array_mut()) {
            for tool in tools.iter_mut() {
                if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
                    if !name.starts_with(TOOL_PREFIX) {
                        tool["name"] =
                            serde_json::Value::String(format!("{}{}", TOOL_PREFIX, name));
                    }
                }
            }
        }

        serde_json::to_vec(&json).unwrap_or_else(|_| body.to_vec())
    }
    
    async fn forward_to_amp(
        path: &str,
        query: Option<&str>,
        headers: &HyperHeaderMap,
        body: &[u8],
    ) -> Result<ProcessedRequest> {
        let proxy_mgr = crate::services::proxy_config_manager::ProxyConfigManager::new()
            .map_err(|e| anyhow!("ProxyConfigManager 初始化失败: {}", e))?;

        let config = proxy_mgr
            .get_config("amp-code")
            .map_err(|e| anyhow!("读取配置失败: {}", e))?
            .ok_or_else(|| anyhow!("AMP Code 代理未配置"))?;

        let token = config
            .real_api_key
            .ok_or_else(|| anyhow!("AMP Code Access Token 未配置"))?;

        let base_url = config
            .real_base_url
            .unwrap_or_else(|| "https://ampcode.com".to_string());

        let target_url = match query {
            Some(q) => format!("{}{}?{}", base_url, path, q),
            None => format!("{}{}", base_url, path),
        };

        tracing::info!("AMP Code → ampcode.com: {}", target_url);

        let mut new_headers = headers.clone();
        new_headers.remove(hyper::header::AUTHORIZATION);
        let x_api_key = hyper::header::HeaderName::from_static("x-api-key");
        new_headers.remove(&x_api_key);
        new_headers.insert(
            hyper::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().unwrap(),
        );
        new_headers.insert(x_api_key, token.parse().unwrap());

        Ok(ProcessedRequest {
            target_url,
            headers: new_headers,
            body: body.to_vec().into(),
        })
    }
}

#[async_trait]
impl RequestProcessor for AmpHeadersProcessor {
    fn tool_id(&self) -> &str {
        "amp-code"
    }

    async fn process_outgoing_request(
        &self,
        _base_url: &str,
        _api_key: &str,
        path: &str,
        query: Option<&str>,
        original_headers: &HyperHeaderMap,
        body: &[u8],
    ) -> Result<ProcessedRequest> {
        let api_type = Self::detect_api_type(path, original_headers, body);
        tracing::debug!("AMP Code 路由: path={}, type={:?}", path, api_type);

        if api_type == ApiType::AmpInternal {
            return Self::forward_to_amp(path, query, original_headers, body).await;
        }

        // LLM 请求 → 用户配置的 Profile
        let profile_mgr =
            ProfileManager::new().map_err(|e| anyhow!("ProfileManager 初始化失败: {}", e))?;

        let (claude, codex, gemini) = profile_mgr
            .resolve_amp_selection()
            .map_err(|e| anyhow!("Profile 解析失败: {}", e))?;

        let llm_path = Self::extract_llm_path(path);

        match api_type {
            ApiType::Claude => {
                let p = claude.ok_or_else(|| anyhow!("未配置 Claude Profile"))?;
                tracing::info!("AMP Code → Claude: {}{}", p.base_url, llm_path);
                let prefixed_body = Self::add_tool_prefix(body);

                let mut result = ClaudeHeadersProcessor
                    .process_outgoing_request(
                        &p.base_url,
                        &p.api_key,
                        &llm_path,
                        query,
                        original_headers,
                        &prefixed_body,
                    )
                    .await?;

                let amp_headers: Vec<_> = result
                    .headers
                    .keys()
                    .filter(|k| k.as_str().starts_with("x-amp-"))
                    .cloned()
                    .collect();
                for key in amp_headers {
                    result.headers.remove(&key);
                }

                result.headers.remove("content-length");
                result.headers.remove("transfer-encoding");

                result.headers.insert(
                    "user-agent",
                    Self::get_user_agent(api_type, path, body).parse().unwrap(),
                );
                result.headers.insert("x-app", "cli".parse().unwrap());

                if !result.target_url.contains("beta=true") {
                    if result.target_url.contains('?') {
                        result.target_url.push_str("&beta=true");
                    } else {
                        result.target_url.push_str("?beta=true");
                    }
                }

                Ok(result)
            }
            ApiType::Codex => {
                let p = codex.ok_or_else(|| anyhow!("未配置 Codex Profile"))?;
                tracing::info!("AMP Code → Codex: {}{}", p.base_url, llm_path);
                let mut result = CodexHeadersProcessor
                    .process_outgoing_request(
                        &p.base_url,
                        &p.api_key,
                        &llm_path,
                        query,
                        original_headers,
                        body,
                    )
                    .await?;
                result.headers.insert(
                    "user-agent",
                    Self::get_user_agent(api_type, path, body).parse().unwrap(),
                );
                Ok(result)
            }
            ApiType::Gemini => {
                let p = gemini.ok_or_else(|| anyhow!("未配置 Gemini Profile"))?;
                tracing::info!("AMP Code → Gemini: {}{}", p.base_url, llm_path);
                let mut result = GeminiHeadersProcessor
                    .process_outgoing_request(
                        &p.base_url,
                        &p.api_key,
                        &llm_path,
                        query,
                        original_headers,
                        body,
                    )
                    .await?;
                result.headers.insert(
                    "user-agent",
                    Self::get_user_agent(api_type, path, body).parse().unwrap(),
                );
                Ok(result)
            }
            ApiType::AmpInternal => unreachable!(),
        }
    }
}
