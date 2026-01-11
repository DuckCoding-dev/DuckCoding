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
}

impl ResponseTokenInfo {
    /// 从SSE数据合并得到完整信息
    ///
    /// 合并规则：
    /// - model, message_id, input_tokens: 始终使用 message_start 的值
    /// - output_tokens, cache_*: 优先使用 message_delta 的值，回退到 message_start
    pub fn from_sse_data(start: MessageStartData, delta: Option<MessageDeltaData>) -> Self {
        let (cache_creation, cache_read, output) = if let Some(d) = delta {
            // 优先使用 delta 的值（最终统计）
            (
                d.cache_creation_tokens,
                d.cache_read_tokens,
                d.output_tokens,
            )
        } else {
            // 回退到 start 的值（初始统计）
            (
                start.cache_creation_tokens,
                start.cache_read_tokens,
                start.output_tokens,
            )
        };

        Self {
            model: start.model,
            message_id: start.message_id,
            input_tokens: start.input_tokens,
            output_tokens: output,
            cache_creation_tokens: cache_creation,
            cache_read_tokens: cache_read,
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
                // 检查是否有 stop_reason（任何值都接受：end_turn, tool_use, max_tokens 等）
                if let Some(delta) = json.get("delta") {
                    if delta.get("stop_reason").and_then(|v| v.as_str()).is_some() {
                        if let Some(usage) = json.get("usage") {
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

                            result.message_delta = Some(MessageDeltaData {
                                cache_creation_tokens: cache_creation,
                                cache_read_tokens: cache_read,
                                output_tokens,
                            });
                        }
                    }
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
        })
    }
}

/// 创建Token提取器工厂函数
pub fn create_extractor(tool_type: &str) -> Result<Box<dyn TokenExtractor>> {
    // 支持破折号和下划线两种格式
    let normalized = tool_type.replace('-', "_");
    match normalized.as_str() {
        "claude_code" => Ok(Box::new(ClaudeTokenExtractor)),
        // 预留扩展点
        "codex" => anyhow::bail!("Codex token extractor not implemented yet"),
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
        let body = r#"{"model":"claude-3-5-sonnet-20241022","messages":[]}"#;

        let model = extractor
            .extract_model_from_request(body.as_bytes())
            .unwrap();
        assert_eq!(model, "claude-3-5-sonnet-20241022");
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
        assert!(create_extractor("codex").is_err());
        assert!(create_extractor("gemini_cli").is_err());
        assert!(create_extractor("unknown").is_err());
    }

    #[test]
    fn test_extract_nested_cache_creation_json() {
        // 测试嵌套 cache_creation 对象的提取（JSON 响应）
        let extractor = ClaudeTokenExtractor;
        let json_str = r#"{
            "id": "msg_013B8kRbTZdntKmHWE6AZzuU",
            "model": "claude-3-5-sonnet-20241022",
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

        assert_eq!(result.model, "claude-3-5-sonnet-20241022");
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
}
