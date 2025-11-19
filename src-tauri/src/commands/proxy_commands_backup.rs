// 统计相关命令
//
// 包含用量统计、用户额度查询等功能

use serde::Serialize;
use crate::GlobalConfig;

/// 用量统计数据结构
#[derive(serde::Deserialize, Serialize, Debug, Clone)]
pub struct UsageData {
    id: i64,
    user_id: i64,
    username: String,
    model_name: String,
    created_at: i64,
    token_used: i64,
    count: i64,
    quota: i64,
}

#[derive(serde::Deserialize, Debug)]
struct UsageApiResponse {
    success: bool,
    message: String,
    data: Option<Vec<UsageData>>,
}

#[derive(serde::Serialize)]
pub struct UsageStatsResult {
    success: bool,
    message: String,
    data: Vec<UsageData>,
}

#[derive(serde::Deserialize, Serialize, Debug)]
struct UserInfo {
    id: i64,
    username: String,
    quota: i64,
    used_quota: i64,
    request_count: i64,
}

#[derive(serde::Deserialize, Debug)]
struct UserApiResponse {
    success: bool,
    message: String,
    data: Option<UserInfo>,
}

#[derive(serde::Serialize)]
pub struct UserQuotaResult {
    success: bool,
    message: String,
    total_quota: f64,
    used_quota: f64,
    remaining_quota: f64,
    request_count: i64,
}

fn get_global_config_path() -> Result<std::path::PathBuf, String> {
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;
    let config_dir = home_dir.join(".duckcoding");
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    Ok(config_dir.join("config.json"))
}

async fn get_global_config() -> Result<Option<GlobalConfig>, String> {
    let config_path = get_global_config_path()?;
    if !config_path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    let config: GlobalConfig = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config: {}", e))?;
    Ok(Some(config))
}

async fn apply_proxy_if_configured() {
    if let Ok(Some(config)) = get_global_config().await {
        crate::ProxyService::apply_proxy_from_config(&config);
    }
}

fn build_reqwest_client() -> Result<reqwest::Client, String> {
    crate::http_client::build_client()
}

#[tauri::command]
pub async fn get_usage_stats() -> Result<UsageStatsResult, String> {
    apply_proxy_if_configured().await;
    let global_config = get_global_config().await?.ok_or("请先配置用户ID和系统访问令牌")?;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    let beijing_offset = 8 * 3600;
    let today_end = (now + beijing_offset) / 86400 * 86400 + 86400 - beijing_offset;
    let start_timestamp = today_end - 30 * 86400;
    let end_timestamp = today_end;
    let client = build_reqwest_client().map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;
    let url = format!("https://duckcoding.com/api/data/self?start_timestamp={}&end_timestamp={}", start_timestamp, end_timestamp);
    let response = client.get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .header("Accept", "application/json, text/plain, */*")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("Referer", "https://duckcoding.com/")
        .header("Origin", "https://duckcoding.com")
        .header("Authorization", format!("Bearer {}", global_config.system_token))
        .header("New-Api-User", &global_config.user_id)
        .send().await.map_err(|e| format!("获取用量统计失败: {}", e))?;
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Ok(UsageStatsResult { success: false, message: format!("获取用量统计失败 ({}): {}", status, error_text), data: vec![] });
    }
    let content_type = response.headers().get("content-type").and_then(|v| v.to_str().ok()).map(|s| s.to_string()).unwrap_or_default();
    if !content_type.contains("application/json") {
        return Ok(UsageStatsResult { success: false, message: format!("服务器返回了非JSON格式的响应 (Content-Type: {})", content_type), data: vec![] });
    }
    let api_response: UsageApiResponse = response.json().await.map_err(|e| format!("解析响应失败: {}", e))?;
    if !api_response.success {
        return Ok(UsageStatsResult { success: false, message: format!("API返回错误: {}", api_response.message), data: vec![] });
    }
    Ok(UsageStatsResult { success: true, message: "获取成功".to_string(), data: api_response.data.unwrap_or_default() })
}

#[tauri::command]
pub async fn get_user_quota() -> Result<UserQuotaResult, String> {
    apply_proxy_if_configured().await;
    let global_config = get_global_config().await?.ok_or("请先配置用户ID和系统访问令牌")?;
    let client = build_reqwest_client().map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;
    let url = "https://duckcoding.com/api/user/self";
    let response = client.get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .header("Accept", "application/json, text/plain, */*")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("Referer", "https://duckcoding.com/")
        .header("Origin", "https://duckcoding.com")
        .header("Authorization", format!("Bearer {}", global_config.system_token))
        .header("New-Api-User", &global_config.user_id)
        .send().await.map_err(|e| format!("获取用户信息失败: {}", e))?;
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("获取用户信息失败 ({}): {}", status, error_text));
    }
    let content_type = response.headers().get("content-type").and_then(|v| v.to_str().ok()).map(|s| s.to_string()).unwrap_or_default();
    if !content_type.contains("application/json") {
        return Err(format!("服务器返回了非JSON格式的响应 (Content-Type: {})", content_type));
    }
    let api_response: UserApiResponse = response.json().await.map_err(|e| format!("解析响应失败: {}", e))?;
    if !api_response.success {
        return Err(format!("API返回错误: {}", api_response.message));
    }
    let user_info = api_response.data.ok_or("未获取到用户信息")?;
    let remaining_quota = user_info.quota as f64 / 500000.0;
    let used_quota = user_info.used_quota as f64 / 500000.0;
    let total_quota = remaining_quota + used_quota;
    Ok(UserQuotaResult { success: true, message: "获取成功".to_string(), total_quota, used_quota, remaining_quota, request_count: user_info.request_count })
}
