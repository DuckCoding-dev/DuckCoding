// 代理相关命令

use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex as TokioMutex;

use ::duckcoding::services::proxy::{
    ProxyManager, TransparentProxyConfigService, TransparentProxyService,
};
use ::duckcoding::utils::config::{read_global_config, write_global_config};
use ::duckcoding::{GlobalConfig, ProxyConfig, Tool};

// ==================== 类型定义 ====================

// 透明代理全局状态（旧架构，保持兼容）
pub struct TransparentProxyState {
    pub service: Arc<TokioMutex<TransparentProxyService>>,
}

// 代理管理器状态（新架构）
pub struct ProxyManagerState {
    pub manager: Arc<ProxyManager>,
}

// 透明代理相关的 Tauri Commands
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

// ==================== 辅助函数 ====================

// Tauri命令：读取全局配置
async fn get_global_config() -> Result<Option<GlobalConfig>, String> {
    read_global_config()
}

// Tauri命令：保存全局配置
async fn save_global_config(config: GlobalConfig) -> Result<(), String> {
    write_global_config(&config)
}
#[tauri::command]
pub async fn start_transparent_proxy(
    state: State<'_, TransparentProxyState>,
) -> Result<String, String> {
    // 读取全局配置
    let mut config = get_global_config()
        .await
        .map_err(|e| format!("读取配置失败: {e}"))?
        .ok_or_else(|| "全局配置不存在，请先配置用户信息".to_string())?;
    let original_config = config.clone();

    if !config.transparent_proxy_enabled {
        return Err("透明代理未启用，请先在设置中启用".to_string());
    }

    let local_api_key = config
        .transparent_proxy_api_key
        .clone()
        .ok_or_else(|| "透明代理保护密钥未设置".to_string())?;

    let proxy_port = config.transparent_proxy_port;

    let tool = Tool::claude_code();

    // 每次启动都检查并确保配置正确设置
    // 如果还没有备份过真实配置，先备份
    if config.transparent_proxy_real_api_key.is_none() {
        // 启用透明代理（保存真实配置并修改 ClaudeCode 配置）
        TransparentProxyConfigService::enable_transparent_proxy(
            &tool,
            &mut config,
            proxy_port,
            &local_api_key,
        )
        .map_err(|e| format!("启用透明代理失败: {e}"))?;
    } else {
        // 已经备份过配置，只需确保当前配置指向本地代理
        TransparentProxyConfigService::update_config_to_proxy(&tool, proxy_port, &local_api_key)
            .map_err(|e| format!("更新代理配置失败: {e}"))?;
    }

    // 从全局配置获取真实的 API 配置
    let (target_api_key, target_base_url) = TransparentProxyConfigService::get_real_config(&config)
        .map_err(|e| format!("获取真实配置失败: {e}"))?;

    println!(
        "🔑 真实 API Key: {}...",
        &target_api_key[..4.min(target_api_key.len())]
    );
    println!("🌐 真实 Base URL: {target_base_url}");

    // 创建代理配置
    let proxy_config = ProxyConfig {
        target_api_key,
        target_base_url,
        local_api_key,
    };

    // 启动代理服务
    let service = state.service.lock().await;
    let allow_public = config.transparent_proxy_allow_public;
    if let Err(start_err) = service.start(proxy_config, allow_public).await {
        if let Err(disable_err) =
            TransparentProxyConfigService::disable_transparent_proxy(&tool, &config)
        {
            eprintln!("恢复 ClaudeCode 配置失败（代理启动错误后）: {disable_err}");
        }
        if let Err(save_err) = save_global_config(original_config).await {
            eprintln!("恢复全局配置失败（代理启动错误后）: {save_err}");
        }
        return Err(format!("启动透明代理服务失败: {start_err}"));
    }

    // 保存更新后的全局配置
    save_global_config(config.clone())
        .await
        .map_err(|e| format!("保存配置失败: {e}"))?;

    Ok(format!(
        "✅ 透明代理已启动\n监听端口: {proxy_port}\nClaudeCode 请求将自动转发"
    ))
}

