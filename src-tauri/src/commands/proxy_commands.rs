// 代理相关命令

use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::Mutex as TokioMutex;
use std::fs;
use tauri::State;

use ::duckcoding::{GlobalConfig, ProxyConfig, Tool};
use ::duckcoding::services::proxy::{TransparentProxyConfigService, TransparentProxyService};

// ==================== 类型定义 ====================

// 透明代理全局状态
pub struct TransparentProxyState {
    pub service: Arc<TokioMutex<TransparentProxyService>>,
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

// 全局配置辅助函数
fn get_global_config_path() -> Result<PathBuf, String> {
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;
    let config_dir = home_dir.join(".duckcoding");
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    Ok(config_dir.join("config.json"))
}

// Tauri命令：读取全局配置
async fn get_global_config() -> Result<Option<GlobalConfig>, String> {
    let config_path = get_global_config_path()?;

    if !config_path.exists() {
        return Ok(None);
    }

    let content =
        fs::read_to_string(&config_path).map_err(|e| format!("Failed to read config: {}", e))?;

    let config: GlobalConfig =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse config: {}", e))?;

    Ok(Some(config))
}

// Tauri命令：保存全局配置
async fn save_global_config(config: GlobalConfig) -> Result<(), String> {
    let config_path = get_global_config_path()?;

    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&config_path, json).map_err(|e| format!("Failed to write config: {}", e))?;

    println!("Config saved successfully");

    // 设置文件权限为仅所有者可读写（Unix系统）
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(&config_path)
            .map_err(|e| format!("Failed to get file metadata: {}", e))?;
        let mut perms = metadata.permissions();
        perms.set_mode(0o600); // -rw-------
        fs::set_permissions(&config_path, perms)
            .map_err(|e| format!("Failed to set file permissions: {}", e))?;
    }

    // 立即应用代理配置到环境变量
    ::duckcoding::ProxyService::apply_proxy_from_config(&config);

    Ok(())
}
#[tauri::command]
pub async fn start_transparent_proxy(
    state: State<'_, TransparentProxyState>,
) -> Result<String, String> {
    // 读取全局配置
    let mut config = get_global_config()
        .await
        .map_err(|e| format!("读取配置失败: {}", e))?
        .ok_or_else(|| "全局配置不存在，请先配置用户信息".to_string())?;

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
        .map_err(|e| format!("启用透明代理失败: {}", e))?;

        // 保存更新后的全局配置
        save_global_config(config.clone())
            .await
            .map_err(|e| format!("保存配置失败: {}", e))?;
    } else {
        // 已经备份过配置，只需确保当前配置指向本地代理
        TransparentProxyConfigService::update_config_to_proxy(&tool, proxy_port, &local_api_key)
            .map_err(|e| format!("更新代理配置失败: {}", e))?;
    }

    // 从全局配置获取真实的 API 配置
    let (target_api_key, target_base_url) = TransparentProxyConfigService::get_real_config(&config)
        .map_err(|e| format!("获取真实配置失败: {}", e))?;

    println!(
        "🔑 真实 API Key: {}...",
        &target_api_key[..4.min(target_api_key.len())]
    );
    println!("🌐 真实 Base URL: {}", target_base_url);

    // 创建代理配置
    let proxy_config = ProxyConfig {
        target_api_key,
        target_base_url,
        local_api_key,
    };

    // 启动代理服务
    let service = state.service.lock().await;
    let allow_public = config.transparent_proxy_allow_public;
    service
        .start(proxy_config, allow_public)
        .await
        .map_err(|e| format!("启动透明代理服务失败: {}", e))?;

    Ok(format!(
        "✅ 透明代理已启动\n监听端口: {}\nClaudeCode 请求将自动转发",
        proxy_port
    ))
}

#[tauri::command]
pub async fn stop_transparent_proxy(
    state: State<'_, TransparentProxyState>,
) -> Result<String, String> {
    // 读取全局配置
    let config = get_global_config()
        .await
        .map_err(|e| format!("读取配置失败: {}", e))?
        .ok_or_else(|| "全局配置不存在".to_string())?;

    // 停止代理服务
    let service = state.service.lock().await;
    service
        .stop()
        .await
        .map_err(|e| format!("停止透明代理服务失败: {}", e))?;

    // 恢复 ClaudeCode 配置
    if config.transparent_proxy_real_api_key.is_some() {
        let tool = Tool::claude_code();
        TransparentProxyConfigService::disable_transparent_proxy(&tool, &config)
            .map_err(|e| format!("恢复配置失败: {}", e))?;
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
        .map_err(|e| format!("读取配置失败: {}", e))?
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
    .map_err(|e| format!("更新配置失败: {}", e))?;

    // 保存更新后的全局配置
    save_global_config(config.clone())
        .await
        .map_err(|e| format!("保存配置失败: {}", e))?;

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
        .map_err(|e| format!("更新代理配置失败: {}", e))?;

    println!("🔄 透明代理配置已更新:");
    println!(
        "   API Key: {}...",
        &new_api_key[..4.min(new_api_key.len())]
    );
    println!("   Base URL: {}", new_base_url);

    Ok("✅ 透明代理配置已更新，无需重启".to_string())
}
#[tauri::command]
pub fn get_current_proxy() -> Result<Option<String>, String> {
    Ok(::duckcoding::ProxyService::get_current_proxy())
}

// Add runtime command to re-apply proxy from saved config without recompiling
#[tauri::command]
pub fn apply_proxy_now() -> Result<Option<String>, String> {
    let config_path = get_global_config_path()?;
    if !config_path.exists() {
        return Err("config not found".to_string());
    }
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    let cfg: GlobalConfig =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse config: {}", e))?;

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
                format!("{}:{}@", username, password)
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
                .map_err(|e| format!("Failed to build client with proxy: {}", e))?,
            Err(e) => {
                return Ok(TestProxyResult {
                    success: false,
                    status: 0,
                    url: None,
                    error: Some(format!("Invalid proxy URL: {}", e)),
                });
            }
        }
    } else {
        // 不使用代理的客户端
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to build client: {}", e))?
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
