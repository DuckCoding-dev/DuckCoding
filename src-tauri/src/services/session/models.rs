// 会话数据模型和事件定义

use serde::{Deserialize, Serialize};

/// 代理会话记录（数据库模型）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxySession {
    /// 完整会话ID（主键，来自 metadata.user_id）
    pub session_id: String,
    /// 显示ID（_session_ 后的 UUID 部分）
    pub display_id: String,
    /// 工具ID（"claude-code", "codex", "gemini-cli"）
    pub tool_id: String,
    /// 配置名称（"global" 或 "custom"）
    pub config_name: String,
    /// 自定义配置名称（config_name 为 custom 时记录）
    pub custom_profile_name: Option<String>,
    /// API Base URL（当 config_name 为 custom 时使用）
    pub url: String,
    /// API Key（当 config_name 为 custom 时使用）
    pub api_key: String,
    /// 会话备注
    pub note: Option<String>,
    /// 首次记录时间（Unix 时间戳，秒）
    pub first_seen_at: i64,
    /// 最后活跃时间（Unix 时间戳，秒）
    pub last_seen_at: i64,
    /// 请求次数
    pub request_count: i32,
    /// 创建时间（Unix 时间戳，秒）
    pub created_at: i64,
    /// 更新时间（Unix 时间戳，秒）
    pub updated_at: i64,
    /// 价格模板 ID（用于成本计算）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pricing_template_id: Option<String>,
}

/// 会话事件（异步队列传递）
#[derive(Debug, Clone)]
pub enum SessionEvent {
    /// 新请求事件
    NewRequest {
        session_id: String,
        tool_id: String,
        timestamp: i64,
    },
}

/// 会话列表响应
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionListResponse {
    /// 会话列表
    pub sessions: Vec<ProxySession>,
    /// 总数
    pub total: usize,
    /// 当前页
    pub page: usize,
    /// 每页大小
    pub page_size: usize,
}

impl ProxySession {
    /// 从 session_id 提取 display_id
    /// - Claude 格式：user_xxx_session_<uuid> → 提取 UUID
    /// - Codex 格式：prompt_cache_key → 使用前 12 字符
    pub fn extract_display_id(session_id: &str) -> String {
        // Claude 格式：提取 _session_ 后的 UUID
        if let Some(uuid) = session_id.split("_session_").nth(1) {
            return uuid.to_string();
        }
        // Codex/其他格式：使用前 12 字符或完整 ID
        if session_id.len() <= 12 {
            session_id.to_string()
        } else {
            session_id[..12].to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_display_id_claude() {
        let user_id = "user_42a8ca527d882b0a9b60e27011856f2018295786e9c6dc09cefbb7e0caba49ab_account__session_f7aa73fc-73a9-4148-ba8b-1b9f4aa5ebc3";
        let display_id = ProxySession::extract_display_id(user_id);
        assert_eq!(display_id, "f7aa73fc-73a9-4148-ba8b-1b9f4aa5ebc3");
    }

    #[test]
    fn test_extract_display_id_codex() {
        let session_id = "abc123def456ghi789";
        let display_id = ProxySession::extract_display_id(session_id);
        assert_eq!(display_id, "abc123def456");
    }

    #[test]
    fn test_extract_display_id_short() {
        let session_id = "short";
        let display_id = ProxySession::extract_display_id(session_id);
        assert_eq!(display_id, "short");
    }
}
