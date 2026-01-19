//! Token 日志记录器模块
//!
//! 负责将 Token 信息记录到日志，各工具独立实现

mod claude;
mod codex;
mod types;

pub use claude::ClaudeLogger;
pub use codex::CodexLogger;
pub use types::{LogStatus, ResponseType};

use crate::models::token_stats::TokenLog;
use anyhow::{anyhow, Result};

/// 工具日志记录器 - 负责将 Token 信息记录到日志
pub trait TokenLogger: Send + Sync {
    /// 工具 ID
    fn tool_id(&self) -> &str;

    /// 记录 SSE 响应日志
    ///
    /// # 参数
    /// - `request_body`: 请求体（用于提取 model）
    /// - `sse_chunks`: SSE 数据行（Vec<String>）
    /// - `session_id`: 会话 ID
    /// - `config_name`: 配置名称
    /// - `client_ip`: 客户端 IP
    /// - `response_time_ms`: 响应时间（毫秒）
    ///
    /// # 返回
    /// - TokenLog: 日志记录对象
    fn log_sse_response(
        &self,
        request_body: &[u8],
        sse_chunks: Vec<String>,
        session_id: String,
        config_name: String,
        client_ip: String,
        response_time_ms: Option<i64>,
    ) -> Result<TokenLog>;

    /// 记录 JSON 响应日志
    ///
    /// # 参数
    /// - `request_body`: 请求体（用于提取 model，如果响应中没有）
    /// - `json`: JSON 响应体
    /// - `session_id`: 会话 ID
    /// - `config_name`: 配置名称
    /// - `client_ip`: 客户端 IP
    /// - `response_time_ms`: 响应时间（毫秒）
    ///
    /// # 返回
    /// - TokenLog: 日志记录对象
    fn log_json_response(
        &self,
        request_body: &[u8],
        json: &serde_json::Value,
        session_id: String,
        config_name: String,
        client_ip: String,
        response_time_ms: Option<i64>,
    ) -> Result<TokenLog>;

    /// 记录失败请求日志
    ///
    /// # 参数
    /// - `request_body`: 请求体（用于提取 model）
    /// - `session_id`: 会话 ID
    /// - `config_name`: 配置名称
    /// - `client_ip`: 客户端 IP
    /// - `response_time_ms`: 响应时间（毫秒）
    /// - `error_type`: 错误类型（如 "network_error", "api_error"）
    /// - `error_detail`: 错误详情
    ///
    /// # 返回
    /// - TokenLog: 日志记录对象
    fn log_failed_request(
        &self,
        request_body: &[u8],
        session_id: String,
        config_name: String,
        client_ip: String,
        response_time_ms: Option<i64>,
        error_type: String,
        error_detail: String,
    ) -> Result<TokenLog>;
}

/// 创建工具日志记录器
///
/// # 参数
/// - `tool_id`: 工具标识（claude-code/codex）
///
/// # 返回
/// - Box<dyn TokenLogger>: 对应的日志记录器实例
pub fn create_logger(tool_id: &str) -> Result<Box<dyn TokenLogger>> {
    match tool_id {
        "claude-code" => Ok(Box::new(ClaudeLogger)),
        "codex" => Ok(Box::new(CodexLogger)),
        _ => Err(anyhow!("Unsupported tool: {}", tool_id)),
    }
}
