//! Logger 类型定义
//!
//! 定义日志记录中使用的枚举类型

use serde::{Deserialize, Serialize};

/// 日志状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogStatus {
    /// 成功
    Success,
    /// 失败
    Failed,
    /// 部分成功（已提取部分 Token 信息）
    Partial,
}

impl LogStatus {
    /// 转换为字符串（用于数据库存储）
    pub fn as_str(&self) -> &'static str {
        match self {
            LogStatus::Success => "success",
            LogStatus::Failed => "failed",
            LogStatus::Partial => "partial",
        }
    }

    /// 从字符串解析
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "success" => LogStatus::Success,
            "failed" => LogStatus::Failed,
            "partial" => LogStatus::Partial,
            _ => LogStatus::Failed,
        }
    }
}

/// 响应类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResponseType {
    /// SSE 流式响应
    Sse,
    /// JSON 响应
    Json,
    /// 未知类型
    Unknown,
}

impl ResponseType {
    /// 转换为字符串（用于数据库存储）
    pub fn as_str(&self) -> &'static str {
        match self {
            ResponseType::Sse => "sse",
            ResponseType::Json => "json",
            ResponseType::Unknown => "unknown",
        }
    }

    /// 从字符串解析
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "sse" => ResponseType::Sse,
            "json" => ResponseType::Json,
            _ => ResponseType::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_status_as_str() {
        assert_eq!(LogStatus::Success.as_str(), "success");
        assert_eq!(LogStatus::Failed.as_str(), "failed");
        assert_eq!(LogStatus::Partial.as_str(), "partial");
    }

    #[test]
    fn test_log_status_from_str() {
        assert_eq!(LogStatus::from_str("success"), LogStatus::Success);
        assert_eq!(LogStatus::from_str("failed"), LogStatus::Failed);
        assert_eq!(LogStatus::from_str("partial"), LogStatus::Partial);
        assert_eq!(LogStatus::from_str("unknown"), LogStatus::Failed); // 回退
    }

    #[test]
    fn test_response_type_as_str() {
        assert_eq!(ResponseType::Sse.as_str(), "sse");
        assert_eq!(ResponseType::Json.as_str(), "json");
        assert_eq!(ResponseType::Unknown.as_str(), "unknown");
    }

    #[test]
    fn test_response_type_from_str() {
        assert_eq!(ResponseType::from_str("sse"), ResponseType::Sse);
        assert_eq!(ResponseType::from_str("json"), ResponseType::Json);
        assert_eq!(ResponseType::from_str("xyz"), ResponseType::Unknown); // 回退
    }
}
