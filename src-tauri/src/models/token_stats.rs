use serde::{Deserialize, Serialize};

/// Token日志记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenLog {
    /// 主键ID（自增）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,

    /// 工具类型：claude_code, codex, gemini_cli
    pub tool_type: String,

    /// 请求时间戳（毫秒）
    pub timestamp: i64,

    /// 客户端IP地址
    pub client_ip: String,

    /// 会话ID
    pub session_id: String,

    /// 使用的配置名称
    pub config_name: String,

    /// 模型名称
    pub model: String,

    /// API返回的消息ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,

    /// 输入Token数量
    pub input_tokens: i64,

    /// 输出Token数量
    pub output_tokens: i64,

    /// 缓存创建Token数量
    pub cache_creation_tokens: i64,

    /// 缓存读取Token数量
    pub cache_read_tokens: i64,

    /// 请求状态：success, failed
    pub request_status: String,

    /// 响应类型：sse, json, unknown
    pub response_type: String,

    /// 错误类型：parse_error, request_interrupted, upstream_error（成功时为None）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,

    /// 错误详情（成功时为None）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_detail: Option<String>,

    /// 响应时间（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_time_ms: Option<i64>,

    /// 输入部分价格（USD）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "crate::utils::precision::option_price_precision")]
    pub input_price: Option<f64>,

    /// 输出部分价格（USD）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "crate::utils::precision::option_price_precision")]
    pub output_price: Option<f64>,

    /// 缓存写入部分价格（USD）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "crate::utils::precision::option_price_precision")]
    pub cache_write_price: Option<f64>,

    /// 缓存读取部分价格（USD）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "crate::utils::precision::option_price_precision")]
    pub cache_read_price: Option<f64>,

    /// 总成本（USD）
    #[serde(default)]
    #[serde(with = "crate::utils::precision::price_precision")]
    pub total_cost: f64,

    /// 使用的价格模板ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing_template_id: Option<String>,
}

impl TokenLog {
    /// 创建新的Token日志记录
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tool_type: String,
        timestamp: i64,
        client_ip: String,
        session_id: String,
        config_name: String,
        model: String,
        message_id: Option<String>,
        input_tokens: i64,
        output_tokens: i64,
        cache_creation_tokens: i64,
        cache_read_tokens: i64,
        request_status: String,
        response_type: String,
        error_type: Option<String>,
        error_detail: Option<String>,
        response_time_ms: Option<i64>,
        input_price: Option<f64>,
        output_price: Option<f64>,
        cache_write_price: Option<f64>,
        cache_read_price: Option<f64>,
        total_cost: f64,
        pricing_template_id: Option<String>,
    ) -> Self {
        Self {
            id: None,
            tool_type,
            timestamp,
            client_ip,
            session_id,
            config_name,
            model,
            message_id,
            input_tokens,
            output_tokens,
            cache_creation_tokens,
            cache_read_tokens,
            request_status,
            response_type,
            error_type,
            error_detail,
            response_time_ms,
            input_price,
            output_price,
            cache_write_price,
            cache_read_price,
            total_cost,
            pricing_template_id,
        }
    }

    /// 计算总Token数量
    pub fn total_tokens(&self) -> i64 {
        self.input_tokens + self.output_tokens
    }

    /// 计算总缓存Token数量
    pub fn total_cache_tokens(&self) -> i64 {
        self.cache_creation_tokens + self.cache_read_tokens
    }

    /// 是否成功
    pub fn is_success(&self) -> bool {
        self.request_status == "success"
    }
}

/// 会话统计数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    /// 总输入Token数量
    pub total_input: i64,

    /// 总输出Token数量
    pub total_output: i64,

    /// 总缓存创建Token数量
    pub total_cache_creation: i64,

    /// 总缓存读取Token数量
    pub total_cache_read: i64,

    /// 请求总数
    pub request_count: i64,
}

impl SessionStats {
    /// 创建空的统计数据
    pub fn empty() -> Self {
        Self {
            total_input: 0,
            total_output: 0,
            total_cache_creation: 0,
            total_cache_read: 0,
            request_count: 0,
        }
    }

    /// 计算总Token数量
    pub fn total_tokens(&self) -> i64 {
        self.total_input + self.total_output
    }

    /// 计算总缓存Token数量
    pub fn total_cache_tokens(&self) -> i64 {
        self.total_cache_creation + self.total_cache_read
    }
}

/// Token日志查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStatsQuery {
    /// 工具类型筛选
    pub tool_type: Option<String>,

    /// 会话ID筛选
    pub session_id: Option<String>,

    /// 配置名称筛选
    pub config_name: Option<String>,

    /// 开始时间戳（毫秒）
    pub start_time: Option<i64>,

    /// 结束时间戳（毫秒）
    pub end_time: Option<i64>,

    /// 分页：页码（从0开始）
    pub page: u32,

    /// 分页：每页大小
    pub page_size: u32,
}

impl Default for TokenStatsQuery {
    fn default() -> Self {
        Self {
            tool_type: None,
            session_id: None,
            config_name: None,
            start_time: None,
            end_time: None,
            page: 0,
            page_size: 20,
        }
    }
}

/// 分页查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenLogsPage {
    /// 日志列表
    pub logs: Vec<TokenLog>,

    /// 总记录数
    pub total: i64,

    /// 当前页码
    pub page: u32,

    /// 每页大小
    pub page_size: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_log_creation() {
        let log = TokenLog::new(
            "claude_code".to_string(),
            1700000000000,
            "127.0.0.1".to_string(),
            "session_123".to_string(),
            "default".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
            Some("msg_123".to_string()),
            1000,
            500,
            100,
            200,
            "success".to_string(),
            "sse".to_string(),
            None,
            None,
            Some(1500),
            Some(0.003),
            Some(0.0075),
            Some(0.000375),
            Some(0.00006),
            0.011235,
            Some("claude_official_2025_01".to_string()),
        );

        assert_eq!(log.tool_type, "claude_code");
        assert_eq!(log.total_tokens(), 1500);
        assert_eq!(log.total_cache_tokens(), 300);
        assert!(log.is_success());
        assert_eq!(log.response_time_ms, Some(1500));
        assert_eq!(log.total_cost, 0.011235);
        assert_eq!(
            log.pricing_template_id,
            Some("claude_official_2025_01".to_string())
        );
    }

    #[test]
    fn test_session_stats_calculation() {
        let stats = SessionStats {
            total_input: 10000,
            total_output: 5000,
            total_cache_creation: 1000,
            total_cache_read: 2000,
            request_count: 10,
        };

        assert_eq!(stats.total_tokens(), 15000);
        assert_eq!(stats.total_cache_tokens(), 3000);
    }

    #[test]
    fn test_query_default() {
        let query = TokenStatsQuery::default();
        assert_eq!(query.page, 0);
        assert_eq!(query.page_size, 20);
        assert!(query.tool_type.is_none());
    }
}
