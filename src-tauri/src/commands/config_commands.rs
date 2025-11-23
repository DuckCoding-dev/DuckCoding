// 配置管理相关命令

use serde_json::Value;
use std::fs;

use super::proxy_commands::{ProxyManagerState, TransparentProxyState};
use super::types::ActiveConfig;
use ::duckcoding::services::config::{
    CodexSettingsPayload, GeminiEnvPayload, GeminiSettingsPayload,
};
use ::duckcoding::services::proxy::{ProxyConfig, TransparentProxyConfigService};
use ::duckcoding::utils::config::{
    apply_proxy_if_configured, read_global_config, write_global_config,
};
use ::duckcoding::ConfigService;
use ::duckcoding::GlobalConfig;
use ::duckcoding::Tool;

// ==================== 类型定义 ====================

#[derive(serde::Deserialize, Debug)]
struct TokenData {
    id: i64,
    key: String,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    group: String,
}

#[derive(serde::Deserialize, Debug)]
struct ApiResponse {
    success: bool,
    message: String,
    data: Option<Vec<TokenData>>,
}

#[derive(serde::Serialize)]
pub struct GenerateApiKeyResult {
    success: bool,
    message: String,
    api_key: Option<String>,
}

// ==================== 辅助函数 ====================

fn build_reqwest_client() -> Result<reqwest::Client, String> {
    ::duckcoding::http_client::build_client()
}

fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        return "****".to_string();
    }
    let prefix = &key[..4];
    let suffix = &key[key.len() - 4..];
    format!("{prefix}...{suffix}")
}

