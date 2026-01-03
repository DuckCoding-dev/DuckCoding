// 统计相关命令
//
// 包含用量统计、用户额度查询等功能

use crate::commands::provider_commands::ProviderManagerState;
use ::duckcoding::services::proxy::config::apply_global_proxy;
use serde::Serialize;
use tauri::State;

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

fn build_reqwest_client() -> Result<reqwest::Client, String> {
    ::duckcoding::http_client::build_client()
}

#[tauri::command]
pub async fn get_usage_stats(
    provider_id: String,
    provider_state: State<'_, ProviderManagerState>,
) -> Result<UsageStatsResult, String> {
    apply_global_proxy().ok();

    // 根据 provider_id 获取供应商
    let providers = provider_state
        .manager
        .list_providers()
        .map_err(|e| format!("获取供应商列表失败: {}", e))?;

    let provider = providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| format!("未找到供应商: {}", provider_id))?;

    // 验证供应商凭证
    if provider.user_id.is_empty() || provider.access_token.is_empty() {
        return Err("请先配置供应商的用户ID和访问令牌".to_string());
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let beijing_offset = 8 * 3600;
    let today_end = (now + beijing_offset) / 86400 * 86400 + 86400 - beijing_offset;
    let start_timestamp = today_end - 30 * 86400;
    let end_timestamp = today_end;
    let client = build_reqwest_client().map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    // 使用供应商的 website_url
    let url = format!(
        "{}/api/data/self?start_timestamp={start_timestamp}&end_timestamp={end_timestamp}",
        provider.website_url.trim_end_matches('/')
    );

    let response = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .header("Accept", "application/json, text/plain, */*")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("Referer", &provider.website_url)
        .header("Origin", &provider.website_url)
        .header("Authorization", format!("Bearer {}", provider.access_token))
        .header("New-Api-User", &provider.user_id)
        .send()
        .await
        .map_err(|e| format!("获取用量统计失败: {e}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Ok(UsageStatsResult {
            success: false,
            message: format!("获取用量统计失败 ({status}): {error_text}"),
            data: vec![],
        });
    }
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_default();
    if !content_type.contains("application/json") {
        return Ok(UsageStatsResult {
            success: false,
            message: format!("服务器返回了非JSON格式的响应 (Content-Type: {content_type})"),
            data: vec![],
        });
    }
    let api_response: UsageApiResponse = response
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {e}"))?;
    if !api_response.success {
        return Ok(UsageStatsResult {
            success: false,
            message: format!("API返回错误: {}", api_response.message),
            data: vec![],
        });
    }
    Ok(UsageStatsResult {
        success: true,
        message: "获取成功".to_string(),
        data: api_response.data.unwrap_or_default(),
    })
}

#[tauri::command]
pub async fn get_user_quota(
    provider_id: String,
    provider_state: State<'_, ProviderManagerState>,
) -> Result<UserQuotaResult, String> {
    apply_global_proxy().ok();

    // 根据 provider_id 获取供应商
    let providers = provider_state
        .manager
        .list_providers()
        .map_err(|e| format!("获取供应商列表失败: {}", e))?;

    let provider = providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| format!("未找到供应商: {}", provider_id))?;

    // 验证供应商凭证
    if provider.user_id.is_empty() || provider.access_token.is_empty() {
        return Err("请先配置供应商的用户ID和访问令牌".to_string());
    }

    let client = build_reqwest_client().map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    // 使用供应商的 website_url
    let url = format!(
        "{}/api/user/self",
        provider.website_url.trim_end_matches('/')
    );

    let response = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .header("Accept", "application/json, text/plain, */*")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("Referer", &provider.website_url)
        .header("Origin", &provider.website_url)
        .header("Authorization", format!("Bearer {}", provider.access_token))
        .header("New-Api-User", &provider.user_id)
        .send()
        .await
        .map_err(|e| format!("获取用户信息失败: {e}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("获取用户信息失败 ({status}): {error_text}"));
    }
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_default();
    if !content_type.contains("application/json") {
        return Err(format!(
            "服务器返回了非JSON格式的响应 (Content-Type: {content_type})"
        ));
    }
    let api_response: UserApiResponse = response
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {e}"))?;
    if !api_response.success {
        return Err(format!("API返回错误: {}", api_response.message));
    }
    let user_info = api_response.data.ok_or("未获取到用户信息")?;
    let remaining_quota = user_info.quota as f64 / 500000.0;
    let used_quota = user_info.used_quota as f64 / 500000.0;
    let total_quota = remaining_quota + used_quota;
    Ok(UserQuotaResult {
        success: true,
        message: "获取成功".to_string(),
        total_quota,
        used_quota,
        remaining_quota,
        request_count: user_info.request_count,
    })
}
