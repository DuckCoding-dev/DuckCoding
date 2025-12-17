//! 代理错误响应模板
//!
//! 统一的 JSON 错误格式和响应构建

use bytes::Bytes;
use hyper::{Response, StatusCode};

use super::body::{box_body, BoxBody};

/// 配置缺失错误
pub fn configuration_missing(tool_id: &str) -> Response<BoxBody> {
    Response::builder()
        .status(StatusCode::BAD_GATEWAY)
        .header("content-type", "application/json")
        .body(box_body(http_body_util::Full::new(Bytes::from(format!(
            r#"{{
  "error": "CONFIGURATION_MISSING",
  "message": "{tool_id} 透明代理配置不完整",
  "details": "请先配置有效的 API Key 和 Base URL"
}}"#
        )))))
        .unwrap()
}

/// 代理回环错误
pub fn proxy_loop_detected(tool_id: &str) -> Response<BoxBody> {
    Response::builder()
        .status(StatusCode::BAD_GATEWAY)
        .header("content-type", "application/json")
        .body(box_body(http_body_util::Full::new(Bytes::from(format!(
            r#"{{
  "error": "PROXY_LOOP_DETECTED",
  "message": "{tool_id} 透明代理配置错误导致回环",
  "details": "请检查代理配置，确保 Base URL 不指向本地代理端口"
}}"#
        )))))
        .unwrap()
}

/// 未授权错误
pub fn unauthorized() -> Response<BoxBody> {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .body(box_body(http_body_util::Full::new(Bytes::from(
            "Unauthorized: Invalid API Key",
        ))))
        .unwrap()
}

/// 内部错误
pub fn internal_error(message: &str) -> Response<BoxBody> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(box_body(http_body_util::Full::new(Bytes::from(format!(
            "代理错误: {}",
            message
        )))))
        .unwrap()
}
