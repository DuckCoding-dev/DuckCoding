// 余额查询相关命令
//
// 支持通过自定义 API 端点和提取器脚本查询余额信息

use ::duckcoding::http_client::build_client;
use ::duckcoding::utils::config::apply_proxy_if_configured;
use std::collections::HashMap;

/// Tauri command: 通用 API 请求
///
/// # 参数
/// - `endpoint`: API 端点 URL
/// - `method`: HTTP 方法 (GET 或 POST)
/// - `headers`: 请求头 (包含 Authorization 等)
/// - `timeout_ms`: 可选的请求超时时间(毫秒)
///
/// # 返回
/// 返回原始 JSON 响应，由前端执行 extractor 脚本提取余额信息
#[tauri::command]
pub async fn fetch_api(
    endpoint: String,
    method: String,
    headers: HashMap<String, String>,
    timeout_ms: Option<u64>,
) -> Result<serde_json::Value, String> {
    apply_proxy_if_configured();

    // 验证 HTTP 方法
    let method_normalized = method.to_uppercase();
    if method_normalized != "GET" && method_normalized != "POST" {
        return Err(format!("不支持的 HTTP 方法: {method}，仅支持 GET 和 POST"));
    }

    // 使用 build_client 确保代理配置等被应用
    let client = build_client().map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    // 构建请求
    let mut request_builder = if method_normalized == "GET" {
        client.get(&endpoint)
    } else {
        client.post(&endpoint)
    };

    // 添加请求头
    for (key, value) in headers {
        request_builder = request_builder.header(key, value);
    }

    // 应用自定义超时
    if let Some(ms) = timeout_ms {
        request_builder = request_builder.timeout(std::time::Duration::from_millis(ms));
    }

    // 发送请求
    let response = request_builder
        .send()
        .await
        .map_err(|e| format!("请求 API 失败: {e}"))?;

    // 检查响应状态
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("API 请求失败 ({status}): {error_text}"));
    }

    // 解析为 JSON
    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析响应 JSON 失败: {e}"))?;

    Ok(data)
}
