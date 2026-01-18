use anyhow::{Context, Result};
use serde_json::Value;

/// Token提取器统一接口
pub trait TokenExtractor: Send + Sync {
    /// 从请求体中提取模型名称
    fn extract_model_from_request(&self, body: &[u8]) -> Result<String>;

    /// 从SSE数据块中提取Token信息
    fn extract_from_sse_chunk(&self, chunk: &str) -> Result<Option<SseTokenData>>;

    /// 从JSON响应中提取Token信息
    fn extract_from_json(&self, json: &Value) -> Result<ResponseTokenInfo>;
}

/// SSE流式数据中的Token信息
#[derive(Debug, Clone, Default)]
pub struct SseTokenData {
    /// message_start块数据
    pub message_start: Option<MessageStartData>,
    /// message_delta块数据（end_turn）
    pub message_delta: Option<MessageDeltaData>,
}

/// message_start块数据
#[derive(Debug, Clone)]
pub struct MessageStartData {
    pub model: String,
    pub message_id: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_tokens: i64,
    pub cache_read_tokens: i64,
}

/// message_delta块数据（end_turn）
#[derive(Debug, Clone)]
pub struct MessageDeltaData {
    pub input_tokens: Option<i64>, // Codex 的 response.completed 包含 input_tokens
    pub cache_creation_tokens: i64,
    pub cache_read_tokens: i64,
    pub output_tokens: i64,
}

/// 响应Token信息（完整）
#[derive(Debug, Clone)]
pub struct ResponseTokenInfo {
    pub model: String,
    pub message_id: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_tokens: i64,
    pub cache_read_tokens: i64,
    pub reasoning_tokens: i64,
}

impl ResponseTokenInfo {
    /// 从SSE数据合并得到完整信息
    ///
    /// 合并规则：
    /// - model, message_id, input_tokens: 始终使用 message_start 的值
    /// - output_tokens, cache_*: 优先使用 message_delta 的值，回退到 message_start
    pub fn from_sse_data(start: MessageStartData, delta: Option<MessageDeltaData>) -> Self {
        let (input, cache_creation, cache_read, output) = if let Some(d) = delta {
            // 优先使用 delta 的值（最终统计）
            (
                d.input_tokens.unwrap_or(start.input_tokens), // Codex 的 input_tokens 在 delta 中
                d.cache_creation_tokens,
                d.cache_read_tokens,
                d.output_tokens,
            )
        } else {
            // 回退到 start 的值（初始统计）
            (
                start.input_tokens,
                start.cache_creation_tokens,
                start.cache_read_tokens,
                start.output_tokens,
            )
        };

        Self {
            model: start.model,
            message_id: start.message_id,
            input_tokens: input,
            output_tokens: output,
            cache_creation_tokens: cache_creation,
            cache_read_tokens: cache_read,
            reasoning_tokens: 0, // Claude 不使用 reasoning tokens
        }
    }
}

/// Claude Code工具的Token提取器
pub struct ClaudeTokenExtractor;

impl TokenExtractor for ClaudeTokenExtractor {
    fn extract_model_from_request(&self, body: &[u8]) -> Result<String> {
        let json: Value =
            serde_json::from_slice(body).context("Failed to parse request body as JSON")?;

        json.get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .context("Missing 'model' field in request body")
    }

