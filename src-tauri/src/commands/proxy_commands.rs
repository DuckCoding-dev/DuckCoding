// 代理相关命令

use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;

use crate::commands::profile_commands::ProfileManagerState;
use ::duckcoding::services::amp_native_config;
use ::duckcoding::services::proxy::ProxyManager;
use ::duckcoding::services::proxy_config_manager::ProxyConfigManager;
use ::duckcoding::utils::config::read_global_config;

// ==================== 类型定义 ====================

// 代理管理器状态（新架构）
pub struct ProxyManagerState {
    pub manager: Arc<ProxyManager>,
}

// 透明代理状态（用于新架构的多工具状态返回）
#[derive(serde::Serialize)]
pub struct TransparentProxyStatus {
    running: bool,
    port: u16,
}

#[derive(serde::Deserialize)]
pub struct ProxyTestConfig {
    enabled: bool,
    proxy_type: String,
    host: String,
    port: String,
    username: Option<String>,
    password: Option<String>,
}

#[derive(serde::Serialize)]
pub struct TestProxyResult {
    success: bool,
    status: u16,
    url: Option<String>,
    error: Option<String>,
}

#[tauri::command]
pub fn get_current_proxy() -> Result<Option<String>, String> {
    Ok(::duckcoding::ProxyService::get_current_proxy())
}

// Add runtime command to re-apply proxy from saved config without recompiling
#[tauri::command]
pub fn apply_proxy_now() -> Result<Option<String>, String> {
    let cfg = read_global_config()?.ok_or_else(|| "config not found".to_string())?;
    ::duckcoding::ProxyService::apply_proxy_from_config(&cfg);
    Ok(::duckcoding::ProxyService::get_current_proxy())
}
#[tauri::command]
pub async fn test_proxy_request(
    test_url: String,
    proxy_config: ProxyTestConfig,
) -> Result<TestProxyResult, String> {
    // 根据代理配置构建客户端
    let client = if proxy_config.enabled {
        // 构建代理 URL
        let auth = if let (Some(username), Some(password)) =
            (&proxy_config.username, &proxy_config.password)
        {
            if !username.is_empty() && !password.is_empty() {
                format!("{username}:{password}@")
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let scheme = match proxy_config.proxy_type.as_str() {
            "socks5" => "socks5",
            "https" => "https",
            _ => "http",
        };

        let proxy_url = format!(
            "{}://{}{}:{}",
            scheme, auth, proxy_config.host, proxy_config.port
        );

        tracing::debug!(
            proxy_url = %proxy_url.replace(&auth, "***:***@"),
            "测试代理请求"
        );

        // 构建带代理的客户端
        match reqwest::Proxy::all(&proxy_url) {
            Ok(proxy) => reqwest::Client::builder()
                .proxy(proxy)
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .map_err(|e| format!("Failed to build client with proxy: {e}"))?,
            Err(e) => {
                return Ok(TestProxyResult {
                    success: false,
                    status: 0,
                    url: None,
                    error: Some(format!("Invalid proxy URL: {e}")),
                });
            }
        }
    } else {
        // 不使用代理的客户端
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to build client: {e}"))?
    };

    match client.get(&test_url).send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let url_ret = resp.url().as_str().to_string();
            Ok(TestProxyResult {
                success: resp.status().is_success(),
                status,
                url: Some(url_ret),
                error: None,
            })
        }
        Err(e) => Ok(TestProxyResult {
            success: false,
            status: 0,
            url: None,
            error: Some(e.to_string()),
        }),
    }
}

