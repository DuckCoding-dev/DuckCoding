// Amp Code 请求处理器
//
// Amp CLI 的请求本质上是 Anthropic/OpenAI/Gemini 兼容的 API 调用，
// 此处理器根据请求路径/headers 判断请求类型，然后委托给对应的 processor 处理。
//
// 核心逻辑：
// 1. 检测请求类型（Claude/Codex/Gemini）
// 2. 从 ProfileManager::resolve_amp_selection() 获取用户选择的 profile
// 3. 委托给对应的 processor 完成实际的请求转换

use super::{
    ClaudeHeadersProcessor, CodexHeadersProcessor, GeminiHeadersProcessor, ProcessedRequest,
    RequestProcessor,
};
use crate::services::profile_manager::ProfileManager;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use hyper::HeaderMap as HyperHeaderMap;

/// Amp Code 请求处理器
///
/// 作为路由层，根据请求特征判断应该使用哪个上游 API，
/// 然后委托给对应的 processor 完成实际转换。
///
/// # 路由规则（按优先级）
/// 1. `/v1/messages` → Claude (Anthropic API)
/// 2. `/v1/chat/completions`, `/v1/responses`, `/v1/completions` → Codex (OpenAI API)
/// 3. `/v1beta/`, `:generateContent`, `:streamGenerateContent` → Gemini API
/// 4. 兜底：检查 headers 中的 `anthropic-version` 或请求体中的 model 字段
#[derive(Debug)]
pub struct AmpHeadersProcessor;

/// 检测请求类型
#[derive(Debug, Clone, Copy, PartialEq)]
enum DetectedApiType {
    Claude,
    Codex,
    Gemini,
}

impl AmpHeadersProcessor {
    /// 根据请求路径和 headers 检测 API 类型
    fn detect_api_type(path: &str, headers: &HyperHeaderMap, body: &[u8]) -> DetectedApiType {
        // 1. 优先按路径判断（最稳定）
        let path_lower = path.to_lowercase();

        // Anthropic Claude API
        if path_lower.contains("/messages") && !path_lower.contains("/chat/completions") {
            return DetectedApiType::Claude;
        }

        // OpenAI API
        if path_lower.contains("/chat/completions")
            || path_lower.contains("/responses")
            || path_lower.ends_with("/completions")
        {
            return DetectedApiType::Codex;
        }

        // Gemini API
        if path_lower.contains("/v1beta")
            || path_lower.contains(":generatecontent")
            || path_lower.contains(":streamgeneratecontent")
        {
            return DetectedApiType::Gemini;
        }

        // 2. 其次按 headers 判断
        if headers.contains_key("anthropic-version") {
            return DetectedApiType::Claude;
        }

        // 3. 最后按 body 判断（解析 JSON 中的 model 字段）
        if !body.is_empty() {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(body) {
                if let Some(model) = json.get("model").and_then(|m| m.as_str()) {
                    let model_lower = model.to_lowercase();

                    // Gemini models
                    if model_lower.contains("gemini") {
                        return DetectedApiType::Gemini;
                    }

                    // Claude models
                    if model_lower.contains("claude") {
                        return DetectedApiType::Claude;
                    }

                    // OpenAI models (gpt, o1, etc.)
                    if model_lower.contains("gpt")
                        || model_lower.starts_with("o1")
                        || model_lower.starts_with("o3")
                    {
                        return DetectedApiType::Codex;
                    }
                }
            }
        }

        // 默认使用 Claude（因为 Amp 主要面向 Claude API）
        DetectedApiType::Claude
    }
}

#[async_trait]
impl RequestProcessor for AmpHeadersProcessor {
    fn tool_id(&self) -> &str {
        "amp-code"
    }

    async fn process_outgoing_request(
        &self,
        _base_url: &str, // 忽略传入的 base_url，使用 resolve_amp_selection 获取
        _api_key: &str,  // 忽略传入的 api_key，使用 resolve_amp_selection 获取
        path: &str,
        query: Option<&str>,
        original_headers: &HyperHeaderMap,
        body: &[u8],
    ) -> Result<ProcessedRequest> {
        // 1. 检测请求类型
        let api_type = Self::detect_api_type(path, original_headers, body);
        tracing::debug!("Amp Code 请求路由: path={}, api_type={:?}", path, api_type);

        // 2. 获取 ProfileManager 并解析 AMP 选择
        let profile_manager =
            ProfileManager::new().map_err(|e| anyhow!("无法初始化 ProfileManager: {}", e))?;

        let (claude_profile, codex_profile, gemini_profile) = profile_manager
            .resolve_amp_selection()
            .map_err(|e| anyhow!("无法解析 AMP Code Profile 选择: {}", e))?;

        // 3. 根据检测到的 API 类型选择对应的 profile 和 processor
        match api_type {
            DetectedApiType::Claude => {
                let profile = claude_profile.ok_or_else(|| {
                    anyhow!("AMP Code 未配置 Claude Profile，请先在 Profile 管理页面选择")
                })?;

                tracing::info!("Amp Code 请求转发到 Claude: base_url={}", profile.base_url);

                // 委托给 Claude processor
                ClaudeHeadersProcessor
                    .process_outgoing_request(
                        &profile.base_url,
                        &profile.api_key,
                        path,
                        query,
                        original_headers,
                        body,
                    )
                    .await
            }
            DetectedApiType::Codex => {
                let profile = codex_profile.ok_or_else(|| {
                    anyhow!("AMP Code 未配置 Codex Profile，请先在 Profile 管理页面选择")
                })?;

                tracing::info!("Amp Code 请求转发到 Codex: base_url={}", profile.base_url);

                // 委托给 Codex processor
                CodexHeadersProcessor
                    .process_outgoing_request(
                        &profile.base_url,
                        &profile.api_key,
                        path,
                        query,
                        original_headers,
                        body,
                    )
                    .await
            }
            DetectedApiType::Gemini => {
                let profile = gemini_profile.ok_or_else(|| {
                    anyhow!("AMP Code 未配置 Gemini Profile，请先在 Profile 管理页面选择")
                })?;

                tracing::info!("Amp Code 请求转发到 Gemini: base_url={}", profile.base_url);

                // 委托给 Gemini processor
                GeminiHeadersProcessor
                    .process_outgoing_request(
                        &profile.base_url,
                        &profile.api_key,
                        path,
                        query,
                        original_headers,
                        body,
                    )
                    .await
            }
        }
    }
}
