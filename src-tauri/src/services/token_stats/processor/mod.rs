//! Token 处理器模块
//!
//! 负责从原始响应中提取 Token 信息，各工具独立实现

mod claude;
mod codex;
mod token_info;

pub use claude::ClaudeProcessor;
pub use codex::CodexProcessor;
pub use token_info::TokenInfo;

use anyhow::{anyhow, Result};
use serde_json::Value;

/// 工具处理器 - 负责从原始响应中提取 Token 信息
pub trait ToolProcessor: Send + Sync {
    /// 工具 ID
    fn tool_id(&self) -> &str;

    /// 从 SSE 响应中提取 Token 信息（完整流程）
    ///
    /// # 参数
    /// - `request_body`: 请求体（用于提取 model）
    /// - `sse_chunks`: SSE 数据行（Vec<String>）
    ///
    /// # 返回
    /// - TokenInfo: 完整的 Token 统计信息
    fn process_sse_response(
        &self,
        request_body: &[u8],
        sse_chunks: Vec<String>,
    ) -> Result<TokenInfo>;

    /// 从 JSON 响应中提取 Token 信息
    ///
    /// # 参数
    /// - `request_body`: 请求体（用于提取 model，如果响应中没有）
    /// - `json`: JSON 响应体
    ///
    /// # 返回
    /// - TokenInfo: 完整的 Token 统计信息
    fn process_json_response(&self, request_body: &[u8], json: &Value) -> Result<TokenInfo>;
}

/// 创建工具处理器
///
/// # 参数
/// - `tool_id`: 工具标识（claude-code/codex）
///
/// # 返回
/// - Box<dyn ToolProcessor>: 对应的处理器实例
pub fn create_processor(tool_id: &str) -> Result<Box<dyn ToolProcessor>> {
    match tool_id {
        "claude-code" => Ok(Box::new(ClaudeProcessor)),
        "codex" => Ok(Box::new(CodexProcessor)),
        _ => Err(anyhow!("Unsupported tool: {}", tool_id)),
    }
}