#[tauri::command]
pub async fn stop_transparent_proxy(
    state: State<'_, TransparentProxyState>,
) -> Result<String, String> {
    // 读取全局配置
    let config = get_global_config()
        .await
        .map_err(|e| format!("读取配置失败: {e}"))?
        .ok_or_else(|| "全局配置不存在".to_string())?;

    // 停止代理服务
    let service = state.service.lock().await;
    service
        .stop()
        .await
        .map_err(|e| format!("停止透明代理服务失败: {e}"))?;

    // 恢复 ClaudeCode 配置
    if config.transparent_proxy_real_api_key.is_some() {
        let tool = Tool::claude_code();
        TransparentProxyConfigService::disable_transparent_proxy(&tool, &config)
            .map_err(|e| format!("恢复配置失败: {e}"))?;
    }

    Ok("✅ 透明代理已停止\nClaudeCode 配置已恢复".to_string())
}

#[tauri::command]
pub async fn get_transparent_proxy_status(
    state: State<'_, TransparentProxyState>,
) -> Result<TransparentProxyStatus, String> {
    let config = get_global_config().await.ok().flatten();
    let port = config
        .as_ref()
        .map(|c| c.transparent_proxy_port)
        .unwrap_or(8787);

    let service = state.service.lock().await;
    let running = service.is_running().await;

    Ok(TransparentProxyStatus { running, port })
}