    fn extract_from_sse_chunk(&self, chunk: &str) -> Result<Option<SseTokenData>> {
        // SSE格式: data: {...} 或直接 {...}（已去掉前缀）
        let data_line = chunk.trim();

        // 跳过空行
        if data_line.is_empty() {
            return Ok(None);
        }

        // 兼容处理：去掉 "data: " 前缀（如果存在）
        let json_str = if let Some(stripped) = data_line.strip_prefix("data: ") {
            stripped
        } else {
            data_line
        };

        // 跳过 [DONE] 标记
        if json_str.trim() == "[DONE]" {
            return Ok(None);
        }

        let json: Value =
            serde_json::from_str(json_str).context("Failed to parse SSE chunk as JSON")?;

        let event_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");

        tracing::debug!(event_type = event_type, "解析 SSE 事件");

        let mut result = SseTokenData::default();

        match event_type {
            "message_start" => {
                if let Some(message) = json.get("message") {
                    let model = message
                        .get("model")
                        .and_then(|v| v.as_str())
                        .context("Missing model in message_start")?
                        .to_string();

                    let message_id = message
                        .get("id")
                        .and_then(|v| v.as_str())
                        .context("Missing id in message_start")?
                        .to_string();

                    let usage = message
                        .get("usage")
                        .context("Missing usage in message_start")?;

                    let input_tokens = usage
                        .get("input_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);

                    let output_tokens = usage
                        .get("output_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);

                    // 提取缓存创建 token：优先读取扁平字段，回退到嵌套对象
                    let cache_creation_tokens = usage
                        .get("cache_creation_input_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or_else(|| {
                            if let Some(cache_obj) = usage.get("cache_creation") {
                                let ephemeral_5m = cache_obj
                                    .get("ephemeral_5m_input_tokens")
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0);
                                let ephemeral_1h = cache_obj
                                    .get("ephemeral_1h_input_tokens")
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0);
                                ephemeral_5m + ephemeral_1h
                            } else {
                                0
                            }
                        });

                    let cache_read_tokens = usage
                        .get("cache_read_input_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);

                    result.message_start = Some(MessageStartData {
                        model,
                        message_id,
                        input_tokens,
                        output_tokens,
                        cache_creation_tokens,
                        cache_read_tokens,
                    });
                }
            }
            "message_delta" => {
                tracing::info!("检测到 message_delta 事件");

                // message_delta 事件包含最终的usage统计
                // 条件：必须有 usage 字段（无论是否有 stop_reason）
                if let Some(usage) = json.get("usage") {
                    tracing::info!("message_delta 包含 usage 字段");

                    // 提取缓存创建 token：优先读取扁平字段，回退到嵌套对象
                    let cache_creation = usage
                        .get("cache_creation_input_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or_else(|| {
                            if let Some(cache_obj) = usage.get("cache_creation") {
                                let ephemeral_5m = cache_obj
                                    .get("ephemeral_5m_input_tokens")
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0);
                                let ephemeral_1h = cache_obj
                                    .get("ephemeral_1h_input_tokens")
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0);
                                ephemeral_5m + ephemeral_1h
                            } else {
                                0
                            }
                        });

                    let cache_read = usage
                        .get("cache_read_input_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);

                    let output_tokens = usage
                        .get("output_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);

                    tracing::info!(
                        output_tokens = output_tokens,
                        cache_creation = cache_creation,
                        cache_read = cache_read,
                        "message_delta 提取成功"
                    );

                    result.message_delta = Some(MessageDeltaData {
                        input_tokens: None, // Claude 的 input_tokens 在 message_start 中
                        cache_creation_tokens: cache_creation,
                        cache_read_tokens: cache_read,
                        output_tokens,
                    });
                } else {
                    tracing::warn!("message_delta 事件缺少 usage 字段");
                }
            }
            _ => {}
        }

