//! Claude Code 工具的日志记录器

use super::{LogStatus, ResponseType, TokenLogger};
use crate::models::token_stats::TokenLog;
use crate::services::pricing::PRICING_MANAGER;
use crate::services::token_stats::processor::{create_processor, TokenInfo};
use anyhow::Result;
use chrono::Utc;

/// Claude Code 日志记录器
pub struct ClaudeLogger;

impl ClaudeLogger {
    /// 从 TokenInfo 构建 TokenLog
    #[allow(clippy::too_many_arguments)]
    fn build_log(
        &self,
        token_info: TokenInfo,
        session_id: String,
        config_name: String,
        client_ip: String,
        response_time_ms: Option<i64>,
        response_type: ResponseType,
        status: LogStatus,
    ) -> Result<TokenLog> {
        // 计算成本
        let cost_result = PRICING_MANAGER.calculate_cost(
            None,                // 使用默认模板
            Some("claude-code"), // 工具 ID
            &token_info.model,
            token_info.input_tokens,
            token_info.output_tokens,
            token_info.cache_creation_tokens,
            token_info.cache_creation_1h_tokens,
            token_info.cache_read_tokens,
            token_info.reasoning_tokens,
        );

        let (
            input_price,
            output_price,
            cache_write_price,
            cache_read_price,
            reasoning_price,
            total_cost,
            template_id,
        ) = match cost_result {
            Ok(breakdown) => (
                Some(breakdown.input_price),
                Some(breakdown.output_price),
                Some(breakdown.cache_write_price),
                Some(breakdown.cache_read_price),
                Some(breakdown.reasoning_price),
                breakdown.total_cost,
                Some(breakdown.template_id),
            ),
            Err(e) => {
                tracing::warn!("Failed to calculate cost: {}", e);
                (None, None, None, None, None, 0.0, None)
            }
        };

        Ok(TokenLog::new(
            self.tool_id().to_string(),
            Utc::now().timestamp_millis(),
            client_ip,
            session_id,
            config_name,
            token_info.model,
            Some(token_info.message_id),
            token_info.input_tokens,
            token_info.output_tokens,
            token_info.cache_creation_tokens,
            token_info.cache_creation_1h_tokens,
            token_info.cache_read_tokens,
            token_info.reasoning_tokens,
            status.as_str().to_string(),
            response_type.as_str().to_string(),
            None, // error_type
            None, // error_detail
            response_time_ms,
            input_price,
            output_price,
            cache_write_price,
            cache_read_price,
            reasoning_price,
            total_cost,
            template_id,
        ))
    }
}

impl TokenLogger for ClaudeLogger {
    fn tool_id(&self) -> &str {
        "claude-code"
    }

    fn log_sse_response(
        &self,
        request_body: &[u8],
        sse_chunks: Vec<String>,
        session_id: String,
        config_name: String,
        client_ip: String,
        response_time_ms: Option<i64>,
    ) -> Result<TokenLog> {
        // 使用 processor 提取 TokenInfo
        let processor = create_processor("claude-code")?;
        let token_info = processor.process_sse_response(request_body, sse_chunks)?;

        // 构建日志（成功状态）
        self.build_log(
            token_info,
            session_id,
            config_name,
            client_ip,
            response_time_ms,
            ResponseType::Sse,
            LogStatus::Success,
        )
    }

    fn log_json_response(
        &self,
        request_body: &[u8],
        json: &serde_json::Value,
        session_id: String,
        config_name: String,
        client_ip: String,
        response_time_ms: Option<i64>,
    ) -> Result<TokenLog> {
        // 使用 processor 提取 TokenInfo
        let processor = create_processor("claude-code")?;
        let token_info = processor.process_json_response(request_body, json)?;

        // 构建日志（成功状态）
        self.build_log(
            token_info,
            session_id,
            config_name,
            client_ip,
            response_time_ms,
            ResponseType::Json,
            LogStatus::Success,
        )
    }