// ==================== 多工具代理命令（新架构） ====================
/// 内部实现：尝试启动代理（支持回滚）
pub(crate) async fn try_start_proxy_internal(
    tool_id: &str,
    manager_state: &ProxyManagerState,
    profile_state: &ProfileManagerState,
) -> Result<(String, u16), String> {
    let profile_mgr = profile_state.manager.read().await;
    let proxy_config_mgr = ProxyConfigManager::new().map_err(|e| e.to_string())?;

    // 读取当前配置
    let mut tool_config = proxy_config_mgr
        .get_config(tool_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("工具 {} 的代理配置不存在", tool_id))?;

    // 检查是否已在运行
    if manager_state.manager.is_running(tool_id).await {
        return Err(format!("{} 代理已在运行", tool_id));
    }

    // 检查必要字段
    if !tool_config.enabled {
        return Err(format!("{} 的透明代理未启用", tool_id));
    }
    if tool_config.local_api_key.is_none() {
        return Err("透明代理保护密钥未设置".to_string());
    }

    // amp-code 验证：检查是否至少配置了一个工具的 Profile
    if tool_id == "amp-code" {
        let (claude, codex, gemini) = profile_mgr
            .resolve_amp_selection()
            .map_err(|e| format!("读取 AMP Code Profile 选择失败: {}", e))?;

        if claude.is_none() && codex.is_none() && gemini.is_none() {
            return Err(
                "AMP Code 未配置任何 Profile，请先在 Profile 管理页面选择至少一个工具的配置"
                    .to_string(),
            );
        }
    } else {
        // 其他工具需要 real_api_key/real_base_url
        if tool_config.real_api_key.is_none() || tool_config.real_base_url.is_none() {
            return Err("真实 API Key 或 Base URL 未设置".to_string());
        }
    }

    // ========== Profile 切换逻辑（amp-code 跳过，因为它动态路由到其他工具的 Profile） ==========

    if tool_id != "amp-code" {
        // 1. 读取当前激活的 Profile 名称
        let original_profile = profile_mgr
            .get_active_profile_name(tool_id)
            .map_err(|e| e.to_string())?;

        // 2. 保存到 ToolProxyConfig
        tool_config.original_active_profile = original_profile.clone();
        proxy_config_mgr
            .update_config(tool_id, tool_config.clone())
            .map_err(|e| e.to_string())?;

        // 3. 验证内置 Profile 是否存在
        let proxy_profile_name = format!("dc_proxy_{}", tool_id.replace("-", "_"));

        let profile_exists = match tool_id {
            "claude-code" => profile_mgr.get_claude_profile(&proxy_profile_name).is_ok(),
            "codex" => profile_mgr.get_codex_profile(&proxy_profile_name).is_ok(),
            "gemini-cli" => profile_mgr.get_gemini_profile(&proxy_profile_name).is_ok(),
            _ => false,
        };

        if !profile_exists {
            return Err(format!(
                "内置 Profile 不存在，请先保存代理配置: {}",
                proxy_profile_name
            ));
        }

        // 4. 激活内置 Profile（这会自动同步到原生配置文件）
        profile_mgr
            .activate_profile(tool_id, &proxy_profile_name)
            .map_err(|e| format!("激活内置 Profile 失败: {}", e))?;

        tracing::info!(
            tool_id = %tool_id,
            original_profile = ?original_profile,
            proxy_profile = %proxy_profile_name,
            "已切换到代理 Profile"
        );
    } else {
        // amp-code：直接修改 AMP Code 原生配置文件
        let proxy_url = format!("http://127.0.0.1:{}", tool_config.port);
        let local_key = tool_config
            .local_api_key
            .as_ref()
            .ok_or_else(|| "透明代理保护密钥未设置".to_string())?;

        // 1. 完整备份当前 AMP Code 配置
        let backup = amp_native_config::backup_amp_config()
            .map_err(|e| format!("备份 AMP Code 配置失败: {}", e))?;

        tool_config.original_amp_settings = backup.settings;
        tool_config.original_amp_secrets = backup.secrets;

        // 2. 保存备份到 proxy.json
        proxy_config_mgr
            .update_config(tool_id, tool_config.clone())
            .map_err(|e| e.to_string())?;

        // 3. 应用代理配置
        amp_native_config::apply_proxy_config(&proxy_url, local_key)
            .map_err(|e| format!("应用 AMP Code 代理配置失败: {}", e))?;

        tracing::info!(
            tool_id = %tool_id,
            proxy_url = %proxy_url,
            has_original_settings = tool_config.original_amp_settings.is_some(),
            has_original_secrets = tool_config.original_amp_secrets.is_some(),
            "已应用 AMP Code 代理配置"
        );
    }

    // ========== 启动代理 ==========

    let proxy_port = tool_config.port;

    manager_state
        .manager
        .start_proxy(tool_id, tool_config)
        .await
        .map_err(|e| format!("启动代理失败: {}", e))?;

    Ok((tool_id.to_string(), proxy_port))
}

