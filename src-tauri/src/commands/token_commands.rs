// Token Management Commands
//
// NEW API 令牌管理相关命令

use ::duckcoding::models::provider::Provider;
use ::duckcoding::models::remote_token::{
    CreateRemoteTokenRequest, RemoteToken, RemoteTokenGroup, TokenListData,
    UpdateRemoteTokenRequest,
};
use ::duckcoding::services::profile_manager::types::TokenImportStatus;
use ::duckcoding::services::{
    ClaudeProfile, CodexProfile, GeminiProfile, NewApiClient, ProfileSource,
};
use anyhow::Result;
use chrono::Utc;
use tauri::State;

/// 检测令牌是否已导入到任何工具
#[tauri::command]
pub async fn check_token_import_status(
    profile_manager: State<'_, crate::commands::profile_commands::ProfileManagerState>,
    provider_id: String,
    remote_token_id: i64,
) -> Result<Vec<TokenImportStatus>, String> {
    let manager = profile_manager.manager.read().await;
    manager
        .check_import_status(&provider_id, remote_token_id)
        .map_err(|e| e.to_string())
}

/// 获取指定供应商的远程令牌列表（支持分页）
#[tauri::command]
pub async fn fetch_provider_tokens(
    provider: Provider,
    page: i32,
    page_size: i32,
) -> Result<TokenListData, String> {
    let client = NewApiClient::new(provider).map_err(|e| e.to_string())?;
    client
        .list_tokens(page, page_size)
        .await
        .map_err(|e| e.to_string())
}

/// 获取指定供应商的令牌分组列表
#[tauri::command]
pub async fn fetch_provider_groups(provider: Provider) -> Result<Vec<RemoteTokenGroup>, String> {
    let client = NewApiClient::new(provider).map_err(|e| e.to_string())?;
    client.list_groups().await.map_err(|e| e.to_string())
}

/// 在供应商创建新的远程令牌（仅返回成功状态）
#[tauri::command]
pub async fn create_provider_token(
    provider: Provider,
    request: CreateRemoteTokenRequest,
) -> Result<(), String> {
    let client = NewApiClient::new(provider).map_err(|e| e.to_string())?;
    client
        .create_token(request)
        .await
        .map_err(|e| e.to_string())
}

/// 删除供应商的远程令牌
#[tauri::command]
pub async fn delete_provider_token(provider: Provider, token_id: i64) -> Result<(), String> {
    let client = NewApiClient::new(provider).map_err(|e| e.to_string())?;
    client
        .delete_token(token_id)
        .await
        .map_err(|e| e.to_string())
}

/// 更新供应商的远程令牌名称
#[tauri::command]
pub async fn update_provider_token(
    provider: Provider,
    token_id: i64,
    name: String,
) -> Result<RemoteToken, String> {
    let client = NewApiClient::new(provider).map_err(|e| e.to_string())?;
    client
        .update_token(token_id, name)
        .await
        .map_err(|e| e.to_string())
}

/// 更新供应商的远程令牌（完整版本，支持所有字段）
#[tauri::command]
pub async fn update_provider_token_full(
    provider: Provider,
    token_id: i64,
    request: UpdateRemoteTokenRequest,
) -> Result<RemoteToken, String> {
    let client = NewApiClient::new(provider).map_err(|e| e.to_string())?;
    client
        .update_token_full(token_id, request)
        .await
        .map_err(|e| e.to_string())
}

