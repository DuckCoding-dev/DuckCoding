//! Claude Code 工具的 Token 处理器

use super::{TokenInfo, ToolProcessor};
use anyhow::{Context, Result};
use serde_json::Value;

/// Claude Code 工具处理器
pub struct ClaudeProcessor;

impl ToolProcessor for ClaudeProcessor {
    fn tool_id(&self) -> &str {
        "claude-code"
    }

    fn process_sse_response(
        &self,
        request_body: &[u8],
        sse_chunks: Vec<String>,
    ) -> Result<TokenInfo> {
        // 1. 从请求体提取 model
        let request_json: Value =
            serde_json::from_slice(request_body).context("Failed to parse request body")?;
        let model = request_json
            .get("model")
            .and_then(|v| v.as_str())
            .context("Missing 'model' field in request body")?
            .to_string();

        // 2. 解析 SSE 事件，收集 message_start 和 message_delta
        let mut message_id: Option<String> = None;
        let mut input_tokens = 0i64;
        let mut output_tokens = 0i64;
        let mut cache_creation_tokens = 0i64;
        let mut cache_creation_1h_tokens = 0i64;
        let mut cache_read_tokens = 0i64;

        for chunk in sse_chunks {
            let data_line = chunk.trim();

            // 跳过空行
            if data_line.is_empty() {
                continue;
            }

            // 去掉 "data: " 前缀
            let json_str = if let Some(stripped) = data_line.strip_prefix("data: ") {
                stripped
            } else {
                data_line
            };

            // 跳过 [DONE] 标记
            if json_str.trim() == "[DONE]" {
                continue;
            }

            let json: Value = match serde_json::from_str(json_str) {
                Ok(j) => j,
                Err(e) => {
                    tracing::warn!("Failed to parse SSE chunk: {}", e);
                    continue;
                }
            };

            let event_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");

            match event_type {
                "message_start" => {
                    if let Some(message) = json.get("message") {
                        // 提取 message_id
                        if let Some(id) = message.get("id").and_then(|v| v.as_str()) {
                            message_id = Some(id.to_string());
                        }

                        // 提取 usage
                        if let Some(usage) = message.get("usage") {
                            input_tokens = usage
                                .get("input_tokens")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0);

                            output_tokens = usage
                                .get("output_tokens")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0);

                            // 提取缓存创建 token：优先读取扁平字段，回退到嵌套对象
                            if let Some(flat_val) = usage
                                .get("cache_creation_input_tokens")
                                .and_then(|v| v.as_i64())
                            {
                                // 扁平字段：无法区分 5m/1h，全部视为 5m
                                cache_creation_tokens = flat_val;
                                cache_creation_1h_tokens = 0;
                            } else if let Some(cache_obj) = usage.get("cache_creation") {
                                let ephemeral_5m = cache_obj
                                    .get("ephemeral_5m_input_tokens")
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0);
                                let ephemeral_1h = cache_obj
                                    .get("ephemeral_1h_input_tokens")
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0);
                                cache_creation_tokens = ephemeral_5m + ephemeral_1h;
                                cache_creation_1h_tokens = ephemeral_1h;
                            }

                            cache_read_tokens = usage
                                .get("cache_read_input_tokens")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0);

                            tracing::debug!(
                                model = %model,
                                message_id = ?message_id,
                                input_tokens = input_tokens,
                                cache_creation_1h_tokens = cache_creation_1h_tokens,
                                "Claude message_start 提取成功"
                            );
                        }
                    }
                }
                "message_delta" => {
                    // message_delta 包含最终的 usage 统计（累加值）
                    if let Some(usage) = json.get("usage") {
                        // 更新 output_tokens 和缓存统计（这些是最终值）
                        output_tokens = usage
                            .get("output_tokens")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(output_tokens);

                        if let Some(flat_val) = usage
                            .get("cache_creation_input_tokens")
                            .and_then(|v| v.as_i64())
                        {
                            cache_creation_tokens = flat_val;
                            // 扁平字段无法区分 5m/1h，保持之前的 1h 值
                        } else if let Some(cache_obj) = usage.get("cache_creation") {
                            let ephemeral_5m = cache_obj
                                .get("ephemeral_5m_input_tokens")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0);
                            let ephemeral_1h = cache_obj
                                .get("ephemeral_1h_input_tokens")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0);
                            cache_creation_tokens = ephemeral_5m + ephemeral_1h;
                            cache_creation_1h_tokens = ephemeral_1h;
                        }

                        cache_read_tokens = usage
                            .get("cache_read_input_tokens")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(cache_read_tokens);

                        tracing::debug!(
                            output_tokens = output_tokens,
                            cache_creation_tokens = cache_creation_tokens,
                            cache_creation_1h_tokens = cache_creation_1h_tokens,
                            cache_read_tokens = cache_read_tokens,
                            "Claude message_delta 提取成功"
                        );
                    }
                }
                _ => {}
            }
        }

        // 3. 验证必需字段
        let message_id = message_id.context("Missing message_id in SSE stream")?;

        // 4. 构建 TokenInfo
        Ok(TokenInfo::new(
            model,
            message_id,
            input_tokens,
            output_tokens,
            cache_creation_tokens,
            cache_creation_1h_tokens,
            cache_read_tokens,
            0, // Claude 不使用 reasoning tokens
        ))
    }

    fn process_json_response(&self, request_body: &[u8], json: &Value) -> Result<TokenInfo> {
        // 1. 提取 model（优先使用响应中的 model，回退到请求体）
        let model = json
            .get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                // 回退到请求体
                serde_json::from_slice::<Value>(request_body)
                    .ok()
                    .and_then(|req| {
                        req.get("model")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    })
            })
            .context("Missing 'model' field in both response and request")?;

        // 2. 提取 message_id
        let message_id = json
            .get("id")
            .and_then(|v| v.as_str())
            .context("Missing 'id' field in response")?
            .to_string();

        // 3. 提取 usage
        let usage = json
            .get("usage")
            .context("Missing 'usage' field in response")?;

        let input_tokens = usage
            .get("input_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let output_tokens = usage
            .get("output_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        // 提取缓存创建 token：优先读取扁平字段，回退到嵌套对象
        let (cache_creation_tokens, cache_creation_1h_tokens) = if let Some(flat_val) = usage
            .get("cache_creation_input_tokens")
            .and_then(|v| v.as_i64())
        {
            // 扁平字段：无法区分 5m/1h，全部视为 5m
            (flat_val, 0)
        } else if let Some(cache_obj) = usage.get("cache_creation") {
            let ephemeral_5m = cache_obj
                .get("ephemeral_5m_input_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let ephemeral_1h = cache_obj
                .get("ephemeral_1h_input_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            (ephemeral_5m + ephemeral_1h, ephemeral_1h)
        } else {
            (0, 0)
        };

        let cache_read_tokens = usage
            .get("cache_read_input_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        // 4. 构建 TokenInfo
        Ok(TokenInfo::new(
            model,
            message_id,
            input_tokens,
            output_tokens,
            cache_creation_tokens,
            cache_creation_1h_tokens,
            cache_read_tokens,
            0, // Claude 不使用 reasoning tokens
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_sse_response() {
        let processor = ClaudeProcessor;
        let request_body = r#"{"model":"claude-sonnet-4-5-20250929","messages":[]}"#;
        let sse_chunks = vec![
            r#"data: {"type":"message_start","message":{"model":"claude-sonnet-4-5-20250929","id":"msg_123","type":"message","role":"assistant","content":[],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":1000,"cache_creation_input_tokens":100,"cache_read_input_tokens":200,"output_tokens":1}}}"#.to_string(),
            r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#.to_string(),
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#.to_string(),
            r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":12}}"#.to_string(),
        ];

        let result = processor
            .process_sse_response(request_body.as_bytes(), sse_chunks)
            .unwrap();

        assert_eq!(result.model, "claude-sonnet-4-5-20250929");
        assert_eq!(result.message_id, "msg_123");
        assert_eq!(result.input_tokens, 1000);
        assert_eq!(result.output_tokens, 12); // message_delta 的最终值
        assert_eq!(result.cache_creation_tokens, 100);
        assert_eq!(result.cache_creation_1h_tokens, 0); // 扁平字段无法区分，全部视为 5m
        assert_eq!(result.cache_read_tokens, 200);
        assert_eq!(result.reasoning_tokens, 0);
    }

    #[test]
    fn test_process_json_response() {
        let processor = ClaudeProcessor;
        let request_body = r#"{"model":"claude-sonnet-4-5-20250929","messages":[]}"#;
        let json_str = r#"{
            "id": "msg_123",
            "model": "claude-sonnet-4-5-20250929",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "Hello"}],
            "usage": {
                "input_tokens": 1000,
                "output_tokens": 500,
                "cache_creation_input_tokens": 100,
                "cache_read_input_tokens": 200
            }
        }"#;

        let json: Value = serde_json::from_str(json_str).unwrap();
        let result = processor
            .process_json_response(request_body.as_bytes(), &json)
            .unwrap();

        assert_eq!(result.model, "claude-sonnet-4-5-20250929");
        assert_eq!(result.message_id, "msg_123");
        assert_eq!(result.input_tokens, 1000);
        assert_eq!(result.output_tokens, 500);
        assert_eq!(result.cache_creation_tokens, 100);
        assert_eq!(result.cache_creation_1h_tokens, 0); // 扁平字段全部视为 5m
        assert_eq!(result.cache_read_tokens, 200);
        assert_eq!(result.reasoning_tokens, 0);
    }

    #[test]
    fn test_process_json_nested_cache_creation() {
        let processor = ClaudeProcessor;
        let request_body = r#"{"model":"claude-sonnet-4-5-20250929","messages":[]}"#;
        let json_str = r#"{
            "id": "msg_456",
            "model": "claude-sonnet-4-5-20250929",
            "usage": {
                "input_tokens": 500,
                "output_tokens": 300,
                "cache_creation": {
                    "ephemeral_5m_input_tokens": 50,
                    "ephemeral_1h_input_tokens": 100
                },
                "cache_read_input_tokens": 200
            }
        }"#;

        let json: Value = serde_json::from_str(json_str).unwrap();
        let result = processor
            .process_json_response(request_body.as_bytes(), &json)
            .unwrap();

        assert_eq!(result.cache_creation_tokens, 150); // 50 + 100
        assert_eq!(result.cache_creation_1h_tokens, 100); // 1h 部分
        assert_eq!(result.cache_read_tokens, 200);
    }
}
