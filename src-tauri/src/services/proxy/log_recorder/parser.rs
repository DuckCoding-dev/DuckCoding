// 响应解析层
//
// 职责：安全解析响应数据，区分 SSE 流式和 JSON 非流式，永不 panic

use serde_json::Value;

/// 解析后的响应数据
#[derive(Debug)]
pub enum ParsedResponse {
    /// SSE 流式响应（已提取的 data 块）
    Sse { data_lines: Vec<String> },
    /// JSON 响应（已解析的 JSON）
    Json { data: Value },
    /// 空响应（上游失败或连接中断）
    Empty,
    /// 解析失败（保留原始字节和错误信息）
    ParseError {
        raw_bytes: Vec<u8>,
        error: String,
        response_type: &'static str, // "sse" 或 "json"
    },
}

pub struct ResponseParser;

impl ResponseParser {
    /// 安全解析响应（区分 SSE/JSON，永不 panic）
    pub fn parse(response_body: &[u8], response_status: u16, is_sse: bool) -> ParsedResponse {
        // 1. 检查空响应或无状态码
        if response_body.is_empty() || response_status == 0 {
            return ParsedResponse::Empty;
        }

        // 2. 根据类型解析
        if is_sse {
            Self::parse_sse(response_body)
        } else {
            Self::parse_json(response_body)
        }
    }

    /// 解析 SSE 流式响应
    ///
    /// SSE 格式示例：
    /// ```
    /// data: {"type":"message_start","message":{...}}
    ///
    /// data: {"type":"content_block_delta",...}
    ///
    /// data: {"type":"message_delta","delta":{...},"usage":{...}}
    /// ```
    fn parse_sse(response_body: &[u8]) -> ParsedResponse {
        let body_str = String::from_utf8_lossy(response_body);
        let data_lines: Vec<String> = body_str
            .lines()
            .filter(|line| line.starts_with("data: "))
            .map(|line| line.trim_start_matches("data: ").to_string())
            .filter(|line| !line.is_empty() && line != "[DONE]") // 过滤空行和结束标记
            .collect();

        if data_lines.is_empty() {
            // SSE 流为空或仅包含无效数据
            return ParsedResponse::ParseError {
                raw_bytes: response_body.to_vec(),
                error: "SSE 流不包含有效的 data 块".to_string(),
                response_type: "sse",
            };
        }

        ParsedResponse::Sse { data_lines }
    }

    /// 解析 JSON 响应
    fn parse_json(response_body: &[u8]) -> ParsedResponse {
        match serde_json::from_slice::<Value>(response_body) {
            Ok(data) => ParsedResponse::Json { data },
            Err(e) => ParsedResponse::ParseError {
                raw_bytes: response_body.to_vec(),
                error: e.to_string(),
                response_type: "json",
            },
        }
    }
}
