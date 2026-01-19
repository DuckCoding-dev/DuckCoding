//! Codex 工具的 Token 处理器

use super::{TokenInfo, ToolProcessor};
use anyhow::{Context, Result};
use serde_json::Value;

/// Codex 工具处理器
pub struct CodexProcessor;

impl ToolProcessor for CodexProcessor {
    fn tool_id(&self) -> &str {
        "codex"
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

        // 2. 解析 SSE 事件，收集 response.created 和 response.completed
        let mut message_id: Option<String> = None;
        let mut input_tokens = 0i64;
        let mut output_tokens = 0i64;
        let mut cache_read_tokens = 0i64;
        let mut reasoning_tokens = 0i64;

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
                "response.created" => {
                    // 提取 response_id
                    if let Some(response) = json.get("response") {
                        if let Some(id) = response.get("id").and_then(|v| v.as_str()) {
                            message_id = Some(id.to_string());
                            tracing::debug!(response_id = %id, "Codex response.created");
                        }
                    }
                }
                "response.completed" => {
                    // 提取完整的 usage 统计
                    if let Some(response) = json.get("response") {
                        // 更新 response_id（以防 created 事件缺失）
                        if message_id.is_none() {
                            if let Some(id) = response.get("id").and_then(|v| v.as_str()) {
                                message_id = Some(id.to_string());
                            }
                        }

                        if let Some(usage) = response.get("usage") {
                            // Codex 的 input_tokens 包括缓存的 token
                            // 需要减去 cached_tokens 才是真正的新输入
                            let total_input_tokens = usage
                                .get("input_tokens")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0);

                            output_tokens = usage
                                .get("output_tokens")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0);

                            // 提取 cached_tokens（缓存读取）
                            cache_read_tokens = usage
                                .get("input_tokens_details")
                                .and_then(|d| d.get("cached_tokens"))
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0);

                            // 计算实际新输入 = 总输入 - 缓存读取
                            // 这样才能避免重复计费
                            input_tokens = total_input_tokens - cache_read_tokens;

                            // 提取 reasoning_tokens
                            reasoning_tokens = usage
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
                                message_id = ?message_id,
                                total_input = total_input_tokens,
                                cached = cache_read_tokens,
                                new_input = input_tokens,
                                output_tokens = output_tokens,
                                "Codex response.completed 提取成功（input = total - cached）"
                            );
                        }
                    }
                }
                _ => {}
            }
        }

        // 3. 验证必需字段
        let message_id = message_id.context("Missing response_id in SSE stream")?;

        // 4. 构建 TokenInfo
        Ok(TokenInfo::new(
            model,
            message_id,
            input_tokens,
            output_tokens,
            0, // Codex 不报告 cache_creation_tokens
            cache_read_tokens,
            reasoning_tokens,
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
                    .and_then(|req| req.get("model").and_then(|v| v.as_str()).map(|s| s.to_string()))
            })
            .context("Missing 'model' field in both response and request")?;

        // 2. 提取 response_id
        let message_id = json
            .get("id")
            .and_then(|v| v.as_str())
            .context("Missing 'id' field in response")?
            .to_string();

        // 3. 提取 usage
        let usage = json.get("usage").context("Missing 'usage' field in response")?;

        // Codex 的 input_tokens 包括缓存的 token
        // 需要减去 cached_tokens 才是真正的新输入
        let total_input_tokens = usage
            .get("input_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let output_tokens = usage
            .get("output_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        // 提取 cached_tokens（缓存读取）
        let cache_read_tokens = usage
            .get("input_tokens_details")
            .and_then(|d| d.get("cached_tokens"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        // 计算实际新输入 = 总输入 - 缓存读取
        let input_tokens = total_input_tokens - cache_read_tokens;

        // 提取 reasoning_tokens
        let reasoning_tokens = usage
            .get("output_tokens_details")
            .and_then(|d| d.get("reasoning_tokens"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        // 4. 构建 TokenInfo
        Ok(TokenInfo::new(
            model,
            message_id,
            input_tokens,
            output_tokens,
            0, // Codex 不报告 cache_creation_tokens
            cache_read_tokens,
            reasoning_tokens,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_sse_response() {
        let processor = CodexProcessor;
        let request_body = r#"{"model":"gpt-5.1","messages":[],"prompt_cache_key":"test123"}"#;
        let sse_chunks = vec![
            r#"{"type":"response.created","response":{"id":"resp_abc123"}}"#.to_string(),
            r#"{"type":"response.output_item.added","item":{"type":"message","role":"assistant"}}"#
                .to_string(),
            r#"{"type":"response.completed","response":{"id":"resp_abc123","usage":{"input_tokens":10591,"input_tokens_details":{"cached_tokens":10240},"output_tokens":15,"output_tokens_details":{"reasoning_tokens":0},"total_tokens":10606}}}"#.to_string(),
        ];

        let result = processor
            .process_sse_response(request_body.as_bytes(), sse_chunks)
            .unwrap();

        assert_eq!(result.model, "gpt-5.1");
        assert_eq!(result.message_id, "resp_abc123");
        // input_tokens 应该是新输入 = 总输入 - 缓存 = 10591 - 10240 = 351
        assert_eq!(result.input_tokens, 351);
        assert_eq!(result.output_tokens, 15);
        assert_eq!(result.cache_creation_tokens, 0);
        assert_eq!(result.cache_read_tokens, 10240);
        assert_eq!(result.reasoning_tokens, 0);
    }

    #[test]
    fn test_process_sse_with_reasoning_tokens() {
        let processor = CodexProcessor;
        let request_body = r#"{"model":"gpt-5.1","messages":[]}"#;
        let sse_chunks = vec![
            r#"{"type":"response.created","response":{"id":"resp_xyz"}}"#.to_string(),
            r#"{"type":"response.completed","response":{"id":"resp_xyz","usage":{"input_tokens":1000,"input_tokens_details":{"cached_tokens":0},"output_tokens":500,"output_tokens_details":{"reasoning_tokens":200},"total_tokens":1500}}}"#.to_string(),
        ];

        let result = processor
            .process_sse_response(request_body.as_bytes(), sse_chunks)
            .unwrap();

        // 无缓存时，新输入 = 总输入 = 1000
        assert_eq!(result.input_tokens, 1000);
        assert_eq!(result.output_tokens, 500);
        assert_eq!(result.reasoning_tokens, 200);
    }

    #[test]
    fn test_process_json_response() {
        let processor = CodexProcessor;
        let request_body = r#"{"model":"gpt-4","messages":[]}"#;
        let json_str = r#"{
            "id": "resp_test123",
            "model": "gpt-4",
            "usage": {
                "input_tokens": 100,
                "input_tokens_details": {"cached_tokens": 50},
                "output_tokens": 20,
                "output_tokens_details": {"reasoning_tokens": 0}
            }
        }"#;

        let json: Value = serde_json::from_str(json_str).unwrap();
        let result = processor
            .process_json_response(request_body.as_bytes(), &json)
            .unwrap();

        assert_eq!(result.model, "gpt-4");
        assert_eq!(result.message_id, "resp_test123");
        // input_tokens 应该是新输入 = 100 - 50 = 50
        assert_eq!(result.input_tokens, 50);
        assert_eq!(result.output_tokens, 20);
        assert_eq!(result.cache_creation_tokens, 0);
        assert_eq!(result.cache_read_tokens, 50);
        assert_eq!(result.reasoning_tokens, 0);
    }

    #[test]
    fn test_process_json_no_cached_tokens() {
        let processor = CodexProcessor;
        let request_body = r#"{"model":"gpt-3.5","messages":[]}"#;
        let json_str = r#"{
            "id": "resp_456",
            "model": "gpt-3.5",
            "usage": {
                "input_tokens": 200,
                "output_tokens": 50
            }
        }"#;

        let json: Value = serde_json::from_str(json_str).unwrap();
        let result = processor
            .process_json_response(request_body.as_bytes(), &json)
            .unwrap();

        assert_eq!(result.input_tokens, 200);
        assert_eq!(result.output_tokens, 50);
        assert_eq!(result.cache_read_tokens, 0);
        assert_eq!(result.reasoning_tokens, 0);
    }
}