        Ok(
            if result.message_start.is_some() || result.message_delta.is_some() {
                Some(result)
            } else {
                None
            },
        )
    }

    fn extract_from_json(&self, json: &Value) -> Result<ResponseTokenInfo> {
        let model = json
            .get("model")
            .and_then(|v| v.as_str())
            .context("Missing model field")?
            .to_string();

        let message_id = json
            .get("id")
            .and_then(|v| v.as_str())
            .context("Missing id field")?
            .to_string();

        let usage = json.get("usage").context("Missing usage field")?;

        let input_tokens = usage
            .get("input_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let output_tokens = usage
            .get("output_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        // 提取 cache_creation_input_tokens：
        // 优先读取扁平字段，如果不存在则尝试从嵌套对象聚合
        let cache_creation = usage
            .get("cache_creation_input_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or_else(|| {
                // 回退：尝试从嵌套的 cache_creation 对象聚合
                if let Some(cache_obj) = usage.get("cache_creation") {
                    let ephemeral_5m = cache_obj
                        .get("ephemeral_5m_input_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);
                    let ephemeral_1h = cache_obj
                        .get("ephemeral_1h_input_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);
                    ephemeral_5m + ephemeral_1h
                } else {
                    0
                }
            });

        let cache_read = usage
            .get("cache_read_input_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        Ok(ResponseTokenInfo {
            model,
            message_id,
            input_tokens,
            output_tokens,
            cache_creation_tokens: cache_creation,
            cache_read_tokens: cache_read,
            reasoning_tokens: 0, // Claude 不使用 reasoning tokens
        })
    }
}

/// Codex 工具的 Token 提取器
pub struct CodexTokenExtractor;

impl TokenExtractor for CodexTokenExtractor {
    fn extract_model_from_request(&self, body: &[u8]) -> Result<String> {
        let json: Value =
            serde_json::from_slice(body).context("Failed to parse request body as JSON")?;

        json.get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .context("Missing 'model' field in request body")
    }

    fn extract_from_sse_chunk(&self, chunk: &str) -> Result<Option<SseTokenData>> {
        // SSE格式: data: {...} 或直接 {...}（已去掉前缀）
        let data_line = chunk.trim();

        // 跳过空行
        if data_line.is_empty() {
            return Ok(None);
        }

        // 兼容处理：去掉 "data: " 前缀（如果存在）
        let json_str = if let Some(stripped) = data_line.strip_prefix("data: ") {
            stripped
        } else {
            data_line
        };

        // 跳过 [DONE] 标记
        if json_str.trim() == "[DONE]" {
            return Ok(None);
        }

        let json: Value =
            serde_json::from_str(json_str).context("Failed to parse SSE chunk as JSON")?;

        let event_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");

        tracing::debug!(event_type = event_type, "解析 Codex SSE 事件");

        match event_type {
            "response.created" => {
                if let Some(response) = json.get("response") {
                    let response_id = response
                        .get("id")
                        .and_then(|v| v.as_str())
                        .context("Missing id in response.created")?
                        .to_string();

                    tracing::debug!(response_id = %response_id, "Codex response.created");

                    // 返回占位符 MessageStartData（model 从请求体获取）
                    Ok(Some(SseTokenData {
                        message_start: Some(MessageStartData {
                            model: "unknown".to_string(),
                            message_id: response_id,
                            input_tokens: 0,
                            output_tokens: 0,
                            cache_creation_tokens: 0,
                            cache_read_tokens: 0,
                        }),
                        message_delta: None,
                    }))
                } else {
                    Ok(None)
                }
            }
            "response.completed" => {
                if let Some(response) = json.get("response") {
                    let response_id = response
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let usage = response
                        .get("usage")
                        .context("Missing usage in response.completed")?;

                    let input_tokens = usage
                        .get("input_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);

                    let cached_tokens = usage
                        .get("input_tokens_details")
                        .and_then(|d| d.get("cached_tokens"))
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);

                    let output_tokens = usage
                        .get("output_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);

                    let reasoning_tokens = usage
                        .get("output_tokens_details")
                        .and_then(|d| d.get("reasoning_tokens"))
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);

                    if reasoning_tokens > 0 {
                        tracing::info!(
                            reasoning_tokens = reasoning_tokens,
                            "Codex 响应包含 reasoning tokens（暂不计费）"
                        );
                    }

                    tracing::debug!(
                        response_id = %response_id,
                        input_tokens = input_tokens,
                        cached_tokens = cached_tokens,
                        output_tokens = output_tokens,
                        "Codex response.completed"
                    );

                    // 返回完整的 MessageDeltaData（包含 input_tokens）
                    // 注意：Codex 的 input_tokens 在这里，不在 message_start
                    Ok(Some(SseTokenData {
                        message_start: None,
                        message_delta: Some(MessageDeltaData {
                            input_tokens: Some(input_tokens), // Codex 的 input_tokens
                            cache_creation_tokens: 0,
                            cache_read_tokens: cached_tokens,
                            output_tokens,
                        }),
                    }))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    fn extract_from_json(&self, json: &Value) -> Result<ResponseTokenInfo> {
        let model = json
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let message_id = json
            .get("id")
            .and_then(|v| v.as_str())
            .context("Missing id field")?
            .to_string();

        let usage = json.get("usage").context("Missing usage field")?;

        let input_tokens = usage
            .get("input_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let cached_tokens = usage
            .get("input_tokens_details")
            .and_then(|d| d.get("cached_tokens"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let output_tokens = usage
            .get("output_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let reasoning_tokens = usage
            .get("output_tokens_details")
            .and_then(|d| d.get("reasoning_tokens"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        Ok(ResponseTokenInfo {
            model,
            message_id,
            input_tokens,
            output_tokens,
            cache_creation_tokens: 0,
            cache_read_tokens: cached_tokens,
            reasoning_tokens,
        })
    }
}

/// 创建Token提取器工厂函数
pub fn create_extractor(tool_type: &str) -> Result<Box<dyn TokenExtractor>> {
    // 支持破折号和下划线两种格式
    let normalized = tool_type.replace('-', "_");
    match normalized.as_str() {
        "claude_code" => Ok(Box::new(ClaudeTokenExtractor)),
        "codex" => Ok(Box::new(CodexTokenExtractor)),
        "gemini_cli" => anyhow::bail!("Gemini CLI token extractor not implemented yet"),
        _ => anyhow::bail!("Unknown tool type: {}", tool_type),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_model_from_request() {
        let extractor = ClaudeTokenExtractor;
        let body = r#"{"model":"claude-sonnet-4-5-20250929","messages":[]}"#;

        let model = extractor
            .extract_model_from_request(body.as_bytes())
            .unwrap();
        assert_eq!(model, "claude-sonnet-4-5-20250929");
    }

    #[test]
    fn test_extract_from_sse_message_start() {
        let extractor = ClaudeTokenExtractor;
        let chunk = r#"data: {"type":"message_start","message":{"model":"claude-haiku-4-5-20251001","id":"msg_123","type":"message","role":"assistant","content":[],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":27592,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":1}}}"#;

        let result = extractor.extract_from_sse_chunk(chunk).unwrap().unwrap();
        assert!(result.message_start.is_some());

        let start = result.message_start.unwrap();
        assert_eq!(start.model, "claude-haiku-4-5-20251001");
        assert_eq!(start.message_id, "msg_123");
        assert_eq!(start.input_tokens, 27592);
        assert_eq!(start.output_tokens, 1);
        assert_eq!(start.cache_creation_tokens, 0);
        assert_eq!(start.cache_read_tokens, 0);
    }

    #[test]
    fn test_extract_from_sse_message_delta() {
        let extractor = ClaudeTokenExtractor;
        let chunk = r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"input_tokens":27592,"cache_creation_input_tokens":100,"cache_read_input_tokens":200,"output_tokens":12}}"#;

        let result = extractor.extract_from_sse_chunk(chunk).unwrap().unwrap();
        assert!(result.message_delta.is_some());

        let delta = result.message_delta.unwrap();
        assert_eq!(delta.cache_creation_tokens, 100);
        assert_eq!(delta.cache_read_tokens, 200);
        assert_eq!(delta.output_tokens, 12);
    }

    #[test]
    fn test_extract_from_json() {
        let extractor = ClaudeTokenExtractor;
        let json_str = r#"{
            "content": [{"text": "test", "type": "text"}],
            "id": "msg_018K1Hs5Tm7sC7xdeYpYhUFN",
            "model": "claude-haiku-4-5-20251001",
            "role": "assistant",
            "stop_reason": "end_turn",
            "type": "message",
            "usage": {
                "cache_creation_input_tokens": 50,
                "cache_read_input_tokens": 100,
                "input_tokens": 119,
                "output_tokens": 21
            }
        }"#;

        let json: Value = serde_json::from_str(json_str).unwrap();
        let result = extractor.extract_from_json(&json).unwrap();

        assert_eq!(result.model, "claude-haiku-4-5-20251001");
        assert_eq!(result.message_id, "msg_018K1Hs5Tm7sC7xdeYpYhUFN");
        assert_eq!(result.input_tokens, 119);
        assert_eq!(result.output_tokens, 21);
        assert_eq!(result.cache_creation_tokens, 50);
        assert_eq!(result.cache_read_tokens, 100);
    }

    #[test]
    fn test_response_token_info_from_sse() {
        let start = MessageStartData {
            model: "claude-3".to_string(),
            message_id: "msg_123".to_string(),
            input_tokens: 1000,
            output_tokens: 1,
            cache_creation_tokens: 50,
            cache_read_tokens: 100,
        };

        let delta = MessageDeltaData {
            cache_creation_tokens: 50,
            cache_read_tokens: 100,
            output_tokens: 200,
        };

        let info = ResponseTokenInfo::from_sse_data(start, Some(delta));
        assert_eq!(info.model, "claude-3");
        assert_eq!(info.input_tokens, 1000);
        assert_eq!(info.output_tokens, 200);
        assert_eq!(info.cache_creation_tokens, 50);
        assert_eq!(info.cache_read_tokens, 100);
    }

    #[test]
    fn test_create_extractor() {
        assert!(create_extractor("claude_code").is_ok());
        assert!(create_extractor("codex").is_ok());
        assert!(create_extractor("gemini_cli").is_err());
        assert!(create_extractor("unknown").is_err());
    }

    #[test]
    fn test_extract_nested_cache_creation_json() {
        // 测试嵌套 cache_creation 对象的提取（JSON 响应）
        let extractor = ClaudeTokenExtractor;
        let json_str = r#"{
            "id": "msg_013B8kRbTZdntKmHWE6AZzuU",
            "model": "claude-sonnet-4-5-20250929",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "test"}],
            "usage": {
                "cache_creation": {
                    "ephemeral_1h_input_tokens": 0,
                    "ephemeral_5m_input_tokens": 73444
                },
                "cache_creation_input_tokens": 73444,
                "cache_read_input_tokens": 19198,
                "input_tokens": 12,
                "output_tokens": 259,
                "service_tier": "standard"
            }
        }"#;

        let json: Value = serde_json::from_str(json_str).unwrap();
        let result = extractor.extract_from_json(&json).unwrap();

        assert_eq!(result.model, "claude-sonnet-4-5-20250929");
        assert_eq!(result.message_id, "msg_013B8kRbTZdntKmHWE6AZzuU");
        assert_eq!(result.input_tokens, 12);
        assert_eq!(result.output_tokens, 259);
        assert_eq!(result.cache_creation_tokens, 73444);
        assert_eq!(result.cache_read_tokens, 19198);
    }

    #[test]
    fn test_extract_nested_cache_creation_sse_start() {
        // 测试嵌套 cache_creation 对象的提取（SSE message_start）
        let extractor = ClaudeTokenExtractor;
        let chunk = r#"data: {"type":"message_start","message":{"model":"claude-sonnet-4-5-20250929","id":"msg_018GWR1gBaJBchrC6t5nnRui","type":"message","role":"assistant","content":[],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":9,"cache_creation_input_tokens":2122,"cache_read_input_tokens":123663,"cache_creation":{"ephemeral_5m_input_tokens":2122,"ephemeral_1h_input_tokens":0},"output_tokens":1,"service_tier":"standard"}}}"#;

        let result = extractor.extract_from_sse_chunk(chunk).unwrap().unwrap();
        assert!(result.message_start.is_some());

        let start = result.message_start.unwrap();
        assert_eq!(start.model, "claude-sonnet-4-5-20250929");
        assert_eq!(start.message_id, "msg_018GWR1gBaJBchrC6t5nnRui");
        assert_eq!(start.input_tokens, 9);
        assert_eq!(start.output_tokens, 1);
        assert_eq!(start.cache_creation_tokens, 2122);
        assert_eq!(start.cache_read_tokens, 123663);
    }

    #[test]
    fn test_extract_message_delta_with_tool_use() {
        // 测试 stop_reason="tool_use" 的情况
        let extractor = ClaudeTokenExtractor;
        let chunk = r#"data: {"type":"message_delta","delta":{"stop_reason":"tool_use","stop_sequence":null},"usage":{"input_tokens":9,"cache_creation_input_tokens":2122,"cache_read_input_tokens":123663,"output_tokens":566}}"#;

        let result = extractor.extract_from_sse_chunk(chunk).unwrap().unwrap();
        assert!(result.message_delta.is_some());

        let delta = result.message_delta.unwrap();
        assert_eq!(delta.cache_creation_tokens, 2122);
        assert_eq!(delta.cache_read_tokens, 123663);
        assert_eq!(delta.output_tokens, 566);
    }

    #[test]
    fn test_from_sse_data_without_delta() {
        // 测试没有 delta 时使用 start 的缓存值
        let start = MessageStartData {
            model: "claude-3".to_string(),
            message_id: "msg_test".to_string(),
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_tokens: 200,
            cache_read_tokens: 300,
        };

        let info = ResponseTokenInfo::from_sse_data(start, None);
        assert_eq!(info.input_tokens, 100);
        assert_eq!(info.output_tokens, 50);
        assert_eq!(info.cache_creation_tokens, 200);
        assert_eq!(info.cache_read_tokens, 300);
    }

    // ========== Codex Token Extractor Tests ==========

    #[test]
    fn test_codex_extract_model_from_request() {
        let extractor = CodexTokenExtractor;
        let body = r#"{"model":"gpt-4","messages":[],"prompt_cache_key":"test123"}"#;

        let model = extractor
            .extract_model_from_request(body.as_bytes())
            .unwrap();
        assert_eq!(model, "gpt-4");
    }

    #[test]
    fn test_codex_sse_response_created() {
        let extractor = CodexTokenExtractor;
        let chunk = r#"{"type":"response.created","response":{"id":"resp_abc123"}}"#;

        let result = extractor.extract_from_sse_chunk(chunk).unwrap();
        assert!(result.is_some());

        let data = result.unwrap();
        assert!(data.message_start.is_some());
        assert!(data.message_delta.is_none());

        let start = data.message_start.unwrap();
        assert_eq!(start.message_id, "resp_abc123");
        assert_eq!(start.input_tokens, 0);
        assert_eq!(start.output_tokens, 0);
    }

    #[test]
    fn test_codex_sse_response_completed() {
        let extractor = CodexTokenExtractor;
        let chunk = r#"{
            "type":"response.completed",
            "response":{
                "id":"resp_abc123",
                "usage":{
                    "input_tokens":8299,
                    "input_tokens_details":{"cached_tokens":100},
                    "output_tokens":36,
                    "output_tokens_details":{"reasoning_tokens":0},
                    "total_tokens":8335
                }
            }
        }"#;

        let result = extractor.extract_from_sse_chunk(chunk).unwrap();
        assert!(result.is_some());

        let data = result.unwrap();
        assert!(data.message_start.is_none());
        assert!(data.message_delta.is_some());

        let delta = data.message_delta.unwrap();
        assert_eq!(delta.cache_read_tokens, 100);
        assert_eq!(delta.output_tokens, 36);
        assert_eq!(delta.cache_creation_tokens, 0);
    }

    #[test]
    fn test_codex_sse_response_completed_with_reasoning() {
        let extractor = CodexTokenExtractor;
        let chunk = r#"{
            "type":"response.completed",
            "response":{
                "id":"resp_xyz",
                "usage":{
                    "input_tokens":1000,
                    "input_tokens_details":{"cached_tokens":0},
                    "output_tokens":500,
                    "output_tokens_details":{"reasoning_tokens":200},
                    "total_tokens":1500
                }
            }
        }"#;

        let result = extractor.extract_from_sse_chunk(chunk).unwrap();
        assert!(result.is_some());

        let data = result.unwrap();
        let delta = data.message_delta.unwrap();
        assert_eq!(delta.output_tokens, 500);
        // reasoning_tokens 记录到日志但不影响计费
    }

    #[test]
    fn test_codex_json_response() {
        let extractor = CodexTokenExtractor;
        let json_str = r#"{
            "id":"resp_abc123",
            "model":"gpt-4",
            "usage":{
                "input_tokens":100,
                "input_tokens_details":{"cached_tokens":50},
                "output_tokens":20
            }
        }"#;

        let json: Value = serde_json::from_str(json_str).unwrap();
        let result = extractor.extract_from_json(&json).unwrap();

        assert_eq!(result.model, "gpt-4");
        assert_eq!(result.message_id, "resp_abc123");
        assert_eq!(result.input_tokens, 100);
        assert_eq!(result.cache_read_tokens, 50);
        assert_eq!(result.output_tokens, 20);
        assert_eq!(result.cache_creation_tokens, 0);
    }

    #[test]
    fn test_codex_json_response_no_cached() {
        let extractor = CodexTokenExtractor;
        let json_str = r#"{
            "id":"resp_test",
            "model":"gpt-3.5-turbo",
            "usage":{
                "input_tokens":200,
                "output_tokens":50
            }
        }"#;

        let json: Value = serde_json::from_str(json_str).unwrap();
        let result = extractor.extract_from_json(&json).unwrap();

        assert_eq!(result.input_tokens, 200);
        assert_eq!(result.cache_read_tokens, 0);
        assert_eq!(result.output_tokens, 50);
    }

    #[test]
    fn test_create_codex_extractor() {
        let extractor = create_extractor("codex");
        assert!(extractor.is_ok());
    }
}
