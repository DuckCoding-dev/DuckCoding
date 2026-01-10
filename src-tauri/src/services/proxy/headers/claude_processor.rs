// Claude Code 请求处理器

use super::{ProcessedRequest, RequestProcessor};
use crate::services::session::{ProxySession, SessionEvent, SESSION_MANAGER};
use crate::services::token_stats::TokenStatsManager;
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use hyper::{HeaderMap as HyperHeaderMap, StatusCode};
use reqwest::header::HeaderMap as ReqwestHeaderMap;

/// Claude Code 专用请求处理器
///
/// 处理 Anthropic Claude API 的请求转换：
/// - URL 构建：使用标准拼接（无特殊逻辑）
/// - 认证方式：Bearer Token
/// - Authorization header 格式：`Bearer sk-ant-xxx`
#[derive(Debug)]
pub struct ClaudeHeadersProcessor;

#[async_trait]
impl RequestProcessor for ClaudeHeadersProcessor {
    fn tool_id(&self) -> &str {
        "claude-code"
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
        // 0. 查询会话配置并决定使用哪个 URL 和 API Key
        let (final_base_url, final_api_key) = if !body.is_empty() {
            // 尝试解析请求体 JSON 提取 user_id
            if let Ok(json_body) = serde_json::from_slice::<serde_json::Value>(body) {
                if let Some(user_id) = json_body["metadata"]["user_id"].as_str() {
                    let timestamp = chrono::Utc::now().timestamp();

                    // 查询会话配置
                    if let Ok(Some((config_name, session_url, session_api_key))) =
                        SESSION_MANAGER.get_session_config(user_id)
                    {
                        // 如果是自定义配置且有 URL 和 API Key，使用数据库的配置
                        if config_name == "custom"
                            && !session_url.is_empty()
                            && !session_api_key.is_empty()
                        {
                            // 记录会话事件（使用自定义配置）
                            if let Err(e) = SESSION_MANAGER.send_event(SessionEvent::NewRequest {
                                session_id: user_id.to_string(),
                                tool_id: "claude-code".to_string(),
                                timestamp,
                            }) {
                                tracing::warn!("Session 事件发送失败: {}", e);
                            }
                            (session_url, session_api_key)
                        } else {
                            // 使用全局配置并记录会话
                            if let Err(e) = SESSION_MANAGER.send_event(SessionEvent::NewRequest {
                                session_id: user_id.to_string(),
                                tool_id: "claude-code".to_string(),
                                timestamp,
                            }) {
                                tracing::warn!("Session 事件发送失败: {}", e);
                            }
                            (base_url.to_string(), api_key.to_string())
                        }
                    } else {
                        // 会话不存在，使用全局配置并记录新会话
                        if let Err(e) = SESSION_MANAGER.send_event(SessionEvent::NewRequest {
                            session_id: user_id.to_string(),
                            tool_id: "claude-code".to_string(),
                            timestamp,
                        }) {
                            tracing::warn!("Session 事件发送失败: {}", e);
                        }
                        (base_url.to_string(), api_key.to_string())
                    }
                } else {
                    // 没有 user_id，使用全局配置
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

        // 1. 构建目标 URL（标准拼接）
        let base = final_base_url.trim_end_matches('/');
        let query_str = query.map(|q| format!("?{q}")).unwrap_or_default();
        let target_url = format!("{base}{path}{query_str}");

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

        // 3. 添加真实的 API Key
        headers.insert(
            "authorization",
            format!("Bearer {final_api_key}")
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid authorization header: {e}"))?,
        );

        // 4. 返回处理后的请求
        Ok(ProcessedRequest {
            target_url,
            headers,
            body: Bytes::copy_from_slice(body),
        })
    }

    // Claude Code 不需要特殊的响应处理
    // 使用默认实现即可

    /// 提取模型名称
    fn extract_model(&self, request_body: &[u8]) -> Option<String> {
        if request_body.is_empty() {
            return None;
        }

        // 尝试解析请求体 JSON
        if let Ok(json_body) = serde_json::from_slice::<serde_json::Value>(request_body) {
            // Claude API 的模型字段在顶层
            json_body.get("model")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        } else {
            None
        }
    }

    /// Claude Code 的请求日志记录实现
    ///
    /// 从请求体中提取会话 ID（metadata.user_id），根据响应类型解析 Token 统计
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
        // 1. 提取会话 ID（从 metadata.user_id 的 _session_ 后部分）
        let session_id = if !request_body.is_empty() {
            if let Ok(json_body) = serde_json::from_slice::<serde_json::Value>(request_body) {
                if let Some(user_id) = json_body["metadata"]["user_id"].as_str() {
                    // 使用 ProxySession::extract_display_id 提取 _session_ 后的 UUID
                    ProxySession::extract_display_id(user_id)
                        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
                } else {
                    uuid::Uuid::new_v4().to_string()
                }
            } else {
                uuid::Uuid::new_v4().to_string()
            }
        } else {
            uuid::Uuid::new_v4().to_string()
        };

        // 2. 获取 pricing_template_id（优先级：会话配置 > 代理配置 > None）
        // TODO: Phase 3.4 后续需要从 get_session_config 返回会话的 pricing_template_id
        let pricing_template_id: Option<String> =
            proxy_pricing_template_id.map(|s| s.to_string());

        // 3. 检查响应状态
        let status_code =
            StatusCode::from_u16(response_status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        if !status_code.is_server_error() && !status_code.is_client_error() {
            // 成功响应，记录 Token 统计
            let manager = TokenStatsManager::get();
            let response_data = if is_sse {
                // SSE 流式响应：解析所有 data 块
                let body_str = String::from_utf8_lossy(response_body);
                let data_lines: Vec<String> = body_str
                    .lines()
                    .filter(|line| line.starts_with("data: "))
                    .map(|line| line.trim_start_matches("data: ").to_string())
                    .collect();

                crate::services::token_stats::manager::ResponseData::Sse(data_lines)
            } else {
                // JSON 响应
                let json: serde_json::Value = serde_json::from_slice(response_body)?;
                crate::services::token_stats::manager::ResponseData::Json(json)
            };

            match manager
                .log_request(
                    self.tool_id(),
                    &session_id,
                    config_name,
                    client_ip,
                    request_body,
                    response_data,
                    response_time_ms,
                    pricing_template_id.clone(),
                    None, // input_price 由 TokenStatsManager 内部计算
                    None, // output_price 由 TokenStatsManager 内部计算
                    None, // cache_write_price 由 TokenStatsManager 内部计算
                    None, // cache_read_price 由 TokenStatsManager 内部计算
                    0.0,  // total_cost 由 TokenStatsManager 内部计算
                )
                .await
            {
                Ok(_) => {
                    tracing::info!(
                        tool_id = %self.tool_id(),
                        session_id = %session_id,
                        "Token 统计记录成功"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        tool_id = %self.tool_id(),
                        session_id = %session_id,
                        error = ?e,
                        "Token 统计记录失败"
                    );

                    // 记录解析失败
                    let error_detail = format!("Token parsing failed: {}", e);
                    let response_type = if is_sse { "sse" } else { "json" };
                    let _ = manager
                        .log_failed_request(
                            self.tool_id(),
                            &session_id,
                            config_name,
                            client_ip,
                            request_body,
                            "parse_error",
                            &error_detail,
                            response_type,
                            response_time_ms,
                        )
                        .await;
                }
            }
        } else {
            // 失败响应，记录错误
            let manager = TokenStatsManager::get();
            let error_detail = format!(
                "HTTP {}: {}",
                response_status,
                status_code.canonical_reason().unwrap_or("Unknown")
            );
            let response_type = if is_sse { "sse" } else { "json" };

            let _ = manager
                .log_failed_request(
                    self.tool_id(),
                    &session_id,
                    config_name,
                    client_ip,
                    request_body,
                    "upstream_error",
                    &error_detail,
                    response_type,
                    response_time_ms,
                )
                .await;
        }

        Ok(())
    }
}
