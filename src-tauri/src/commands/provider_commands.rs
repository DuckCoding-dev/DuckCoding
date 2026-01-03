// Provider Commands
//
// 供应商管理 Tauri 命令

use ::duckcoding::models::provider::Provider;
use ::duckcoding::services::ProviderManager;
use anyhow::Result;
use tauri::State;

/// Provider 管理器 State
pub struct ProviderManagerState {
    pub manager: ProviderManager,
}

impl ProviderManagerState {
    pub fn new() -> Self {
        Self {
            manager: ProviderManager::new().expect("Failed to create ProviderManager"),
        }
    }
}

impl Default for ProviderManagerState {
    fn default() -> Self {
        Self::new()
    }
}

/// 列出所有供应商
#[tauri::command]
pub async fn list_providers(
    state: State<'_, ProviderManagerState>,
) -> Result<Vec<Provider>, String> {
    state
        .manager
        .list_providers()
        .map_err(|e| format!("获取供应商列表失败: {}", e))
}

/// 创建新供应商
#[tauri::command]
pub async fn create_provider(
    provider: Provider,
    state: State<'_, ProviderManagerState>,
) -> Result<Provider, String> {
    // 基础验证
    if provider.id.is_empty() {
        return Err("供应商 ID 不能为空".to_string());
    }
    if provider.name.is_empty() {
        return Err("供应商名称不能为空".to_string());
    }
    if provider.website_url.is_empty() {
        return Err("官网地址不能为空".to_string());
    }

    state
        .manager
        .create_provider(provider)
        .map_err(|e| format!("创建供应商失败: {}", e))
}

/// 更新供应商
#[tauri::command]
pub async fn update_provider(
    id: String,
    provider: Provider,
    state: State<'_, ProviderManagerState>,
) -> Result<Provider, String> {
    // 基础验证
    if provider.name.is_empty() {
        return Err("供应商名称不能为空".to_string());
    }
    if provider.website_url.is_empty() {
        return Err("官网地址不能为空".to_string());
    }

    state
        .manager
        .update_provider(&id, provider)
        .map_err(|e| format!("更新供应商失败: {}", e))
}

/// 删除供应商
#[tauri::command]
pub async fn delete_provider(
    id: String,
    state: State<'_, ProviderManagerState>,
) -> Result<(), String> {
    if id.is_empty() {
        return Err("供应商 ID 不能为空".to_string());
    }

    state
        .manager
        .delete_provider(&id)
        .map_err(|e| format!("删除供应商失败: {}", e))
}

/// 验证结果结构
#[derive(serde::Serialize)]
pub struct ValidationResult {
    pub success: bool,
    pub username: Option<String>,
    pub error: Option<String>,
}

/// 验证供应商配置（检查 API 连通性）
#[tauri::command]
pub async fn validate_provider_config(provider: Provider) -> Result<ValidationResult, String> {
    use reqwest::Client;
    use std::time::Duration;

    // 基础验证
    if provider.website_url.is_empty() {
        return Ok(ValidationResult {
            success: false,
            username: None,
            error: Some("官网地址不能为空".to_string()),
        });
    }
    if provider.user_id.is_empty() {
        return Ok(ValidationResult {
            success: false,
            username: None,
            error: Some("用户 ID 不能为空".to_string()),
        });
    }
    if provider.access_token.is_empty() {
        return Ok(ValidationResult {
            success: false,
            username: None,
            error: Some("访问令牌不能为空".to_string()),
        });
    }

    // 构建 API 端点
    let api_url = format!(
        "{}/api/user/self",
        provider.website_url.trim_end_matches('/')
    );

    // 发送验证请求
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let response = client
        .get(&api_url)
        .header("Authorization", format!("Bearer {}", provider.access_token))
        .header("New-Api-User", &provider.user_id)
        .send()
        .await
        .map_err(|e| format!("API 请求失败: {}", e))?;

    if response.status().is_success() {
        // 尝试解析响应，提取用户名
        let json_result = response.json::<serde_json::Value>().await;
        match json_result {
            Ok(json) => {
                // 尝试从响应中提取用户名 (假设在 data.username 或 username 字段)
                let username = json
                    .get("data")
                    .and_then(|data| data.get("username"))
                    .or_else(|| json.get("username"))
                    .and_then(|u| u.as_str())
                    .map(|s| s.to_string());

                Ok(ValidationResult {
                    success: true,
                    username,
                    error: None,
                })
            }
            Err(e) => Ok(ValidationResult {
                success: false,
                username: None,
                error: Some(format!("API 响应格式错误: {}", e)),
            }),
        }
    } else {
        Ok(ValidationResult {
            success: false,
            username: None,
            error: Some(format!(
                "API 验证失败，状态码: {}",
                response.status().as_u16()
            )),
        })
    }
}
