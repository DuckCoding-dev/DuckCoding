// 日志记录层
//
// 职责：统一的日志记录接口，处理成功/失败/解析错误等所有场景

use super::{ParsedResponse, RequestLogContext};
use crate::services::token_stats::manager::{ResponseData, TokenStatsManager};
use anyhow::Result;
use hyper::StatusCode;

pub struct LogRecorder;

impl LogRecorder {
    /// 记录请求日志（统一入口）
    pub async fn record(
        context: &RequestLogContext,
        response_status: u16,
        parsed: ParsedResponse,
    ) -> Result<()> {
        // 1. 检查 HTTP 状态码
        let status_code =
            StatusCode::from_u16(response_status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        if status_code.is_client_error() || status_code.is_server_error() {
            // HTTP 4xx/5xx 错误
            Self::record_http_error(context, response_status, &status_code).await
        } else {
            // HTTP 2xx/3xx 或无状态码，根据解析结果处理
            match parsed {
                ParsedResponse::Sse { data_lines } => {
                    // SSE 成功响应
                    Self::record_sse_success(context, data_lines).await
                }
                ParsedResponse::Json { data } => {
                    // JSON 成功响应
                    Self::record_json_success(context, data).await
                }
                ParsedResponse::Empty => {
                    // 空响应（上游失败）
                    Self::record_upstream_error(context, "上游返回空响应体").await
                }
                ParsedResponse::ParseError {
                    error,
                    response_type,
                    ..
                } => {
                    // 解析失败
                    Self::record_parse_error(context, &error, response_type).await
                }
            }
        }
    }

    /// 记录 SSE 成功响应
    async fn record_sse_success(
        context: &RequestLogContext,
        data_lines: Vec<String>,
    ) -> Result<()> {
        let manager = TokenStatsManager::get();

        match manager
            .log_request(
                &context.tool_id,
                &context.session_id,
                &context.config_name,
                &context.client_ip,
                &context.request_body,
                ResponseData::Sse(data_lines),
                context.response_time_ms,
                context.pricing_template_id.clone(),
            )
            .await
        {
            Ok(_) => {
                tracing::debug!(
                    tool_id = %context.tool_id,
                    session_id = %context.session_id,
                    "SSE 流式响应记录成功"
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    tool_id = %context.tool_id,
                    session_id = %context.session_id,
                    error = ?e,
                    "SSE Token 提取失败，记录为 parse_error"
                );

                // Token 提取失败，记录为 parse_error
                let error_detail = format!("SSE Token 提取失败: {}", e);
                manager
                    .log_failed_request(
                        &context.tool_id,
                        &context.session_id,
                        &context.config_name,
                        &context.client_ip,
                        &context.request_body,
                        "parse_error",
                        &error_detail,
                        "sse",
                        context.response_time_ms,
                    )
                    .await
            }
        }
    }

    /// 记录 JSON 成功响应
    async fn record_json_success(
        context: &RequestLogContext,
        data: serde_json::Value,
    ) -> Result<()> {
        let manager = TokenStatsManager::get();

        match manager
            .log_request(
                &context.tool_id,
                &context.session_id,
                &context.config_name,
                &context.client_ip,
                &context.request_body,
                ResponseData::Json(data),
                context.response_time_ms,
                context.pricing_template_id.clone(),
            )
            .await
        {
            Ok(_) => {
                tracing::debug!(
                    tool_id = %context.tool_id,
                    session_id = %context.session_id,
                    "JSON 响应记录成功"
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    tool_id = %context.tool_id,
                    session_id = %context.session_id,
                    error = ?e,
                    "JSON Token 提取失败，记录为 parse_error"
                );

                // Token 提取失败，记录为 parse_error
                let error_detail = format!("JSON Token 提取失败: {}", e);
                manager
                    .log_failed_request(
                        &context.tool_id,
                        &context.session_id,
                        &context.config_name,
                        &context.client_ip,
                        &context.request_body,
                        "parse_error",
                        &error_detail,
                        "json",
                        context.response_time_ms,
                    )
                    .await
            }
        }
    }

    /// 记录解析错误
    async fn record_parse_error(
        context: &RequestLogContext,
        error: &str,
        response_type: &str,
    ) -> Result<()> {
        let error_detail = format!("响应解析失败: {}", error);

        tracing::warn!(
            tool_id = %context.tool_id,
            session_id = %context.session_id,
            response_type = response_type,
            error = error,
            "响应解析失败"
        );

        TokenStatsManager::get()
            .log_failed_request(
                &context.tool_id,
                &context.session_id,
                &context.config_name,
                &context.client_ip,
                &context.request_body,
                "parse_error",
                &error_detail,
                response_type,
                context.response_time_ms,
            )
            .await
    }

    /// 记录上游错误（空响应或连接失败）
    pub async fn record_upstream_error(context: &RequestLogContext, detail: &str) -> Result<()> {
        tracing::warn!(
            tool_id = %context.tool_id,
            session_id = %context.session_id,
            detail = detail,
            is_stream = context.is_stream,
            "上游请求失败"
        );

        let response_type = if context.is_stream { "sse" } else { "json" };

        TokenStatsManager::get()
            .log_failed_request(
                &context.tool_id,
                &context.session_id,
                &context.config_name,
                &context.client_ip,
                &context.request_body,
                "upstream_error",
                detail,
                response_type, // 根据请求体的 stream 字段判断
                context.response_time_ms,
            )
            .await
    }

    /// 记录 HTTP 错误（4xx/5xx）
    async fn record_http_error(
        context: &RequestLogContext,
        status: u16,
        status_code: &StatusCode,
    ) -> Result<()> {
        let error_detail = format!(
            "HTTP {}: {}",
            status,
            status_code.canonical_reason().unwrap_or("Unknown")
        );

        tracing::warn!(
            tool_id = %context.tool_id,
            session_id = %context.session_id,
            status = status,
            is_stream = context.is_stream,
            "HTTP 错误响应"
        );

        let response_type = if context.is_stream { "sse" } else { "json" };

        TokenStatsManager::get()
            .log_failed_request(
                &context.tool_id,
                &context.session_id,
                &context.config_name,
                &context.client_ip,
                &context.request_body,
                "upstream_error",
                &error_detail,
                response_type, // 根据请求体的 stream 字段判断
                context.response_time_ms,
            )
            .await
    }
}
