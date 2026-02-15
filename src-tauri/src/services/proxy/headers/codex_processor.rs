// Codex 请求处理器

use super::{ProcessedRequest, RequestProcessor};
use crate::services::session::{SessionEvent, SESSION_MANAGER};
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
#[derive(Debug)]
pub struct CodexHeadersProcessor;

impl CodexHeadersProcessor {
    pub async fn process_outgoing_request_for(
        &self,
        caller_tool_id: &str,
        base_url: &str,
        api_key: &str,
        path: &str,
        query: Option<&str>,
        original_headers: &HyperHeaderMap,
        body: &[u8],
    ) -> Result<ProcessedRequest> {
        // 0. 查询会话配置并决定使用哪个 URL 和 API Key
        let (final_base_url, final_api_key) = if !body.is_empty() {
            // 尝试解析请求体 JSON 提取 prompt_cache_key
            if let Ok(json_body) = serde_json::from_slice::<serde_json::Value>(body) {
                if let Some(session_id) = json_body["prompt_cache_key"].as_str() {
                    let timestamp = chrono::Utc::now().timestamp();

                    // 查询会话配置
                    if let Ok(Some((
                        config_name,
                        _custom_profile_name,
                        session_url,
                        session_api_key,
                        _session_pricing_template_id,
                    ))) = SESSION_MANAGER.get_session_config(session_id)
                    {
                        // 如果是自定义配置且有 URL 和 API Key，使用数据库的配置
                        if config_name == "custom"
                            && !session_url.is_empty()
                            && !session_api_key.is_empty()
                        {
                            // 记录会话事件（使用自定义配置）
                            if let Err(e) = SESSION_MANAGER.send_event(SessionEvent::NewRequest {
                                session_id: session_id.to_string(),
                                tool_id: caller_tool_id.to_string(),
                                timestamp,
                            }) {
                                tracing::warn!("Session 事件发送失败: {}", e);
                            }
                            (session_url, session_api_key)
                        } else {
                            // 使用全局配置并记录会话
                            if let Err(e) = SESSION_MANAGER.send_event(SessionEvent::NewRequest {
                                session_id: session_id.to_string(),
                                tool_id: caller_tool_id.to_string(),
                                timestamp,
                            }) {
                                tracing::warn!("Session 事件发送失败: {}", e);
                            }
                            (base_url.to_string(), api_key.to_string())
                        }
                    } else {
                        // 会话不存在，使用全局配置并记录新会话
                        if let Err(e) = SESSION_MANAGER.send_event(SessionEvent::NewRequest {
                            session_id: session_id.to_string(),
                            tool_id: caller_tool_id.to_string(),
                            timestamp,
                        }) {
                            tracing::warn!("Session 事件发送失败: {}", e);
                        }
                        (base_url.to_string(), api_key.to_string())
                    }
                } else {
                    // 没有 prompt_cache_key，使用全局配置
                    (base_url.to_string(), api_key.to_string())
                }
            } else {
                // JSON 解析失败，使用全局配置
                (base_url.to_string(), api_key.to_string())
            }
        } else {
            // 空 body，使用全局配置
            (base_url.to_string(), api_key.to_string())
        };

        // 1. 构建目标 URL（Codex 特殊逻辑：避免 /v1 路径重复）
        let base = final_base_url.trim_end_matches('/');

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
            format!("Bearer {final_api_key}")
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
}

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
        self.process_outgoing_request_for("codex", base_url, api_key, path, query, original_headers, body).await
    }

    // Codex 当前不需要特殊的响应处理
    // 如果未来需要（例如处理速率限制信息），可以在此实现

    /// 提取模型名称
    fn extract_model(&self, request_body: &[u8]) -> Option<String> {
        if request_body.is_empty() {
            return None;
        }

        // 尝试解析请求体 JSON
        if let Ok(json_body) = serde_json::from_slice::<serde_json::Value>(request_body) {
            // OpenAI API 的模型字段在顶层
            json_body
                .get("model")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        } else {
            None
        }
    }

    /// Codex 的请求日志记录实现
    ///
    /// 使用统一的日志记录架构，自动处理所有错误场景
    async fn record_request_log(
        &self,
        client_ip: &str,
        config_name: &str,
        proxy_pricing_template_id: Option<&str>,
        request_body: &[u8],
        response_status: u16,
        response_body: &[u8],
        is_sse: bool,
        response_time_ms: Option<i64>,
    ) -> Result<()> {
        use crate::services::proxy::log_recorder::{
            LogRecorder, RequestLogContext, ResponseParser,
        };

        // 1. 创建请求上下文（一次性提取所有信息）
        let context = RequestLogContext::from_request(
            self.tool_id(),
            config_name,
            client_ip,
            proxy_pricing_template_id,
            request_body,
            response_time_ms,
        );

        // 2. 解析响应
        let parsed = ResponseParser::parse(response_body, response_status, is_sse);

        // 3. 记录日志（自动处理成功/失败/解析错误）
        LogRecorder::record(&context, response_status, parsed).await?;

        Ok(())
    }
}