    fn log_failed_request(
        &self,
        request_body: &[u8],
        session_id: String,
        config_name: String,
        client_ip: String,
        response_time_ms: Option<i64>,
        error_type: String,
        error_detail: String,
    ) -> Result<TokenLog> {
        // 尝试从请求体提取 model
        let model = serde_json::from_slice::<serde_json::Value>(request_body)
            .ok()
            .and_then(|req| {
                req.get("model")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "unknown".to_string());

        Ok(TokenLog::new(
            self.tool_id().to_string(),
            Utc::now().timestamp_millis(),
            client_ip,
            session_id,
            config_name,
            model,
            None, // message_id
            0,    // input_tokens
            0,    // output_tokens
            0,    // cache_creation_tokens
            0,    // cache_creation_1h_tokens
            0,    // cache_read_tokens
            0,    // reasoning_tokens
            LogStatus::Failed.as_str().to_string(),
            ResponseType::Unknown.as_str().to_string(),
            Some(error_type),
            Some(error_detail),
            response_time_ms,
            None, // input_price
            None, // output_price
            None, // cache_write_price
            None, // cache_read_price
            None, // reasoning_price
            0.0,  // total_cost
            None, // pricing_template_id
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_sse_response() {
        let logger = ClaudeLogger;
        let request_body = r#"{"model":"claude-sonnet-4-5-20250929","messages":[]}"#;
        let sse_chunks = vec![
            r#"data: {"type":"message_start","message":{"model":"claude-sonnet-4-5-20250929","id":"msg_123","type":"message","role":"assistant","content":[],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":1000,"cache_creation_input_tokens":100,"cache_read_input_tokens":200,"output_tokens":1}}}"#.to_string(),
            r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":500}}"#.to_string(),
        ];

        let log = logger
            .log_sse_response(
                request_body.as_bytes(),
                sse_chunks,
                "session_123".to_string(),
                "default".to_string(),
                "127.0.0.1".to_string(),
                Some(100),
            )
            .unwrap();

        assert_eq!(log.tool_type, "claude-code");
        assert_eq!(log.model, "claude-sonnet-4-5-20250929");
        assert_eq!(log.message_id, Some("msg_123".to_string()));
        assert_eq!(log.input_tokens, 1000);
        assert_eq!(log.output_tokens, 500);
        assert_eq!(log.request_status, "success");
        assert_eq!(log.response_type, "sse");
        assert!(log.total_cost > 0.0);
    }

    #[test]
    fn test_log_json_response() {
        let logger = ClaudeLogger;
        let request_body = r#"{"model":"claude-sonnet-4-5-20250929","messages":[]}"#;
        let json_str = r#"{
            "id": "msg_456",
            "model": "claude-sonnet-4-5-20250929",
            "usage": {
                "input_tokens": 500,
                "output_tokens": 300,
                "cache_creation_input_tokens": 50,
                "cache_read_input_tokens": 100
            }
        }"#;
        let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

        let log = logger
            .log_json_response(
                request_body.as_bytes(),
                &json,
                "session_456".to_string(),
                "custom".to_string(),
                "192.168.1.1".to_string(),
                Some(200),
            )
            .unwrap();

        assert_eq!(log.tool_type, "claude-code");
        assert_eq!(log.model, "claude-sonnet-4-5-20250929");
        assert_eq!(log.input_tokens, 500);
        assert_eq!(log.output_tokens, 300);
        assert_eq!(log.request_status, "success");
        assert_eq!(log.response_type, "json");
    }

    #[test]
    fn test_log_failed_request() {
        let logger = ClaudeLogger;
        let request_body = r#"{"model":"claude-sonnet-4-5-20250929","messages":[]}"#;

        let log = logger
            .log_failed_request(
                request_body.as_bytes(),
                "session_789".to_string(),
                "default".to_string(),
                "127.0.0.1".to_string(),
                Some(50),
                "network_error".to_string(),
                "Connection timeout".to_string(),
            )
            .unwrap();

        assert_eq!(log.tool_type, "claude-code");
        assert_eq!(log.model, "claude-sonnet-4-5-20250929");
        assert_eq!(log.request_status, "failed");
        assert_eq!(log.response_type, "unknown");
        assert_eq!(log.error_type, Some("network_error".to_string()));
        assert_eq!(log.error_detail, Some("Connection timeout".to_string()));
        assert_eq!(log.total_cost, 0.0);
    }
}
