//! AMP Code 用户认证相关命令
//!
//! 通过 AMP Code Access Token 调用 ampcode.com API 获取用户信息

use ::duckcoding::services::proxy_config_manager::ProxyConfigManager;

/// AMP Code 用户信息响应
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct AmpUserInfo {
    pub id: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub username: Option<String>,
}

/// 通过 AMP Code Access Token 获取用户信息
///
/// 调用 ampcode.com/api/user 验证 token 并获取用户信息
#[tauri::command]
pub async fn get_amp_user_info(access_token: String) -> Result<AmpUserInfo, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let response = client
        .get("https://ampcode.com/api/user")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("X-Api-Key", &access_token)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("请求 AMP Code API 失败: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "无法读取响应".to_string());
        return Err(format!("AMP Code API 返回错误 {}: {}", status, body));
    }

    let user_info: AmpUserInfo = response
        .json()
        .await
        .map_err(|e| format!("解析用户信息失败: {}", e))?;

    tracing::info!(
        user_id = %user_info.id,
        username = ?user_info.username,
        "成功获取 AMP Code 用户信息"
    );

    Ok(user_info)
}

/// 验证 AMP Access Token 并保存到代理配置
///
/// 1. 调用 get_amp_user_info 验证 token
/// 2. 成功后保存 real_api_key 和 real_base_url 到 proxy.json
#[tauri::command]
pub async fn validate_and_save_amp_token(access_token: String) -> Result<AmpUserInfo, String> {
    // 1. 验证 token
    let user_info = get_amp_user_info(access_token.clone()).await?;

    // 2. 保存到 proxy.json
    let proxy_mgr = ProxyConfigManager::new().map_err(|e| e.to_string())?;

    let mut config = proxy_mgr
        .get_config("amp-code")
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| {
            use ::duckcoding::models::proxy_config::ToolProxyConfig;
            ToolProxyConfig::new(8790)
        });

    config.real_api_key = Some(access_token);
    config.real_base_url = Some("https://ampcode.com".to_string());

    proxy_mgr
        .update_config("amp-code", config)
        .map_err(|e| e.to_string())?;

    tracing::info!(
        user_id = %user_info.id,
        "AMP Code Access Token 验证成功，已保存到代理配置"
    );

    Ok(user_info)
}

/// 获取已保存的 AMP Code 用户信息（从 proxy.json 读取 token 并验证）
#[tauri::command]
pub async fn get_saved_amp_user_info() -> Result<Option<AmpUserInfo>, String> {
    let proxy_mgr = ProxyConfigManager::new().map_err(|e| e.to_string())?;

    let config = proxy_mgr
        .get_config("amp-code")
        .map_err(|e| e.to_string())?;

    match config.and_then(|c| c.real_api_key) {
        Some(token) => {
            // 有保存的 token，尝试获取用户信息
            match get_amp_user_info(token).await {
                Ok(info) => Ok(Some(info)),
                Err(e) => {
                    tracing::warn!("已保存的 AMP Code Token 无效: {}", e);
                    Ok(None)
                }
            }
        }
        None => Ok(None),
    }
}
