// Headers 处理器模块 - 为不同工具提供独立的请求处理逻辑

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use hyper::HeaderMap as HyperHeaderMap;
use reqwest::header::HeaderMap as ReqwestHeaderMap;

mod claude_processor;
mod codex_processor;
mod gemini_processor;

pub use claude_processor::ClaudeHeadersProcessor;
pub use codex_processor::CodexHeadersProcessor;
pub use gemini_processor::GeminiHeadersProcessor;

/// 处理后的请求信息
#[derive(Debug)]
pub struct ProcessedRequest {
    /// 目标 URL（完整 URL，包含 base_url + path + query）
    pub target_url: String,
    /// 处理后的请求 headers
    pub headers: ReqwestHeaderMap,
    /// 处理后的请求体（大多数情况下与原始 body 相同）
    pub body: Bytes,
}

/// 请求处理器 trait
///
/// 为不同的 AI 编程工具提供独立的请求处理逻辑。
/// 每个工具可能有不同的 URL 路径规则、认证方式、必需的 headers、或特殊的协议要求。
///
/// 该 trait 封装了从原始请求到上游请求的完整转换过程：
/// - 构建目标 URL（处理工具特定的路径规则）
/// - 处理认证信息（替换/添加 API Key）
/// - 处理其他 headers（添加/修改/删除）
/// - 处理请求体（如需要签名等特殊处理）
#[async_trait]
pub trait RequestProcessor: Send + Sync {
    /// 返回工具标识符
    fn tool_id(&self) -> &str;

    /// 处理出站请求（转发到上游前的完整处理）
    ///
    /// # 参数
    /// - `base_url`: 目标服务的基础 URL（可能包含或不包含 /v1 等路径）
    /// - `api_key`: 目标 API 的真实密钥
    /// - `path`: 原始请求路径（如 "/v1/messages"）
    /// - `query`: 可选的查询字符串（不包含 "?" 前缀）
    /// - `original_headers`: 客户端发送的原始 headers
    /// - `body`: 请求体字节数组
    ///
    /// # 返回
    /// - `Ok(ProcessedRequest)`: 处理成功，包含目标 URL、headers 和 body
    /// - `Err`: 处理失败（会中断请求）
    async fn process_outgoing_request(
        &self,
        base_url: &str,
        api_key: &str,
        path: &str,
        query: Option<&str>,
        original_headers: &HyperHeaderMap,
        body: &[u8],
    ) -> Result<ProcessedRequest>;

    /// 处理响应 headers（返回给客户端前调用，可选）
    ///
    /// # 参数
    /// - `headers`: 可修改的响应 headers
    /// - `body`: 响应体字节数组（如果已读取）
    ///
    /// # 默认实现
    /// 默认不处理响应 headers，直接透传
    async fn process_response(
        &self,
        _headers: &mut HyperHeaderMap,
        _body: Option<&[u8]>,
    ) -> Result<()> {
        Ok(())
    }

    /// 是否需要读取响应体
    ///
    /// 如果返回 `true`，代理服务会先读取完整响应体，
    /// 然后调用 `process_response`。
    ///
    /// # 注意
    /// 启用此选项会增加内存使用和延迟，仅在必要时启用。
    fn should_process_response(&self) -> bool {
        false
    }
}

/// 创建请求处理器工厂函数
///
/// # 参数
/// - `tool_id`: 工具标识符 ("claude-code", "codex", "gemini-cli")
///
/// # 返回
/// - 对应工具的 RequestProcessor 实例
///
/// # Panics
/// 当 `tool_id` 不被支持时会 panic
pub fn create_request_processor(tool_id: &str) -> Box<dyn RequestProcessor> {
    match tool_id {
        "claude-code" => Box::new(ClaudeHeadersProcessor),
        "codex" => Box::new(CodexHeadersProcessor),
        "gemini-cli" => Box::new(GeminiHeadersProcessor),
        _ => panic!("Unsupported tool: {tool_id}"),
    }
}

/// 旧工厂函数名称（向后兼容，已弃用）
#[deprecated(since = "0.1.0", note = "请使用 create_request_processor")]
pub fn create_headers_processor(tool_id: &str) -> Box<dyn RequestProcessor> {
    create_request_processor(tool_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_request_processor() {
        let claude = create_request_processor("claude-code");
        assert_eq!(claude.tool_id(), "claude-code");

        let codex = create_request_processor("codex");
        assert_eq!(codex.tool_id(), "codex");

        let gemini = create_request_processor("gemini-cli");
        assert_eq!(gemini.tool_id(), "gemini-cli");
    }

    #[test]
    #[should_panic(expected = "Unsupported tool")]
    fn test_create_invalid_processor() {
        create_request_processor("invalid-tool");
    }

    #[tokio::test]
    async fn test_claude_processor_basic() {
        let processor = ClaudeHeadersProcessor;
        let headers = HyperHeaderMap::new();

        let result = processor
            .process_outgoing_request(
                "https://api.anthropic.com",
                "test-api-key",
                "/v1/messages",
                None,
                &headers,
                b"",
            )
            .await
            .unwrap();

        assert_eq!(result.target_url, "https://api.anthropic.com/v1/messages");
        let auth_header = result
            .headers
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(auth_header, "Bearer test-api-key");
    }
}