fn detect_profile_name(
    tool: &str,
    active_api_key: &str,
    active_base_url: &str,
    home_dir: &std::path::Path,
) -> Option<String> {
    let config_dir = match tool {
        "claude-code" => home_dir.join(".claude"),
        "codex" => home_dir.join(".codex"),
        "gemini-cli" => home_dir.join(".gemini"),
        _ => return None,
    };

    if !config_dir.exists() {
        return None;
    }

    // 遍历配置目录，查找匹配的备份文件
    if let Ok(entries) = fs::read_dir(&config_dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            // 根据工具类型匹配不同的备份文件格式
            let profile_name = match tool {
                "claude-code" => {
                    // 匹配 settings.{profile}.json
                    if file_name_str.starts_with("settings.")
                        && file_name_str.ends_with(".json")
                        && file_name_str != "settings.json"
                    {
                        file_name_str
                            .strip_prefix("settings.")
                            .and_then(|s| s.strip_suffix(".json"))
                    } else {
                        None
                    }
                }
                "codex" => {
                    // 匹配 config.{profile}.toml
                    if file_name_str.starts_with("config.")
                        && file_name_str.ends_with(".toml")
                        && file_name_str != "config.toml"
                    {
                        file_name_str
                            .strip_prefix("config.")
                            .and_then(|s| s.strip_suffix(".toml"))
                    } else {
                        None
                    }
                }
                "gemini-cli" => {
                    // 匹配 .env.{profile}
                    if file_name_str.starts_with(".env.") && file_name_str != ".env" {
                        file_name_str.strip_prefix(".env.")
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(profile) = profile_name {
                // 读取备份文件并比较内容
                let is_match = match tool {
                    "claude-code" => {
                        if let Ok(content) = fs::read_to_string(entry.path()) {
                            if let Ok(config) = serde_json::from_str::<Value>(&content) {
                                let env_api_key = config
                                    .get("env")
                                    .and_then(|env| env.get("ANTHROPIC_AUTH_TOKEN"))
                                    .and_then(|v| v.as_str());
                                let env_base_url = config
                                    .get("env")
                                    .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
                                    .and_then(|v| v.as_str());

                                let flat_api_key =
                                    config.get("ANTHROPIC_AUTH_TOKEN").and_then(|v| v.as_str());
                                let flat_base_url =
                                    config.get("ANTHROPIC_BASE_URL").and_then(|v| v.as_str());

                                let backup_api_key = env_api_key.or(flat_api_key).unwrap_or("");
                                let backup_base_url = env_base_url.or(flat_base_url).unwrap_or("");

                                backup_api_key == active_api_key
                                    && backup_base_url == active_base_url
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    }
                    "codex" => {
                        // 需要同时检查 config.toml 和 auth.json
                        let auth_backup = config_dir.join(format!("auth.{profile}.json"));

                        let mut api_key_matches = false;
                        if let Ok(auth_content) = fs::read_to_string(&auth_backup) {
                            if let Ok(auth) = serde_json::from_str::<Value>(&auth_content) {
                                let backup_api_key = auth
                                    .get("OPENAI_API_KEY")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");

                                api_key_matches = backup_api_key == active_api_key;
                            }
                        }

                        if !api_key_matches {
                            false
                        } else {
                            // API Key 匹配，继续检查 base_url
                            if let Ok(config_content) = fs::read_to_string(entry.path()) {
                                if let Ok(toml::Value::Table(table)) =
                                    toml::from_str::<toml::Value>(&config_content)
                                {
                                    if let Some(toml::Value::Table(providers)) =
                                        table.get("model_providers")
                                    {
                                        let mut url_matches = false;
                                        for (_, provider) in providers {
                                            if let toml::Value::Table(p) = provider {
                                                if let Some(toml::Value::String(url)) =
                                                    p.get("base_url")
                                                {
                                                    if url == active_base_url {
                                                        url_matches = true;
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                        url_matches
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        }
                    }
                    "gemini-cli" => {
                        if let Ok(content) = fs::read_to_string(entry.path()) {
                            let mut backup_api_key = "";
                            let mut backup_base_url = "";

                            for line in content.lines() {
                                let line = line.trim();
                                if line.is_empty() || line.starts_with('#') {
                                    continue;
                                }

                                if let Some((key, value)) = line.split_once('=') {
                                    match key.trim() {
                                        "GEMINI_API_KEY" => backup_api_key = value.trim(),
                                        "GOOGLE_GEMINI_BASE_URL" => backup_base_url = value.trim(),
                                        _ => {}
                                    }
                                }
                            }

                            backup_api_key == active_api_key && backup_base_url == active_base_url
                        } else {
                            false
                        }
                    }
                    _ => false,
                };

                if is_match {
                    return Some(profile.to_string());
                }
            }
        }
    }

    None
}

// ==================== Tauri 命令 ====================

#[tauri::command]
pub async fn configure_api(
    tool: String,
    _provider: String,
    api_key: String,
    base_url: Option<String>,
    profile_name: Option<String>,
) -> Result<(), String> {
    #[cfg(debug_assertions)]
    tracing::debug!(tool = %tool, "配置API（使用ConfigService）");

    // 获取工具定义
    let tool_obj = Tool::by_id(&tool).ok_or_else(|| format!("❌ 未知的工具: {tool}"))?;

    // 获取 base_url，根据工具类型使用不同的默认值
    let base_url_str = base_url.unwrap_or_else(|| match tool.as_str() {
        "codex" => "https://jp.duckcoding.com/v1".to_string(),
        _ => "https://jp.duckcoding.com".to_string(),
    });

    // 使用 ConfigService 应用配置
    ConfigService::apply_config(&tool_obj, &api_key, &base_url_str, profile_name.as_deref())
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn list_profiles(tool: String) -> Result<Vec<String>, String> {
    #[cfg(debug_assertions)]
    tracing::debug!(tool = %tool, "列出配置文件（使用ConfigService）");

    // 获取工具定义
    let tool_obj = Tool::by_id(&tool).ok_or_else(|| format!("❌ 未知的工具: {tool}"))?;

    // 使用 ConfigService 列出配置
    ConfigService::list_profiles(&tool_obj).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn switch_profile(
    tool: String,
    profile: String,
    state: tauri::State<'_, TransparentProxyState>,
    manager_state: tauri::State<'_, ProxyManagerState>,
) -> Result<(), String> {
    #[cfg(debug_assertions)]
    tracing::debug!(
        tool = %tool,
        profile = %profile,
        "切换配置文件（使用ConfigService）"
    );

    // 获取工具定义
    let tool_obj = Tool::by_id(&tool).ok_or_else(|| format!("❌ 未知的工具: {tool}"))?;

    // 读取全局配置，检查是否在代理模式
    let global_config_opt = get_global_config().await.map_err(|e| e.to_string())?;

    // 检查该工具的透明代理是否启用
    let proxy_enabled = if let Some(ref config) = global_config_opt {
        let tool_proxy_enabled = config
            .get_proxy_config(&tool)
            .map(|c| c.enabled)
            .unwrap_or(false);
        // 兼容旧字段（仅 claude-code）
        let legacy_proxy_enabled = tool == "claude-code" && config.transparent_proxy_enabled;
        tool_proxy_enabled || legacy_proxy_enabled
    } else {
        false
    };

    if proxy_enabled {
        // 代理模式：直接从备份文件读取配置，不修改当前配置文件
        let mut global_config = global_config_opt.ok_or("全局配置不存在")?;

        // 从备份文件读取真实配置
        let (new_api_key, new_base_url) = match tool.as_str() {
            "claude-code" => {
                let backup_path = tool_obj.backup_path(&profile);
                if !backup_path.exists() {
                    return Err(format!("配置文件不存在: {backup_path:?}"));
                }
                let content = fs::read_to_string(&backup_path)
                    .map_err(|e| format!("读取备份配置失败: {e}"))?;
                let backup_data: Value =
                    serde_json::from_str(&content).map_err(|e| format!("解析备份配置失败: {e}"))?;

                // 兼容新旧格式
                let api_key = backup_data
                    .get("ANTHROPIC_AUTH_TOKEN")
                    .and_then(|v| v.as_str())
                    .or_else(|| {
                        backup_data
                            .get("env")
                            .and_then(|env| env.get("ANTHROPIC_AUTH_TOKEN"))
                            .and_then(|v| v.as_str())
                    })
                    .ok_or("备份配置缺少 API Key")?
                    .to_string();

                let base_url = backup_data
                    .get("ANTHROPIC_BASE_URL")
                    .and_then(|v| v.as_str())
                    .or_else(|| {
                        backup_data
                            .get("env")
                            .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
                            .and_then(|v| v.as_str())
                    })
                    .ok_or("备份配置缺少 Base URL")?
                    .to_string();

                (api_key, base_url)
            }
            "codex" => {
                // 读取备份的 auth.json
                let backup_auth = tool_obj.config_dir.join(format!("auth.{profile}.json"));
                let backup_config = tool_obj.config_dir.join(format!("config.{profile}.toml"));

                if !backup_auth.exists() {
                    return Err(format!("配置文件不存在: {backup_auth:?}"));
                }

                let auth_content = fs::read_to_string(&backup_auth)
                    .map_err(|e| format!("读取备份 auth.json 失败: {e}"))?;
                let auth_data: Value = serde_json::from_str(&auth_content)
                    .map_err(|e| format!("解析备份 auth.json 失败: {e}"))?;
                let api_key = auth_data
                    .get("OPENAI_API_KEY")
                    .and_then(|v| v.as_str())
                    .ok_or("备份配置缺少 API Key")?
                    .to_string();

                // 读取备份的 config.toml
                let base_url = if backup_config.exists() {
                    let config_content = fs::read_to_string(&backup_config)
                        .map_err(|e| format!("读取备份 config.toml 失败: {e}"))?;
                    let config: toml::Value = toml::from_str(&config_content)
                        .map_err(|e| format!("解析备份 config.toml 失败: {e}"))?;
                    let provider = config
                        .get("model_provider")
                        .and_then(|v| v.as_str())
                        .unwrap_or("custom");
                    config
                        .get("model_providers")
                        .and_then(|mp| mp.get(provider))
                        .and_then(|p| p.get("base_url"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string()
                } else {
                    return Err(format!("配置文件不存在: {backup_config:?}"));
                };

                (api_key, base_url)
            }
            "gemini-cli" => {
                let backup_env = tool_obj.config_dir.join(format!(".env.{profile}"));
                if !backup_env.exists() {
                    return Err(format!("配置文件不存在: {backup_env:?}"));
                }

                let content = fs::read_to_string(&backup_env)
                    .map_err(|e| format!("读取备份 .env 失败: {e}"))?;
                let mut api_key = String::new();
                let mut base_url = String::new();

                for line in content.lines() {
                    let trimmed = line.trim();
                    if let Some((key, value)) = trimmed.split_once('=') {
                        match key.trim() {
                            "GEMINI_API_KEY" => api_key = value.trim().to_string(),
                            "GOOGLE_GEMINI_BASE_URL" => base_url = value.trim().to_string(),
                            _ => {}
                        }
                    }
                }

                if api_key.is_empty() || base_url.is_empty() {
                    return Err("备份配置缺少必要字段".to_string());
                }

                (api_key, base_url)
            }
            _ => return Err(format!("未知工具: {tool}")),
        };

        if !new_api_key.is_empty() && !new_base_url.is_empty() {
            // 更新保存的真实配置
            TransparentProxyConfigService::update_real_config(
                &tool_obj,
                &mut global_config,
                &new_api_key,
                &new_base_url,
            )
            .map_err(|e| format!("更新真实配置失败: {e}"))?;

            // 同时保存配置名称
            if let Some(proxy_config) = global_config.get_proxy_config_mut(&tool) {
                proxy_config.real_profile_name = Some(profile.clone());
            }

            // 保存全局配置
            save_global_config(global_config.clone())
                .await
                .map_err(|e| format!("保存全局配置失败: {e}"))?;

            // 检查代理是否正在运行并更新
            let is_running = manager_state.manager.is_running(&tool).await;

            if is_running {
                // 更新 ProxyManager 中的配置
                if let Some(tool_config) = global_config.get_proxy_config(&tool) {
                    let mut updated_config = tool_config.clone();
                    updated_config.real_api_key = Some(new_api_key.clone());
                    updated_config.real_base_url = Some(new_base_url.clone());
                    updated_config.real_profile_name = Some(profile.clone());

                    manager_state
                        .manager
                        .update_config(&tool, updated_config)
                        .await
                        .map_err(|e| format!("更新代理配置失败: {e}"))?;

                    tracing::info!(tool = %tool, "透明代理配置已自动更新");
                }
            }

            // 兼容旧版 claude-code 代理
            if tool == "claude-code" && global_config.transparent_proxy_enabled {
                let service = state.service.lock().await;
                if service.is_running().await {
                    let local_api_key = global_config
                        .transparent_proxy_api_key
                        .clone()
                        .unwrap_or_default();

                    let proxy_config = ProxyConfig {
                        target_api_key: new_api_key.clone(),
                        target_base_url: new_base_url.clone(),
                        local_api_key,
                    };

                    service
                        .update_config(proxy_config)
                        .await
                        .map_err(|e| format!("更新代理配置失败: {e}"))?;
                }
                drop(service);
            }

            tracing::info!(
                tool = %tool,
                profile = %profile,
                "配置已切换（代理模式）"
            );
        }
    } else {
        // 非代理模式：正常激活配置
        ConfigService::activate_profile(&tool_obj, &profile).map_err(|e| e.to_string())?;
        tracing::info!(
            tool = %tool,
            profile = %profile,
            "配置已切换"
        );
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_profile(tool: String, profile: String) -> Result<(), String> {
    #[cfg(debug_assertions)]
    tracing::debug!(
        tool = %tool,
        profile = %profile,
        "删除配置文件"
    );

    // 获取工具定义
    let tool_obj = Tool::by_id(&tool).ok_or_else(|| format!("❌ 未知的工具: {tool}"))?;

    // 使用 ConfigService 删除配置
    ConfigService::delete_profile(&tool_obj, &profile).map_err(|e| e.to_string())?;

    #[cfg(debug_assertions)]
    tracing::debug!(profile = %profile, "配置文件删除成功");

    Ok(())
}

#[tauri::command]
pub async fn get_active_config(tool: String) -> Result<ActiveConfig, String> {
    let home_dir = dirs::home_dir().ok_or("❌ 无法获取用户主目录")?;

    match tool.as_str() {
        "claude-code" => {
            let config_path = home_dir.join(".claude").join("settings.json");
            if !config_path.exists() {
                return Ok(ActiveConfig {
                    api_key: "未配置".to_string(),
                    base_url: "未配置".to_string(),
                    profile_name: None,
                });
            }

            let content =
                fs::read_to_string(&config_path).map_err(|e| format!("❌ 读取配置失败: {e}"))?;
            let config: Value =
                serde_json::from_str(&content).map_err(|e| format!("❌ 解析配置失败: {e}"))?;

            let raw_api_key = config
                .get("env")
                .and_then(|env| env.get("ANTHROPIC_AUTH_TOKEN"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let api_key = if raw_api_key.is_empty() {
                "未配置".to_string()
            } else {
                mask_api_key(raw_api_key)
            };

            let base_url = config
                .get("env")
                .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
                .and_then(|v| v.as_str())
                .unwrap_or("未配置");

            // 检测配置名称
            let profile_name = if !raw_api_key.is_empty() && base_url != "未配置" {
                detect_profile_name("claude-code", raw_api_key, base_url, &home_dir)
            } else {
                None
            };

            Ok(ActiveConfig {
                api_key,
                base_url: base_url.to_string(),
                profile_name,
            })
        }
        "codex" => {
            let auth_path = home_dir.join(".codex").join("auth.json");
            let config_path = home_dir.join(".codex").join("config.toml");

            let mut raw_api_key = String::new();
            let mut api_key = "未配置".to_string();
            let mut base_url = "未配置".to_string();

            // 读取 auth.json
            if auth_path.exists() {
                let content = fs::read_to_string(&auth_path)
                    .map_err(|e| format!("❌ 读取认证文件失败: {e}"))?;
                let auth: Value = serde_json::from_str(&content)
                    .map_err(|e| format!("❌ 解析认证文件失败: {e}"))?;

                if let Some(key) = auth.get("OPENAI_API_KEY").and_then(|v| v.as_str()) {
                    raw_api_key = key.to_string();
                    api_key = mask_api_key(key);
                }
            }

            // 读取 config.toml
            if config_path.exists() {
                let content = fs::read_to_string(&config_path)
                    .map_err(|e| format!("❌ 读取配置文件失败: {e}"))?;
                let config: toml::Value =
                    toml::from_str(&content).map_err(|e| format!("❌ 解析TOML失败: {e}"))?;

                if let toml::Value::Table(table) = config {
                    let selected_provider = table
                        .get("model_provider")
                        .and_then(|value| value.as_str())
                        .map(|s| s.to_string());

                    if let Some(toml::Value::Table(providers)) = table.get("model_providers") {
                        if let Some(provider_name) = selected_provider.as_deref() {
                            if let Some(toml::Value::Table(provider_table)) =
                                providers.get(provider_name)
                            {
                                if let Some(toml::Value::String(url)) =
                                    provider_table.get("base_url")
                                {
                                    base_url = url.clone();
                                }
                            }
                        }

                        if base_url == "未配置" {
                            for (_, provider) in providers {
                                if let toml::Value::Table(p) = provider {
                                    if let Some(toml::Value::String(url)) = p.get("base_url") {
                                        base_url = url.clone();
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 检测配置名称
            let profile_name = if !raw_api_key.is_empty() && base_url != "未配置" {
                detect_profile_name("codex", &raw_api_key, &base_url, &home_dir)
            } else {
                None
            };

            Ok(ActiveConfig {
                api_key,
                base_url,
                profile_name,
            })
        }
        "gemini-cli" => {
            let env_path = home_dir.join(".gemini").join(".env");
            if !env_path.exists() {
                return Ok(ActiveConfig {
                    api_key: "未配置".to_string(),
                    base_url: "未配置".to_string(),
                    profile_name: None,
                });
            }

            let content = fs::read_to_string(&env_path)
                .map_err(|e| format!("❌ 读取环境变量配置失败: {e}"))?;

            let mut raw_api_key = String::new();
            let mut api_key = "未配置".to_string();
            let mut base_url = "未配置".to_string();

            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                if let Some((key, value)) = line.split_once('=') {
                    match key.trim() {
                        "GEMINI_API_KEY" => {
                            raw_api_key = value.trim().to_string();
                            api_key = mask_api_key(value.trim());
                        }
                        "GOOGLE_GEMINI_BASE_URL" => base_url = value.trim().to_string(),
                        _ => {}
                    }
                }
            }

            // 检测配置名称
            let profile_name = if !raw_api_key.is_empty() && base_url != "未配置" {
                detect_profile_name("gemini-cli", &raw_api_key, &base_url, &home_dir)
            } else {
                None
            };

            Ok(ActiveConfig {
                api_key,
                base_url,
                profile_name,
            })
        }
        _ => Err(format!("❌ 未知的工具: {tool}")),
    }
}

#[tauri::command]
pub async fn save_global_config(config: GlobalConfig) -> Result<(), String> {
    write_global_config(&config)
}

#[tauri::command]
pub async fn get_global_config() -> Result<Option<GlobalConfig>, String> {
    read_global_config()
}

#[tauri::command]
pub async fn generate_api_key_for_tool(tool: String) -> Result<GenerateApiKeyResult, String> {
    // 应用代理配置（如果已配置）
    apply_proxy_if_configured();

    // 读取全局配置
    let global_config = get_global_config()
        .await?
        .ok_or("请先配置用户ID和系统访问令牌")?;

    // 根据工具名称获取配置
    let (name, group) = match tool.as_str() {
        "claude-code" => ("Claude Code一键创建", "Claude Code专用"),
        "codex" => ("CodeX一键创建", "CodeX专用"),
        "gemini-cli" => ("Gemini CLI一键创建", "Gemini CLI专用"),
        _ => return Err(format!("Unknown tool: {tool}")),
    };

    // 创建token
    let client = build_reqwest_client().map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
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
        .map_err(|e| format!("创建token失败: {e}"))?;

    if !create_response.status().is_success() {
        let status = create_response.status();
        let error_text = create_response.text().await.unwrap_or_default();
        return Ok(GenerateApiKeyResult {
            success: false,
            message: format!("创建token失败 ({status}): {error_text}"),
            api_key: None,
        });
    }

    // 等待一小段时间让服务器处理
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // 搜索刚创建的token
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
        .map_err(|e| format!("搜索token失败: {e}"))?;

    if !search_response.status().is_success() {
        return Ok(GenerateApiKeyResult {
            success: false,
            message: "创建成功但获取API Key失败，请稍后在DuckCoding控制台查看".to_string(),
            api_key: None,
        });
    }

    let api_response: ApiResponse = search_response
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {e}"))?;

    if !api_response.success {
        return Ok(GenerateApiKeyResult {
            success: false,
            message: format!("API返回错误: {}", api_response.message),
            api_key: None,
        });
    }

    // 获取id最大的token（最新创建的）
    if let Some(mut data) = api_response.data {
        if !data.is_empty() {
            // 按id降序排序，取第一个（id最大的）
            data.sort_by(|a, b| b.id.cmp(&a.id));
            let token = &data[0];
            let api_key = format!("sk-{}", token.key);
            return Ok(GenerateApiKeyResult {
                success: true,
                message: "API Key生成成功".to_string(),
                api_key: Some(api_key),
            });
        }
    }

    Ok(GenerateApiKeyResult {
        success: false,
        message: "未找到生成的token".to_string(),
        api_key: None,
    })
}

#[tauri::command]
pub fn get_claude_settings() -> Result<Value, String> {
    ConfigService::read_claude_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_claude_settings(settings: Value) -> Result<(), String> {
    ConfigService::save_claude_settings(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_claude_schema() -> Result<Value, String> {
    ConfigService::get_claude_schema().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_codex_settings() -> Result<CodexSettingsPayload, String> {
    ConfigService::read_codex_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_codex_settings(settings: Value, auth_token: Option<String>) -> Result<(), String> {
    ConfigService::save_codex_settings(&settings, auth_token).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_codex_schema() -> Result<Value, String> {
    ConfigService::get_codex_schema().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_gemini_settings() -> Result<GeminiSettingsPayload, String> {
    ConfigService::read_gemini_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_gemini_settings(settings: Value, env: GeminiEnvPayload) -> Result<(), String> {
    ConfigService::save_gemini_settings(&settings, &env).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_gemini_schema() -> Result<Value, String> {
    ConfigService::get_gemini_schema().map_err(|e| e.to_string())
}

/// 读取指定配置文件的详情（不激活）
#[tauri::command]
pub async fn get_profile_config(tool: String, profile: String) -> Result<ActiveConfig, String> {
    let tool_obj = Tool::by_id(&tool).ok_or_else(|| format!("未知的工具: {tool}"))?;

    match tool.as_str() {
        "claude-code" => {
            let backup_path = tool_obj.backup_path(&profile);
            if !backup_path.exists() {
                return Err(format!("配置文件不存在: {profile}"));
            }

            // 读取备份配置文件
            let backup_content =
                fs::read_to_string(&backup_path).map_err(|e| format!("读取配置文件失败: {e}"))?;
            let backup_data: Value = serde_json::from_str(&backup_content)
                .map_err(|e| format!("解析配置文件失败: {e}"))?;

            // 兼容新旧格式读取 API Key
            let api_key = backup_data
                .get("ANTHROPIC_AUTH_TOKEN")
                .and_then(|v| v.as_str())
                .or_else(|| {
                    backup_data
                        .get("env")
                        .and_then(|env| env.get("ANTHROPIC_AUTH_TOKEN"))
                        .and_then(|v| v.as_str())
                })
                .ok_or_else(|| "配置文件格式错误：缺少 API Key".to_string())?;

            // 兼容新旧格式读取 Base URL
            let base_url = backup_data
                .get("ANTHROPIC_BASE_URL")
                .and_then(|v| v.as_str())
                .or_else(|| {
                    backup_data
                        .get("env")
                        .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
                        .and_then(|v| v.as_str())
                })
                .ok_or_else(|| "配置文件格式错误：缺少 Base URL".to_string())?;

            Ok(ActiveConfig {
                api_key: api_key.to_string(),
                base_url: base_url.to_string(),
                profile_name: Some(profile),
            })
        }
        _ => Err(format!("暂不支持的工具: {tool}")),
    }
}
