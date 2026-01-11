// 请求上下文提取层
//
// 职责：在请求处理早期一次性提取所有必要信息，避免重复解析

use crate::services::session::manager::SESSION_MANAGER;
use crate::services::session::models::ProxySession;
use std::time::Instant;

/// 请求日志上下文（在请求处理早期提取）
#[derive(Debug, Clone)]
pub struct RequestLogContext {
    pub tool_id: String,
    pub session_id: String, // 从 request_body 提取
    pub config_name: String,
    pub client_ip: String,
    pub pricing_template_id: Option<String>, // 会话级 > 代理级
    pub model: Option<String>,               // 从 request_body 提取
    pub is_stream: bool,                     // 从 request_body 提取 stream 字段
    pub request_body: Vec<u8>,               // 保留原始请求体
    pub start_time: Instant,
}

impl RequestLogContext {
    /// 从请求创建上下文（早期提取，仅解析一次）
    pub fn from_request(
        tool_id: &str,
        config_name: &str,
        client_ip: &str,
        proxy_pricing_template_id: Option<&str>,
        request_body: &[u8],
    ) -> Self {
        // 提取 session_id、model 和 stream（仅解析一次）
        let (session_id, model, is_stream) = if !request_body.is_empty() {
            match serde_json::from_slice::<serde_json::Value>(request_body) {
                Ok(json) => {
                    let session_id = json["metadata"]["user_id"]
                        .as_str()
                        .and_then(ProxySession::extract_display_id)
                        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                    let model = json["model"].as_str().map(|s| s.to_string());
                    let is_stream = json["stream"].as_bool().unwrap_or(false);
                    (session_id, model, is_stream)
                }
                Err(_) => (uuid::Uuid::new_v4().to_string(), None, false),
            }
        } else {
            (uuid::Uuid::new_v4().to_string(), None, false)
        };

        // 查询会话级别的 pricing_template_id（优先级：会话 > 代理）
        let pricing_template_id =
            Self::resolve_pricing_template_id(&session_id, proxy_pricing_template_id);

        Self {
            tool_id: tool_id.to_string(),
            session_id,
            config_name: config_name.to_string(),
            client_ip: client_ip.to_string(),
            pricing_template_id,
            model,
            is_stream,
            request_body: request_body.to_vec(),
            start_time: Instant::now(),
        }
    }

    fn resolve_pricing_template_id(
        session_id: &str,
        proxy_template_id: Option<&str>,
    ) -> Option<String> {
        // 优先级：会话配置 > 代理配置
        SESSION_MANAGER
            .get_session_config(session_id)
            .ok()
            .flatten()
            .and_then(|(_, _, _, template_id)| template_id)
            .or_else(|| proxy_template_id.map(|s| s.to_string()))
    }

    pub fn elapsed_ms(&self) -> i64 {
        self.start_time.elapsed().as_millis() as i64
    }
}