/// 启动指定工具的透明代理（带事务回滚）
pub(crate) async fn start_tool_proxy_internal(
    tool_id: &str,
    manager_state: &ProxyManagerState,
    profile_state: &ProfileManagerState,
) -> Result<String, String> {
    // 备份当前状态（用于回滚）
    let profile_mgr = profile_state.manager.read().await;
    let proxy_config_mgr = ProxyConfigManager::new().map_err(|e| e.to_string())?;

    let backup_config = proxy_config_mgr
        .get_config(tool_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("工具 {} 的代理配置不存在", tool_id))?;

    let backup_profile = profile_mgr
        .get_active_profile_name(tool_id)
        .map_err(|e| e.to_string())?;

    // 执行启动操作
    match try_start_proxy_internal(tool_id, manager_state, profile_state).await {
        Ok((tool_id, proxy_port)) => Ok(format!(
            "✅ {} 透明代理已启动\n监听端口: {}\n已切换到代理配置",
            tool_id, proxy_port
        )),
        Err(e) => {
            // 启动失败，开始回滚
            tracing::warn!("代理启动失败，开始回滚: {}", e);

            // 回滚代理配置
            if let Err(rollback_err) = proxy_config_mgr.update_config(tool_id, backup_config) {
                tracing::error!("回滚代理配置失败: {}", rollback_err);
            } else {
                tracing::info!("已回滚代理配置");
            }

            // 回滚 Profile 激活状态（需要写锁）
            drop(profile_mgr); // 释放读锁
            let profile_mgr_write = profile_state.manager.write().await;
            if let Some(name) = backup_profile {
                if let Err(rollback_err) = profile_mgr_write.activate_profile(tool_id, &name) {
                    tracing::error!("回滚 Profile 失败: {}", rollback_err);
                } else {
                    tracing::info!("已回滚 Profile 到: {}", name);
                }
            }

            Err(e)
        }
    }
}

#[tauri::command]
pub async fn start_tool_proxy(
    tool_id: String,
    manager_state: State<'_, ProxyManagerState>,
    profile_state: State<'_, ProfileManagerState>,
) -> Result<String, String> {
    start_tool_proxy_internal(&tool_id, &manager_state, &profile_state).await
}

/// 停止指定工具的透明代理
pub(crate) async fn stop_tool_proxy_internal(
    tool_id: &str,
    manager_state: &ProxyManagerState,
    profile_state: &ProfileManagerState,
) -> Result<String, String> {
    let profile_mgr = profile_state.manager.write().await;
    let proxy_config_mgr = ProxyConfigManager::new().map_err(|e| e.to_string())?;

    // 读取代理配置
    let mut tool_config = proxy_config_mgr
        .get_config(tool_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("工具 {} 的代理配置不存在", tool_id))?;

    // ========== 停止代理 ==========

    manager_state
        .manager
        .stop_proxy(tool_id)
        .await
        .map_err(|e| format!("停止代理失败: {e}"))?;

    // ========== 还原逻辑 ==========

    if tool_id == "amp-code" {
        // amp-code：完整还原 AMP Code 原生配置文件
        let backup = amp_native_config::AmpConfigBackup {
            settings: tool_config.original_amp_settings.take(),
            secrets: tool_config.original_amp_secrets.take(),
        };

        amp_native_config::restore_amp_config(&backup)
            .map_err(|e| format!("还原 AMP Code 配置失败: {}", e))?;

        // 清空备份字段
        proxy_config_mgr
            .update_config(tool_id, tool_config)
            .map_err(|e| e.to_string())?;

        tracing::info!(
            tool_id = %tool_id,
            had_settings = backup.settings.is_some(),
            had_secrets = backup.secrets.is_some(),
            "已完整还原 AMP Code 配置"
        );

        return Ok(format!(
            "✅ {tool_id} 透明代理已停止\n已完整还原 AMP Code 配置"
        ));
    }

    // 其他工具：Profile 还原逻辑
    let original_profile = tool_config.original_active_profile.take();

    if let Some(profile_name) = original_profile {
        // 有原始 Profile，切回去
        profile_mgr
            .activate_profile(tool_id, &profile_name)
            .map_err(|e| format!("还原 Profile 失败: {}", e))?;

        tracing::info!(
            tool_id = %tool_id,
            restored_profile = %profile_name,
            "已还原到原始 Profile"
        );

        // 清空 original_active_profile 字段
        proxy_config_mgr
            .update_config(tool_id, tool_config)
            .map_err(|e| e.to_string())?;

        Ok(format!(
            "✅ {tool_id} 透明代理已停止\n已还原到 Profile: {profile_name}"
        ))
    } else {
        // 没有原始 Profile（启动代理前用户就没激活任何 Profile）
        // 按需求：不做任何操作，保持当前状态
        tracing::info!(
            tool_id = %tool_id,
            "启动代理前无激活 Profile，保持当前状态"
        );

        Ok(format!("✅ {tool_id} 透明代理已停止"))
    }
}

