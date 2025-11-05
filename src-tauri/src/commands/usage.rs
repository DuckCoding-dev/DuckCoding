use std::time::Duration;

use crate::commands::config_ops::load_global_config;
use crate::error::{AppError, AppResult};
use crate::models::{
    ApiResponse, GenerateApiKeyResult, UsageApiResponse, UsageStatsResult, UserApiResponse,
    UserQuotaResult,
};

#[tauri::command]
pub async fn generate_api_key_for_tool(tool: String) -> Result<GenerateApiKeyResult, String> {
    generate_api_key_impl(tool).await.map_err(|e| e.to_string())
}

async fn generate_api_key_impl(tool: String) -> AppResult<GenerateApiKeyResult> {
    let global_config =
        load_global_config()?.ok_or_else(|| AppError::config("请先配置用户ID和系统访问令牌"))?;

    let (name, group) = match tool.as_str() {
        "claude-code" => ("Claude Code一键创建", "Claude Code专用"),
        "codex" => ("CodeX一键创建", "CodeX专用"),
        "gemini-cli" => ("Gemini CLI一键创建", "Gemini CLI专用"),
        _ => return Err(AppError::config(format!("Unknown tool: {}", tool))),
    };

    let client = reqwest::Client::new();
    let create_url = "https://duckcoding.com/api/token";

    let create_body = serde_json::json!({
        "remain_quota": 500000,
        "expired_time": -1,
        "unlimited_quota": true,
        "model_limits_enabled": false,
        "model_limits": "",
        "name": name,
        "group": group,
        "allow_ips": ""
    });

    let create_response = client
        .post(create_url)
        .header(
            "Authorization",
            format!("Bearer {}", global_config.system_token),
        )
        .header("New-Api-User", &global_config.user_id)
        .header("Content-Type", "application/json")
        .json(&create_body)
        .send()
        .await
        .map_err(AppError::from)?;

    if !create_response.status().is_success() {
        let status = create_response.status();
        let error_text = create_response.text().await.unwrap_or_default();
        return Ok(GenerateApiKeyResult {
            success: false,
            message: format!("创建token失败 ({}): {}", status, error_text),
            api_key: None,
        });
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    let search_url = format!(
        "https://duckcoding.com/api/token/search?keyword={}",
        urlencoding::encode(name)
    );

    let search_response = client
        .get(&search_url)
        .header(
            "Authorization",
            format!("Bearer {}", global_config.system_token),
        )
        .header("New-Api-User", &global_config.user_id)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(AppError::from)?;

    if !search_response.status().is_success() {
        return Ok(GenerateApiKeyResult {
            success: false,
            message: "创建成功但获取API Key失败，请稍后在DuckCoding控制台查看".to_string(),
            api_key: None,
        });
    }

    let api_response: ApiResponse = search_response.json().await.map_err(AppError::from)?;

    if !api_response.success {
        return Ok(GenerateApiKeyResult {
            success: false,
            message: format!("API返回错误: {}", api_response.message),
            api_key: None,
        });
    }

    if let Some(mut data) = api_response.data {
        if !data.is_empty() {
            data.sort_by(|a, b| b.id.cmp(&a.id));
            let token = &data[0];
            let api_key = format!("sk-{}", token.key);
            return Ok(GenerateApiKeyResult {
                success: true,
                message: "API Key 创建成功".to_string(),
                api_key: Some(api_key),
            });
        }
    }

    Ok(GenerateApiKeyResult {
        success: false,
        message: "未获取到新创建的 token".to_string(),
        api_key: None,
    })
}

#[tauri::command]
pub async fn get_usage_stats() -> Result<UsageStatsResult, String> {
    get_usage_stats_impl().await.map_err(|e| e.to_string())
}

async fn get_usage_stats_impl() -> AppResult<UsageStatsResult> {
    let global_config =
        load_global_config()?.ok_or_else(|| AppError::config("请先配置用户ID和系统访问令牌"))?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let beijing_offset = 8 * 3600;
    let today_end = (now + beijing_offset) / 86400 * 86400 + 86400 - beijing_offset;
    let start_timestamp = today_end - 30 * 86400;
    let end_timestamp = today_end;

    let client = reqwest::Client::new();
    let url = format!(
        "https://duckcoding.com/api/data/self?start_timestamp={}&end_timestamp={}",
        start_timestamp, end_timestamp
    );

    let response = client
        .get(&url)
        .header(
            "Authorization",
            format!("Bearer {}", global_config.system_token),
        )
        .header("New-Api-User", &global_config.user_id)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(AppError::from)?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Ok(UsageStatsResult {
            success: false,
            message: format!("获取用量统计失败 ({}): {}", status, error_text),
            data: vec![],
        });
    }

    let api_response: UsageApiResponse = response.json().await.map_err(AppError::from)?;

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
pub async fn get_user_quota() -> Result<UserQuotaResult, String> {
    get_user_quota_impl().await.map_err(|e| e.to_string())
}

async fn get_user_quota_impl() -> AppResult<UserQuotaResult> {
    let global_config =
        load_global_config()?.ok_or_else(|| AppError::config("请先配置用户ID和系统访问令牌"))?;

    let client = reqwest::Client::new();
    let url = "https://duckcoding.com/api/user/self";

    let response = client
        .get(url)
        .header(
            "Authorization",
            format!("Bearer {}", global_config.system_token),
        )
        .header("New-Api-User", &global_config.user_id)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(AppError::from)?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(AppError::Other(format!(
            "获取用户信息失败 ({}): {}",
            status, error_text
        )));
    }

    let api_response: UserApiResponse = response.json().await.map_err(AppError::from)?;

    if !api_response.success {
        return Err(AppError::Other(format!(
            "API返回错误: {}",
            api_response.message
        )));
    }

    let user_info = api_response
        .data
        .ok_or_else(|| AppError::Other("未获取到用户信息".into()))?;

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
