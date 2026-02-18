//! Token 信息统一输出格式
//!
//! 所有工具处理器的统一返回值

use serde::{Deserialize, Serialize};

/// Token 信息（统一输出格式）
///
/// 各工具处理器从响应中提取信息后统一返回此结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    /// 模型名称
    pub model: String,

    /// 消息 ID
    pub message_id: String,

    /// 输入 Token 数量
    pub input_tokens: i64,

    /// 输出 Token 数量
    pub output_tokens: i64,

    /// 缓存创建 Token 数量（5m + 1h 总量）
    pub cache_creation_tokens: i64,

    /// 1小时缓存创建 Token 数量（5m 部分 = cache_creation_tokens - cache_creation_1h_tokens）
    pub cache_creation_1h_tokens: i64,

    /// 缓存读取 Token 数量
    pub cache_read_tokens: i64,

    /// 推理 Token 数量
    pub reasoning_tokens: i64,
}

impl TokenInfo {
    /// 创建新的 TokenInfo 实例
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        model: String,
        message_id: String,
        input_tokens: i64,
        output_tokens: i64,
        cache_creation_tokens: i64,
        cache_creation_1h_tokens: i64,
        cache_read_tokens: i64,
        reasoning_tokens: i64,
    ) -> Self {
        Self {
            model,
            message_id,
            input_tokens,
            output_tokens,
            cache_creation_tokens,
            cache_creation_1h_tokens,
            cache_read_tokens,
            reasoning_tokens,
        }
    }

    /// 计算总 Token 数量
    pub fn total_tokens(&self) -> i64 {
        self.input_tokens
            + self.output_tokens
            + self.cache_creation_tokens
            + self.cache_read_tokens
            + self.reasoning_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_info_creation() {
        let info = TokenInfo::new(
            "claude-sonnet-4-5-20250929".to_string(),
            "msg_123".to_string(),
            1000,
            500,
            100, // total cache creation (5m + 1h)
            30,  // 1h cache creation
            200,
            50,
        );

        assert_eq!(info.model, "claude-sonnet-4-5-20250929");
        assert_eq!(info.message_id, "msg_123");
        assert_eq!(info.input_tokens, 1000);
        assert_eq!(info.output_tokens, 500);
        assert_eq!(info.cache_creation_tokens, 100);
        assert_eq!(info.cache_creation_1h_tokens, 30);
        assert_eq!(info.cache_read_tokens, 200);
        assert_eq!(info.reasoning_tokens, 50);
    }

    #[test]
    fn test_total_tokens() {
        let info = TokenInfo::new(
            "test-model".to_string(),
            "msg_test".to_string(),
            1000,
            500,
            100,
            0, // no 1h cache
            200,
            50,
        );

        assert_eq!(info.total_tokens(), 1850);
    }
}