#[tauri::command]
pub async fn stop_tool_proxy(
    tool_id: String,
    manager_state: State<'_, ProxyManagerState>,
    profile_state: State<'_, ProfileManagerState>,
) -> Result<String, String> {
    stop_tool_proxy_internal(&tool_id, &manager_state, &profile_state).await
}

/// 获取所有工具的透明代理状态
#[tauri::command]
pub async fn get_all_proxy_status(
    manager_state: State<'_, ProxyManagerState>,
) -> Result<HashMap<String, TransparentProxyStatus>, String> {
    let proxy_config_mgr = ProxyConfigManager::new().map_err(|e| e.to_string())?;
    let proxy_store = proxy_config_mgr
        .load_proxy_store()
        .map_err(|e| e.to_string())?;

    let mut status_map = HashMap::new();

    for tool_id in &["claude-code", "codex", "gemini-cli", "amp-code"] {
        let port = proxy_store
            .get_config(tool_id)
            .map(|tc| tc.port)
            .unwrap_or_else(|| match *tool_id {
                "claude-code" => 8787,
                "codex" => 8788,
                "gemini-cli" => 8789,
                "amp-code" => 8790,
                _ => 8790,
            });

        let running = manager_state.manager.is_running(tool_id).await;

        status_map.insert(
            tool_id.to_string(),
            TransparentProxyStatus { running, port },
        );
    }

    Ok(status_map)
}

/// 从 Profile 更新代理配置（不激活 Profile）
pub(crate) async fn update_proxy_from_profile_internal(
    tool_id: &str,
    profile_name: &str,
    manager_state: &ProxyManagerState,
    profile_state: &ProfileManagerState,
) -> Result<(), String> {
    let profile_mgr = profile_state.manager.read().await;
    let proxy_config_mgr = ProxyConfigManager::new().map_err(|e| e.to_string())?;

    // 根据工具类型读取 Profile
    let (api_key, base_url, pricing_template_id) = match tool_id {
        "claude-code" => {
            let profile = profile_mgr
                .get_claude_profile(profile_name)
                .map_err(|e| e.to_string())?;
            (
                profile.api_key,
                profile.base_url,
                profile.pricing_template_id,
            )
        }
        "codex" => {
            let profile = profile_mgr
                .get_codex_profile(profile_name)
                .map_err(|e| e.to_string())?;
            (
                profile.api_key,
                profile.base_url,
                profile.pricing_template_id,
            )
        }
        "gemini-cli" => {
            let profile = profile_mgr
                .get_gemini_profile(profile_name)
                .map_err(|e| e.to_string())?;
            (
                profile.api_key,
                profile.base_url,
                profile.pricing_template_id,
            )
        }
        _ => return Err(format!("不支持的工具: {}", tool_id)),
    };

    // 更新代理配置的 real_* 字段
    let mut proxy_config = proxy_config_mgr
        .get_config(tool_id)
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| {
            use ::duckcoding::models::proxy_config::ToolProxyConfig;
            ToolProxyConfig::new(ToolProxyConfig::default_port(tool_id))
        });

    proxy_config.real_api_key = Some(api_key);
    proxy_config.real_base_url = Some(base_url);
    proxy_config.real_profile_name = Some(profile_name.to_string());
    proxy_config.pricing_template_id = pricing_template_id; // Phase 6: 价格模板

    proxy_config_mgr
        .update_config(tool_id, proxy_config.clone())
        .map_err(|e| e.to_string())?;

    // 如果代理正在运行，通知 ProxyManager 重新加载
    if manager_state.manager.is_running(tool_id).await {
        manager_state
            .manager
            .update_config(tool_id, proxy_config)
            .await
            .map_err(|e| e.to_string())?;
        tracing::info!("已更新运行中的代理配置: {} -> {}", tool_id, profile_name);
    }

    Ok(())
}