#[tauri::command]
pub async fn update_transparent_proxy_config(
    state: State<'_, TransparentProxyState>,
    new_api_key: String,
    new_base_url: String,
) -> Result<String, String> {
    // 读取全局配置
    let mut config = get_global_config()
        .await
        .map_err(|e| format!("读取配置失败: {e}"))?
        .ok_or_else(|| "全局配置不存在".to_string())?;

    if !config.transparent_proxy_enabled {
        return Err("透明代理未启用".to_string());
    }

    let local_api_key = config
        .transparent_proxy_api_key
        .clone()
        .ok_or_else(|| "透明代理保护密钥未设置".to_string())?;

    // 更新全局配置中的真实配置
    let tool = Tool::claude_code();
    TransparentProxyConfigService::update_real_config(
        &tool,
        &mut config,
        &new_api_key,
        &new_base_url,
    )
    .map_err(|e| format!("更新配置失败: {e}"))?;

    // 保存更新后的全局配置
    save_global_config(config.clone())
        .await
        .map_err(|e| format!("保存配置失败: {e}"))?;

    // 创建新的代理配置
    let proxy_config = ProxyConfig {
        target_api_key: new_api_key.clone(),
        target_base_url: new_base_url.clone(),
        local_api_key,
    };

    // 更新代理服务的配置
    let service = state.service.lock().await;
    service
        .update_config(proxy_config)
        .await
        .map_err(|e| format!("更新代理配置失败: {e}"))?;

    println!("🔄 透明代理配置已更新:");
    println!(
        "   API Key: {}...",
        &new_api_key[..4.min(new_api_key.len())]
    );
    println!("   Base URL: {new_base_url}");

    Ok("✅ 透明代理配置已更新，无需重启".to_string())
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

        println!(
            "Testing with proxy: {}",
            proxy_url.replace(&auth, "***:***@")
        ); // 隐藏密码

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

/// 启动指定工具的透明代理
#[tauri::command]
pub async fn start_tool_proxy(
    tool_id: String,
    manager_state: State<'_, ProxyManagerState>,
) -> Result<String, String> {
    // 读取全局配置
    let mut config = get_global_config()
        .await
        .map_err(|e| format!("读取配置失败: {e}"))?
        .ok_or_else(|| "全局配置不存在，请先配置用户信息".to_string())?;

    // 确保工具的代理配置存在
    let default_ports: HashMap<&str, u16> =
        [("claude-code", 8787), ("codex", 8788), ("gemini-cli", 8789)]
            .iter()
            .cloned()
            .collect();

    let default_port = default_ports.get(tool_id.as_str()).copied().unwrap_or(8790);
    config.ensure_proxy_config(&tool_id, default_port);

    // 获取工具的代理配置
    let tool_config = config
        .get_proxy_config(&tool_id)
        .ok_or_else(|| format!("工具 {tool_id} 的代理配置不存在"))?
        .clone();

    // 检查是否启用
    if !tool_config.enabled {
        return Err(format!("{tool_id} 的透明代理未启用，请先在设置中启用"));
    }

    // 保存端口用于后续消息
    let proxy_port = tool_config.port;

    // 获取工具定义
    let tool = Tool::by_id(&tool_id).ok_or_else(|| format!("未知工具: {tool_id}"))?;

    // 如果还没有备份过真实配置，先备份
    let updated_config = if tool_config.real_api_key.is_none() {
        let local_api_key = tool_config
            .local_api_key
            .clone()
            .ok_or_else(|| "透明代理保护密钥未设置".to_string())?;

        TransparentProxyConfigService::enable_transparent_proxy(
            &tool,
            &mut config,
            tool_config.port,
            &local_api_key,
        )
        .map_err(|e| format!("启用透明代理失败: {e}"))?;

        // 保存更新后的配置
        save_global_config(config.clone())
            .await
            .map_err(|e| format!("保存配置失败: {e}"))?;

        config
            .get_proxy_config(&tool_id)
            .ok_or_else(|| "配置保存后丢失".to_string())?
            .clone()
    } else {
        // 已经备份过配置，只需确保当前配置指向本地代理
        let local_api_key = tool_config
            .local_api_key
            .clone()
            .ok_or_else(|| "透明代理保护密钥未设置".to_string())?;

        TransparentProxyConfigService::update_config_to_proxy(
            &tool,
            tool_config.port,
            &local_api_key,
        )
        .map_err(|e| format!("更新代理配置失败: {e}"))?;

        tool_config
    };

    // 启动代理
    manager_state
        .manager
        .start_proxy(&tool_id, updated_config)
        .await
        .map_err(|e| format!("启动代理失败: {e}"))?;

    Ok(format!(
        "✅ {tool_id} 透明代理已启动\n监听端口: {proxy_port}\n请求将自动转发"
    ))
}

/// 停止指定工具的透明代理
#[tauri::command]
pub async fn stop_tool_proxy(
    tool_id: String,
    manager_state: State<'_, ProxyManagerState>,
) -> Result<String, String> {
    // 读取全局配置
    let config = get_global_config()
        .await
        .map_err(|e| format!("读取配置失败: {e}"))?
        .ok_or_else(|| "全局配置不存在".to_string())?;

    // 停止代理
    manager_state
        .manager
        .stop_proxy(&tool_id)
        .await
        .map_err(|e| format!("停止代理失败: {e}"))?;

    // 恢复工具配置
    if let Some(tool_config) = config.get_proxy_config(&tool_id) {
        if tool_config.real_api_key.is_some() {
            let tool = Tool::by_id(&tool_id).ok_or_else(|| format!("未知工具: {tool_id}"))?;

            TransparentProxyConfigService::disable_transparent_proxy(&tool, &config)
                .map_err(|e| format!("恢复配置失败: {e}"))?;
        }
    }

    Ok(format!("✅ {tool_id} 透明代理已停止\n配置已恢复"))
}

/// 获取所有工具的透明代理状态
#[tauri::command]
pub async fn get_all_proxy_status(
    manager_state: State<'_, ProxyManagerState>,
) -> Result<HashMap<String, TransparentProxyStatus>, String> {
    let config = get_global_config().await.ok().flatten();

    let mut status_map = HashMap::new();

    for tool_id in &["claude-code", "codex", "gemini-cli"] {
        let port = config
            .as_ref()
            .and_then(|c| c.get_proxy_config(tool_id))
            .map(|tc| tc.port)
            .unwrap_or_else(|| match *tool_id {
                "claude-code" => 8787,
                "codex" => 8788,
                "gemini-cli" => 8789,
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
