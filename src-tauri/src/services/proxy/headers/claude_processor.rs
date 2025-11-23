// Claude Code 请求处理器

use super::{ProcessedRequest, RequestProcessor};
use crate::services::session::{SessionEvent, SESSION_MANAGER};
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use hyper::HeaderMap as HyperHeaderMap;
use reqwest::header::HeaderMap as ReqwestHeaderMap;

/// Claude Code 专用请求处理器
///
/// 处理 Anthropic Claude API 的请求转换：
/// - URL 构建：使用标准拼接（无特殊逻辑）
/// - 认证方式：Bearer Token
/// - Authorization header 格式：`Bearer sk-ant-xxx`
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
                            let _ = SESSION_MANAGER.send_event(SessionEvent::NewRequest {
                                session_id: user_id.to_string(),
                                tool_id: "claude-code".to_string(),
                                timestamp,
                            });
                            (session_url, session_api_key)
                        } else {
                            // 使用全局配置并记录会话
                            let _ = SESSION_MANAGER.send_event(SessionEvent::NewRequest {
                                session_id: user_id.to_string(),
                                tool_id: "claude-code".to_string(),
                                timestamp,
                            });
                            (base_url.to_string(), api_key.to_string())
                        }
                    } else {
                        // 会话不存在，使用全局配置并记录新会话
                        let _ = SESSION_MANAGER.send_event(SessionEvent::NewRequest {
                            session_id: user_id.to_string(),
                            tool_id: "claude-code".to_string(),
                            timestamp,
                        });
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
}