#[tauri::command]
pub async fn update_proxy_from_profile(
    tool_id: String,
    profile_name: String,
    manager_state: State<'_, ProxyManagerState>,
    profile_state: State<'_, ProfileManagerState>,
) -> Result<(), String> {
    update_proxy_from_profile_internal(&tool_id, &profile_name, &manager_state, &profile_state)
        .await
}

/// 获取指定工具的代理配置
#[tauri::command]
pub async fn get_proxy_config(
    tool_id: String,
) -> Result<Option<::duckcoding::models::proxy_config::ToolProxyConfig>, String> {
    let proxy_mgr = ProxyConfigManager::new().map_err(|e| e.to_string())?;
    proxy_mgr.get_config(&tool_id).map_err(|e| e.to_string())
}

/// 更新指定工具的代理配置
#[tauri::command]
pub async fn update_proxy_config(
    tool_id: String,
    config: ::duckcoding::models::proxy_config::ToolProxyConfig,
    manager_state: State<'_, ProxyManagerState>,
    profile_state: State<'_, ProfileManagerState>,
) -> Result<(), String> {
    // ========== 运行时保护检查 ==========
    if manager_state.manager.is_running(&tool_id).await {
        return Err(format!("{} 代理正在运行，请先停止代理再修改配置", tool_id));
    }

    // ========== 更新配置到全局配置文件 ==========
    let proxy_mgr = ProxyConfigManager::new().map_err(|e| e.to_string())?;
    proxy_mgr
        .update_config(&tool_id, config.clone())
        .map_err(|e| e.to_string())?;

    // ========== 同步创建/更新内置 Profile ==========

    // 只有在配置完整时才创建内置 Profile
    if config.enabled
        && config.local_api_key.is_some()
        && config.real_api_key.is_some()
        && config.real_base_url.is_some()
    {
        let profile_mgr = profile_state.manager.write().await;
        let proxy_profile_name = format!("dc_proxy_{}", tool_id.replace("-", "_"));
        let proxy_endpoint = format!("http://127.0.0.1:{}", config.port);

        // 安全获取代理密钥，避免 panic
        let proxy_key = config
            .local_api_key
            .as_ref()
            .ok_or_else(|| format!("工具 {} 缺少代理密钥配置", tool_id))?
            .clone();

        match tool_id.as_str() {
            "claude-code" => {
                profile_mgr
                    .save_claude_profile_internal(&proxy_profile_name, proxy_key, proxy_endpoint)
                    .map_err(|e| format!("同步内置 Profile 失败: {}", e))?;
            }
            "codex" => {
                profile_mgr
                    .save_codex_profile_internal(
                        &proxy_profile_name,
                        proxy_key,
                        proxy_endpoint,
                        Some("responses".to_string()),
                    )
                    .map_err(|e| format!("同步内置 Profile 失败: {}", e))?;
            }
            "gemini-cli" => {
                profile_mgr
                    .save_gemini_profile_internal(
                        &proxy_profile_name,
                        proxy_key,
                        proxy_endpoint,
                        None, // 不设置 model，保留用户原有配置
                    )
                    .map_err(|e| format!("同步内置 Profile 失败: {}", e))?;
            }
            "amp-code" => {
                // AMP 不需要创建内置 Profile，配置已保存到 proxy.json
                tracing::debug!(tool_id = %tool_id, "AMP 代理配置已保存，跳过 Profile 同步");
            }
            _ => return Err(format!("不支持的工具: {}", tool_id)),
        }

        tracing::info!(
            tool_id = %tool_id,
            proxy_profile = %proxy_profile_name,
            port = config.port,
            "已同步更新内置 Profile"
        );
    }

    Ok(())
}

/// 获取所有工具的代理配置
#[tauri::command]
pub async fn get_all_proxy_configs(
) -> Result<::duckcoding::models::proxy_config::ProxyStore, String> {
    let proxy_mgr = ProxyConfigManager::new().map_err(|e| e.to_string())?;
    proxy_mgr.get_all_configs().map_err(|e| e.to_string())
}