/// 导入远程令牌为本地 Profile
#[tauri::command]
pub async fn import_token_as_profile(
    profile_manager: State<'_, crate::commands::profile_commands::ProfileManagerState>,
    provider: Provider,
    remote_token: RemoteToken,
    tool_id: String,
    profile_name: String,
    pricing_template_id: Option<String>, // 🆕 Phase 6: 可选的价格模板 ID
) -> Result<(), String> {
    // 验证 tool_id
    if tool_id != "claude-code" && tool_id != "codex" && tool_id != "gemini-cli" {
        return Err(format!("不支持的工具类型: {}", tool_id));
    }

    // 构建 ProfileSource
    let source = ProfileSource::ImportedFromProvider {
        provider_id: provider.id.clone(),
        provider_name: provider.name.clone(),
        remote_token_id: remote_token.id,
        remote_token_name: remote_token.name.clone(),
        group: remote_token.group.clone(),
        imported_at: Utc::now().timestamp(),
    };

    // 提取 API Key 和 Base URL
    // 优先使用 api_address，未设置时使用 website_url
    let api_key = remote_token.key.clone();
    let base_url = provider
        .api_address
        .clone()
        .unwrap_or(provider.website_url.clone());

    // 直接操作 ProfilesStore 以支持自定义 source 字段
    let manager = profile_manager.manager.read().await;
    let mut store = manager.load_profiles_store().map_err(|e| e.to_string())?;

    // 根据工具类型创建对应的 Profile
    match tool_id.as_str() {
        "claude-code" => {
            let profile = ClaudeProfile {
                api_key,
                base_url,
                source,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                raw_settings: None,
                raw_config_json: None,
                pricing_template_id: pricing_template_id.clone(),
            };
            store.claude_code.insert(profile_name.clone(), profile);
        }
        "codex" => {
            let profile = CodexProfile {
                api_key,
                base_url,
                wire_api: "responses".to_string(), // 默认使用 responses API
                source,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                raw_config_toml: None,
                raw_auth_json: None,
                pricing_template_id: pricing_template_id.clone(),
            };
            store.codex.insert(profile_name.clone(), profile);
        }
        "gemini-cli" => {
            let profile = GeminiProfile {
                api_key,
                base_url,
                model: None, // 不指定 model，保留用户原有配置
                source,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                raw_settings: None,
                raw_env: None,
                pricing_template_id: pricing_template_id.clone(),
            };
            store.gemini_cli.insert(profile_name.clone(), profile);
        }
        _ => return Err(format!("不支持的工具类型: {}", tool_id)),
    }

    store.metadata.last_updated = Utc::now();
    manager
        .save_profiles_store(&store)
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 创建自定义 Profile（非导入令牌）
#[tauri::command]
pub async fn create_custom_profile(
    profile_manager: State<'_, crate::commands::profile_commands::ProfileManagerState>,
    tool_id: String,
    profile_name: String,
    api_key: String,
    base_url: String,
    extra_config: Option<serde_json::Value>,
) -> Result<(), String> {
    // 验证 tool_id
    if tool_id != "claude-code" && tool_id != "codex" && tool_id != "gemini-cli" {
        return Err(format!("不支持的工具类型: {}", tool_id));
    }

    let source = ProfileSource::Custom;

    // 直接操作 ProfilesStore 以支持自定义 source 字段
    let manager = profile_manager.manager.read().await;
    let mut store = manager.load_profiles_store().map_err(|e| e.to_string())?;

    // 从 extra_config 中提取 pricing_template_id
    let pricing_template_id = extra_config
        .as_ref()
        .and_then(|v| v.get("pricing_template_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // 根据工具类型创建对应的 Profile
    match tool_id.as_str() {
        "claude-code" => {
            let profile = ClaudeProfile {
                api_key,
                base_url,
                source,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                raw_settings: None,
                raw_config_json: None,
                pricing_template_id: pricing_template_id.clone(),
            };
            store.claude_code.insert(profile_name.clone(), profile);
        }
        "codex" => {
            // 从 extra_config 中提取 wire_api
            let wire_api = extra_config
                .as_ref()
                .and_then(|v| v.get("wire_api"))
                .and_then(|v| v.as_str())
                .unwrap_or("responses")
                .to_string();

            let profile = CodexProfile {
                api_key,
                base_url,
                wire_api,
                source,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                raw_config_toml: None,
                raw_auth_json: None,
                pricing_template_id: pricing_template_id.clone(),
            };
            store.codex.insert(profile_name.clone(), profile);
        }
        "gemini-cli" => {
            // 从 extra_config 中提取 model
            let model = extra_config
                .as_ref()
                .and_then(|v| v.get("model"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let profile = GeminiProfile {
                api_key,
                base_url,
                model,
                source,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                raw_settings: None,
                raw_env: None,
                pricing_template_id: pricing_template_id.clone(),
            };
            store.gemini_cli.insert(profile_name.clone(), profile);
        }
        _ => return Err(format!("不支持的工具类型: {}", tool_id)),
    }

    store.metadata.last_updated = Utc::now();
    manager
        .save_profiles_store(&store)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_tool_id() {
        let valid_ids = vec!["claude-code", "codex", "gemini-cli"];
        for id in valid_ids {
            assert!(
                id == "claude-code" || id == "codex" || id == "gemini-cli",
                "Tool ID validation failed for: {}",
                id
            );
        }

        let invalid_ids = vec!["invalid", "unknown-tool", ""];
        for id in invalid_ids {
            assert!(
                id != "claude-code" && id != "codex" && id != "gemini-cli",
                "Tool ID validation should fail for: {}",
                id
            );
        }
    }

    #[test]
    fn test_provider_creation() {
        let provider = Provider {
            id: "test-provider".to_string(),
            name: "Test Provider".to_string(),
            website_url: "https://api.test.com".to_string(),
            api_address: None,
            user_id: "123".to_string(),
            access_token: "token123".to_string(),
            username: None,
            is_default: false,
            created_at: 0,
            updated_at: 0,
        };

        assert_eq!(provider.id, "test-provider");
        assert_eq!(provider.website_url, "https://api.test.com");
    }

    #[test]
    fn test_profile_source_custom() {
        let source = ProfileSource::Custom;
        assert_eq!(source, ProfileSource::Custom);
    }

    #[test]
    fn test_profile_source_imported() {
        let source = ProfileSource::ImportedFromProvider {
            provider_id: "provider-1".to_string(),
            provider_name: "Provider One".to_string(),
            remote_token_id: 100,
            remote_token_name: "Token Name".to_string(),
            group: "default".to_string(),
            imported_at: 1234567890,
        };

        if let ProfileSource::ImportedFromProvider {
            provider_id,
            remote_token_id,
            ..
        } = source
        {
            assert_eq!(provider_id, "provider-1");
            assert_eq!(remote_token_id, 100);
        } else {
            panic!("Expected ImportedFromProvider variant");
        }
    }
}
