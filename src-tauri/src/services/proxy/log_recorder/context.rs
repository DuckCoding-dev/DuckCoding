// 请求上下文提取层
//
// 职责：在请求处理早期一次性提取所有必要信息，避免重复解析

use crate::services::session::manager::SESSION_MANAGER;
use crate::services::session::models::ProxySession;

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
    pub response_time_ms: Option<i64>,       // 响应时间（毫秒）
}

impl RequestLogContext {
    /// 从请求创建上下文（早期提取，仅解析一次）
    pub fn from_request(
        tool_id: &str,
        config_name: &str,
        client_ip: &str,
        proxy_pricing_template_id: Option<&str>,
        request_body: &[u8],
        response_time_ms: Option<i64>,
    ) -> Self {
        // 提取 user_id（完整）、display_id（用于日志）、model 和 stream（仅解析一次）
        let (user_id, session_id, model, is_stream) = if !request_body.is_empty() {
            match serde_json::from_slice::<serde_json::Value>(request_body) {
                Ok(json) => {
                    // 提取完整 user_id（用于查询配置）
                    let user_id = json["metadata"]["user_id"]
                        .as_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

                    // 提取 display_id（用于存储日志）
                    let session_id = ProxySession::extract_display_id(&user_id)
                        .unwrap_or_else(|| user_id.clone());

                    let model = json["model"].as_str().map(|s| s.to_string());
                    let is_stream = json["stream"].as_bool().unwrap_or(false);
                    (user_id, session_id, model, is_stream)
                }
                Err(_) => {
                    let fallback_id = uuid::Uuid::new_v4().to_string();
                    (fallback_id.clone(), fallback_id, None, false)
                }
            }
        } else {
            let fallback_id = uuid::Uuid::new_v4().to_string();
            (fallback_id.clone(), fallback_id, None, false)
        };

        // 查询会话级别的配置（优先级：会话 > 代理），使用完整 user_id 查询
        let (config_name, pricing_template_id) =
            Self::resolve_session_config(&user_id, config_name, proxy_pricing_template_id);

        Self {
            tool_id: tool_id.to_string(),
            session_id,
            config_name,
            client_ip: client_ip.to_string(),
            pricing_template_id,
            model,
            is_stream,
            request_body: request_body.to_vec(),
            response_time_ms,
        }
    }

    /// 解析会话级配置（同时提取 config_name 和 pricing_template_id）
    fn resolve_session_config(
        session_id: &str,
        proxy_config_name: &str,
        proxy_template_id: Option<&str>,
    ) -> (String, Option<String>) {
        // 查询会话配置
        if let Ok(Some((
            config_name,
            custom_profile_name,
            session_url,
            session_api_key,
            session_pricing_template_id,
        ))) = SESSION_MANAGER.get_session_config(session_id)
        {
            // 判断是否为自定义配置：config_name == "custom" 且 URL、API Key、pricing_template_id 都不为空
            if config_name == "custom"
                && !session_url.is_empty()
                && !session_api_key.is_empty()
                && session_pricing_template_id.is_some()
            {
                // 使用会话级配置：custom_profile_name 作为配置名（如果存在）
                let final_config_name = custom_profile_name.unwrap_or_else(|| "custom".to_string());
                return (final_config_name, session_pricing_template_id);
            }
        }

        // 回退到代理级配置（包括会话存在但不是自定义配置的情况）
        (
            proxy_config_name.to_string(),
            proxy_template_id.map(|s| s.to_string()),
        )
    }
}
