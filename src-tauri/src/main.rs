// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use serde_json::Value;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime, WebviewWindow,
};

// 导入服务层
use duckcoding::models::update::{PackageFormatInfo, PlatformInfo};
use duckcoding::{
    services::config::{CodexSettingsPayload, GeminiEnvPayload, GeminiSettingsPayload},
    ConfigService, InstallMethod, InstallerService, Tool, UpdateInfo, UpdateService, UpdateStatus,
    VersionService,
};
// Use the shared GlobalConfig from the library crate (models::config)
use duckcoding::GlobalConfig;
// 导入透明代理服务
use duckcoding::{ProxyConfig, TransparentProxyConfigService, TransparentProxyService};
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

// DuckCoding API 响应结构
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
struct GenerateApiKeyResult {
    success: bool,
    message: String,
    api_key: Option<String>,
}

// 用量统计数据结构
#[derive(serde::Deserialize, Serialize, Debug, Clone)]
struct UsageData {
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
struct UsageStatsResult {
    success: bool,
    message: String,
    data: Vec<UsageData>,
}

// 用户信息数据结构
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
struct UserQuotaResult {
    success: bool,
    message: String,
    total_quota: f64,
    used_quota: f64,
    remaining_quota: f64,
    request_count: i64,
}

// Windows特定：隐藏命令行窗口
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

const CLOSE_CONFIRM_EVENT: &str = "duckcoding://request-close-action";
const SINGLE_INSTANCE_EVENT: &str = "single-instance";

#[derive(Clone, Serialize)]
struct SingleInstancePayload {
    args: Vec<String>,
    cwd: String,
}

// 辅助函数：获取扩展的PATH环境变量
fn get_extended_path() -> String {
    #[cfg(target_os = "windows")]
    {
        let user_profile =
            env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".to_string());

        let mut system_paths = vec![
            // Claude Code 可能的安装路径
            format!("{}\\AppData\\Local\\Programs\\claude-code", user_profile),
            format!("{}\\AppData\\Roaming\\npm", user_profile),
            format!(
                "{}\\AppData\\Local\\Programs\\Python\\Python312",
                user_profile
            ),
            format!(
                "{}\\AppData\\Local\\Programs\\Python\\Python312\\Scripts",
                user_profile
            ),
            // 常见安装路径
            "C:\\Program Files\\nodejs".to_string(),
            "C:\\Program Files\\Git\\cmd".to_string(),
            // 系统路径
            "C:\\Windows\\System32".to_string(),
            "C:\\Windows".to_string(),
        ];

        // nvm-windows支持
        if let Ok(nvm_home) = env::var("NVM_HOME") {
            system_paths.insert(0, format!("{}\\current", nvm_home));
        }

        let current_path = env::var("PATH").unwrap_or_default();
        format!("{};{}", system_paths.join(";"), current_path)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home_dir = env::var("HOME").unwrap_or_else(|_| "/Users/default".to_string());

        let mut system_paths = vec![
            // Claude Code 可能的安装路径
            format!("{}/.local/bin", home_dir),
            format!("{}/.claude/bin", home_dir),
            format!("{}/.claude/local", home_dir), // Claude Code local安装
            // Homebrew
            "/opt/homebrew/bin".to_string(),
            "/usr/local/bin".to_string(),
            // 系统路径
            "/usr/bin".to_string(),
            "/bin".to_string(),
            "/usr/sbin".to_string(),
            "/sbin".to_string(),
        ];

        // nvm支持 - 优先使用当前激活的版本
        if let Ok(nvm_dir) = env::var("NVM_DIR") {
            // 检查nvm current symlink
            let nvm_current = format!("{}/current/bin", nvm_dir);
            if std::path::Path::new(&nvm_current).exists() {
                system_paths.insert(0, nvm_current);
            } else {
                // 如果没有current symlink，尝试读取.nvmrc或使用default
                let nvm_default = format!("{}/.nvm/versions/node/default/bin", home_dir);
                if std::path::Path::new(&nvm_default).exists() {
                    system_paths.insert(0, nvm_default);
                }
            }
        } else {
            // 如果NVM_DIR未设置，尝试默认路径
            let nvm_current = format!("{}/.nvm/current/bin", home_dir);
            if std::path::Path::new(&nvm_current).exists() {
                system_paths.insert(0, nvm_current);
            }
        }

        format!(
            "{}:{}",
            system_paths.join(":"),
            env::var("PATH").unwrap_or_default()
        )
    }
}

//定义 Tauri Commands
#[tauri::command]
async fn check_installations() -> Result<Vec<ToolStatus>, String> {
    let installer = InstallerService::new();
    let mut result = Vec::new();

    for tool in Tool::all() {
        let installed = installer.is_installed(&tool).await;
        let version = if installed {
            installer.get_installed_version(&tool).await
        } else {
            None
        };

        result.push(ToolStatus {
            id: tool.id.clone(),
            name: tool.name.clone(),
            installed,
            version,
        });
    }

    Ok(result)
}

// 检测node环境
#[tauri::command]
async fn check_node_environment() -> Result<NodeEnvironment, String> {
    let run_command = |cmd: &str| -> Result<std::process::Output, std::io::Error> {
        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .env("PATH", get_extended_path())
                .arg("/C")
                .arg(cmd)
                .creation_flags(0x08000000) // CREATE_NO_WINDOW - 隐藏终端窗口
                .output()
        }
        #[cfg(not(target_os = "windows"))]
        {
            Command::new("sh")
                .env("PATH", get_extended_path())
                .arg("-c")
                .arg(cmd)
                .output()
        }
    };

    // 检测node
    let (node_available, node_version) = if let Ok(output) = run_command("node --version 2>&1") {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            (true, Some(version))
        } else {
            (false, None)
        }
    } else {
        (false, None)
    };

    // 检测npm
    let (npm_available, npm_version) = if let Ok(output) = run_command("npm --version 2>&1") {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            (true, Some(version))
        } else {
            (false, None)
        }
    } else {
        (false, None)
    };

    Ok(NodeEnvironment {
        node_available,
        node_version,
        npm_available,
        npm_version,
    })
}

// 辅助函数：从全局配置应用代理
async fn apply_proxy_if_configured() {
    if let Ok(Some(config)) = get_global_config().await {
        duckcoding::ProxyService::apply_proxy_from_config(&config);
    }
}

#[tauri::command]
async fn install_tool(
    tool: String,
    method: String,
    force: Option<bool>,
) -> Result<InstallResult, String> {
    // 应用代理配置（如果已配置）
    apply_proxy_if_configured().await;

    let force = force.unwrap_or(false);
    #[cfg(debug_assertions)]
    println!(
        "Installing {} via {} (using InstallerService, force={})",
        tool, method, force
    );

    // 获取工具定义
    let tool_obj =
        Tool::by_id(&tool).ok_or_else(|| "❌ 未知的工具\n\n请联系开发者报告此问题".to_string())?;

    // 转换安装方法
    let install_method = match method.as_str() {
        "npm" => InstallMethod::Npm,
        "brew" => InstallMethod::Brew,
        "official" => InstallMethod::Official,
        _ => return Err(format!("❌ 未知的安装方法: {}", method)),
    };

    // 使用 InstallerService 安装
    let installer = InstallerService::new();

    match installer.install(&tool_obj, &install_method, force).await {
        Ok(_) => {
            // 安装成功，构造成功消息
            let message = match method.as_str() {
                "npm" => format!("✅ {} 安装成功！(通过 npm)", tool_obj.name),
                "brew" => format!("✅ {} 安装成功！(通过 Homebrew)", tool_obj.name),
                "official" => format!("✅ {} 安装成功！", tool_obj.name),
                _ => format!("✅ {} 安装成功！", tool_obj.name),
            };

            Ok(InstallResult {
                success: true,
                message,
                output: String::new(),
            })
        }
        Err(e) => {
            // 安装失败，返回错误信息
            Err(e.to_string())
        }
    }
}

// 只检查更新，不执行
#[tauri::command]
async fn check_update(tool: String) -> Result<UpdateResult, String> {
    // 应用代理配置（如果已配置）
    apply_proxy_if_configured().await;

    #[cfg(debug_assertions)]
    println!("Checking updates for {} (using VersionService)", tool);

    let tool_obj = Tool::by_id(&tool).ok_or_else(|| format!("未知工具: {}", tool))?;

    let version_service = VersionService::new();

    match version_service.check_version(&tool_obj).await {
        Ok(version_info) => Ok(UpdateResult {
            success: true,
            message: "检查完成".to_string(),
            has_update: version_info.has_update,
            current_version: version_info.installed_version,
            latest_version: version_info.latest_version,
            mirror_version: version_info.mirror_version,
            mirror_is_stale: Some(version_info.mirror_is_stale),
            tool_id: Some(tool.clone()),
        }),
        Err(e) => {
            // 降级：如果检查失败，返回无法检查但不报错
            Ok(UpdateResult {
                success: true,
                message: format!("无法检查更新: {}", e),
                has_update: false,
                current_version: None,
                latest_version: None,
                mirror_version: None,
                mirror_is_stale: None,
                tool_id: Some(tool.clone()),
            })
        }
    }
}

// 批量检查所有工具更新（优化：单次网络请求）
#[tauri::command]
async fn check_all_updates() -> Result<Vec<UpdateResult>, String> {
    // 应用代理配置（如果已配置）
    apply_proxy_if_configured().await;

    #[cfg(debug_assertions)]
    println!("Checking updates for all tools (batch mode)");

    let version_service = VersionService::new();
    let version_infos = version_service.check_all_tools().await;

    let results = version_infos
        .into_iter()
        .map(|info| UpdateResult {
            success: true,
            message: "检查完成".to_string(),
            has_update: info.has_update,
            current_version: info.installed_version,
            latest_version: info.latest_version,
            mirror_version: info.mirror_version,
            mirror_is_stale: Some(info.mirror_is_stale),
            tool_id: Some(info.tool_id),
        })
        .collect();

    Ok(results)
}

#[tauri::command]
async fn update_tool(tool: String, force: Option<bool>) -> Result<UpdateResult, String> {
    // 应用代理配置（如果已配置）
    apply_proxy_if_configured().await;

    let force = force.unwrap_or(false);
    #[cfg(debug_assertions)]
    println!(
        "Updating {} (using InstallerService, force={})",
        tool, force
    );

    // 获取工具定义
    let tool_obj =
        Tool::by_id(&tool).ok_or_else(|| "❌ 未知的工具\n\n请联系开发者报告此问题".to_string())?;

    // 使用 InstallerService 更新（内部有120秒超时）
    let installer = InstallerService::new();

    // 执行更新，添加超时控制
    use tokio::time::{timeout, Duration};

    let update_result = timeout(Duration::from_secs(120), installer.update(&tool_obj, force)).await;

    match update_result {
        Ok(Ok(_)) => {
            // 更新成功，获取新版本
            let new_version = installer.get_installed_version(&tool_obj).await;

            Ok(UpdateResult {
                success: true,
                message: "✅ 更新成功！".to_string(),
                has_update: false,
                current_version: new_version.clone(),
                latest_version: new_version,
                mirror_version: None,
                mirror_is_stale: None,
                tool_id: Some(tool.clone()),
            })
        }
        Ok(Err(e)) => {
            // 更新失败，检查特殊错误情况
            let error_str = e.to_string();

            // 检查是否是 Homebrew 版本滞后
            if error_str.contains("Not upgrading") && error_str.contains("already installed") {
                return Err(
                    "⚠️ Homebrew版本滞后\n\nHomebrew cask的版本更新不及时，目前是旧版本。\n\n✅ 解决方案：\n\n方案1 - 使用npm安装最新版本（自动使用国内镜像）：\n1. 卸载Homebrew版本：brew uninstall --cask codex\n2. 安装npm版本：npm install -g @openai/codex --registry https://registry.npmmirror.com\n\n方案2 - 等待Homebrew cask更新\n（可能需要几天到几周时间）\n\n推荐使用方案1，npm版本更新更及时。".to_string()
                );
            }

            // 检查npm是否显示已经是最新版本
            if error_str.contains("up to date") {
                return Err(
                    "ℹ️ 已是最新版本\n\n当前安装的版本已经是最新版本，无需更新。".to_string(),
                );
            }

            // 检查是否是 npm 缓存权限错误
            if error_str.contains("EACCES") && error_str.contains(".npm") {
                return Err(
                    "⚠️ npm 权限问题\n\n这是因为之前使用 sudo npm 安装导致的。\n\n✅ 解决方案（任选其一）：\n\n方案1 - 修复 npm 权限（推荐）：\n在终端运行：\nsudo chown -R $(id -u):$(id -g) \"$HOME/.npm\"\n\n方案2 - 配置 npm 使用用户目录：\nnpm config set prefix ~/.npm-global\nexport PATH=~/.npm-global/bin:$PATH\n\n方案3 - macOS 用户切换到 Homebrew（无需 sudo）：\nbrew uninstall --cask codex\nbrew install --cask codex\n\n然后重试更新。".to_string()
                );
            }

            // 其他错误
            Err(error_str)
        }
        Err(_) => {
            // 超时
            Err("⏱️ 更新超时（120秒）\n\n可能的原因：\n• 网络连接不稳定\n• 服务器响应慢\n\n建议：\n1. 检查网络连接\n2. 重试更新\n3. 或尝试手动更新（详见文档）".to_string())
        }
    }
}

#[tauri::command]
async fn configure_api(
    tool: String,
    _provider: String,
    api_key: String,
    base_url: Option<String>,
    profile_name: Option<String>,
) -> Result<(), String> {
    #[cfg(debug_assertions)]
    println!("Configuring {} (using ConfigService)", tool);

    // 获取工具定义
    let tool_obj = Tool::by_id(&tool).ok_or_else(|| format!("❌ 未知的工具: {}", tool))?;

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
async fn list_profiles(tool: String) -> Result<Vec<String>, String> {
    #[cfg(debug_assertions)]
    println!("Listing profiles for {} (using ConfigService)", tool);

    // 获取工具定义
    let tool_obj = Tool::by_id(&tool).ok_or_else(|| format!("❌ 未知的工具: {}", tool))?;

    // 使用 ConfigService 列出配置
    ConfigService::list_profiles(&tool_obj).map_err(|e| e.to_string())
}

#[tauri::command]
async fn switch_profile(
    tool: String,
    profile: String,
    state: tauri::State<'_, TransparentProxyState>,
) -> Result<(), String> {
    #[cfg(debug_assertions)]
    println!(
        "Switching profile for {} to {} (using ConfigService)",
        tool, profile
    );

    // 获取工具定义
    let tool_obj = Tool::by_id(&tool).ok_or_else(|| format!("❌ 未知的工具: {}", tool))?;

    // 使用 ConfigService 激活配置
    ConfigService::activate_profile(&tool_obj, &profile).map_err(|e| e.to_string())?;

    // 如果是 ClaudeCode 且透明代理已启用，需要更新真实配置
    if tool == "claude-code" {
        // 读取全局配置
        if let Ok(Some(mut global_config)) = get_global_config().await {
            if global_config.transparent_proxy_enabled {
                // 读取切换后的真实配置
                let config_path = tool_obj.config_dir.join(&tool_obj.config_file);
                if config_path.exists() {
                    if let Ok(content) = fs::read_to_string(&config_path) {
                        if let Ok(settings) = serde_json::from_str::<Value>(&content) {
                            if let Some(env) = settings.get("env").and_then(|v| v.as_object()) {
                                let new_api_key = env
                                    .get("ANTHROPIC_AUTH_TOKEN")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                let new_base_url = env
                                    .get("ANTHROPIC_BASE_URL")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");

                                // 检查透明代理功能是否启用
                                let transparent_proxy_enabled =
                                    global_config.transparent_proxy_enabled;

                                if !new_api_key.is_empty() && !new_base_url.is_empty() {
                                    // 总是保存新的真实配置到全局配置（不管代理是否在运行）
                                    TransparentProxyConfigService::update_real_config(
                                        &tool_obj,
                                        &mut global_config,
                                        new_api_key,
                                        new_base_url,
                                    )
                                    .map_err(|e| format!("更新真实配置失败: {}", e))?;

                                    // 保存全局配置
                                    save_global_config(global_config.clone())
                                        .await
                                        .map_err(|e| format!("保存全局配置失败: {}", e))?;

                                    // 如果透明代理功能启用且代理服务正在运行，更新代理配置
                                    if transparent_proxy_enabled {
                                        let service = state.service.lock().await;
                                        if service.is_running().await {
                                            let local_api_key = global_config
                                                .transparent_proxy_api_key
                                                .clone()
                                                .unwrap_or_default();

                                            let proxy_config = ProxyConfig {
                                                target_api_key: new_api_key.to_string(),
                                                target_base_url: new_base_url.to_string(),
                                                local_api_key,
                                            };

                                            service
                                                .update_config(proxy_config)
                                                .await
                                                .map_err(|e| format!("更新代理配置失败: {}", e))?;

                                            println!("✅ 透明代理配置已自动更新");
                                            drop(service); // 释放锁
                                        } // 闭合 if service.is_running()
                                    } // 闭合 if transparent_proxy_enabled

                                    // 只有在透明代理功能启用时才恢复 ClaudeCode 配置指向本地代理
                                    if transparent_proxy_enabled {
                                        let local_proxy_port = global_config.transparent_proxy_port;
                                        let local_proxy_key = global_config
                                            .transparent_proxy_api_key
                                            .unwrap_or_default();

                                        let mut settings_mut = settings.clone();
                                        if let Some(env_mut) = settings_mut
                                            .get_mut("env")
                                            .and_then(|v| v.as_object_mut())
                                        {
                                            env_mut.insert(
                                                "ANTHROPIC_AUTH_TOKEN".to_string(),
                                                Value::String(local_proxy_key),
                                            );
                                            env_mut.insert(
                                                "ANTHROPIC_BASE_URL".to_string(),
                                                Value::String(format!(
                                                    "http://127.0.0.1:{}",
                                                    local_proxy_port
                                                )),
                                            );

                                            let json = serde_json::to_string_pretty(&settings_mut)
                                                .map_err(|e| format!("序列化配置失败: {}", e))?;
                                            fs::write(&config_path, json)
                                                .map_err(|e| format!("写入配置失败: {}", e))?;

                                            println!("✅ ClaudeCode 配置已恢复指向本地代理");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

#[tauri::command]
async fn delete_profile(tool: String, profile: String) -> Result<(), String> {
    #[cfg(debug_assertions)]
    println!("Deleting profile: tool={}, profile={}", tool, profile);

    // 获取工具定义
    let tool_obj = Tool::by_id(&tool).ok_or_else(|| format!("❌ 未知的工具: {}", tool))?;

    // 使用 ConfigService 删除配置
    ConfigService::delete_profile(&tool_obj, &profile).map_err(|e| e.to_string())?;

    #[cfg(debug_assertions)]
    println!("Successfully deleted profile: {}", profile);

    Ok(())
}

// 数据结构定义
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct ToolStatus {
    id: String,
    name: String,
    installed: bool,
    version: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct NodeEnvironment {
    node_available: bool,
    node_version: Option<String>,
    npm_available: bool,
    npm_version: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct InstallResult {
    success: bool,
    message: String,
    output: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct UpdateResult {
    success: bool,
    message: String,
    has_update: bool,
    current_version: Option<String>,
    latest_version: Option<String>,
    mirror_version: Option<String>, // 镜像实际可安装的版本
    mirror_is_stale: Option<bool>,  // 镜像是否滞后
    tool_id: Option<String>,        // 新增：工具ID，用于批量检查时识别工具
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ActiveConfig {
    api_key: String,
    base_url: String,
    profile_name: Option<String>, // 当前配置的名称
}

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

// Tauri命令：保存全局配置
#[tauri::command]
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
    duckcoding::ProxyService::apply_proxy_from_config(&config);

    Ok(())
}

// Tauri命令：读取全局配置
#[tauri::command]
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

// 生成API Key的主函数
#[tauri::command]
async fn generate_api_key_for_tool(tool: String) -> Result<GenerateApiKeyResult, String> {
    // 应用代理配置（如果已配置）
    apply_proxy_if_configured().await;

    // 读取全局配置
    let global_config = get_global_config()
        .await?
        .ok_or("请先配置用户ID和系统访问令牌")?;

    // 根据工具名称获取配置
    let (name, group) = match tool.as_str() {
        "claude-code" => ("Claude Code一键创建", "Claude Code专用"),
        "codex" => ("CodeX一键创建", "CodeX专用"),
        "gemini-cli" => ("Gemini CLI一键创建", "Gemini CLI专用"),
        _ => return Err(format!("Unknown tool: {}", tool)),
    };

    // 创建token
    let client = build_reqwest_client().map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;
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
        .map_err(|e| format!("创建token失败: {}", e))?;

    if !create_response.status().is_success() {
        let status = create_response.status();
        let error_text = create_response.text().await.unwrap_or_default();
        return Ok(GenerateApiKeyResult {
            success: false,
            message: format!("创建token失败 ({}): {}", status, error_text),
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
        .map_err(|e| format!("搜索token失败: {}", e))?;

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
        .map_err(|e| format!("解析响应失败: {}", e))?;

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

// 获取用户用量统计（近30天）
#[tauri::command]
async fn get_usage_stats() -> Result<UsageStatsResult, String> {
    // 应用代理配置（如果已配置）
    apply_proxy_if_configured().await;

    // 读取全局配置
    let global_config = get_global_config()
        .await?
        .ok_or("请先配置用户ID和系统访问令牌")?;

    // 计算时间戳（北京时间）
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // 今天的24:00:00（加上8小时时区偏移，然后取第二天的0点）
    let beijing_offset = 8 * 3600;
    let today_end = (now + beijing_offset) / 86400 * 86400 + 86400 - beijing_offset;

    // 30天前的00:00:00
    let start_timestamp = today_end - 30 * 86400;
    let end_timestamp = today_end;

    // 调用API
    let client = build_reqwest_client().map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;
    let url = format!(
        "https://duckcoding.com/api/data/self?start_timestamp={}&end_timestamp={}",
        start_timestamp, end_timestamp
    );

    let response = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .header("Accept", "application/json, text/plain, */*")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("Referer", "https://duckcoding.com/")
        .header("Origin", "https://duckcoding.com")
        .header(
            "Authorization",
            format!("Bearer {}", global_config.system_token),
        )
        .header("New-Api-User", &global_config.user_id)
        .send()
        .await
        .map_err(|e| format!("获取用量统计失败: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Ok(UsageStatsResult {
            success: false,
            message: format!("获取用量统计失败 ({}): {}", status, error_text),
            data: vec![],
        });
    }

    // 检查 Content-Type 是否为 JSON
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string()) // 克隆字符串，避免借用冲突
        .unwrap_or_default();

    if !content_type.contains("application/json") {
        return Ok(UsageStatsResult {
            success: false,
            message: format!(
                "服务器返回了非JSON格式的响应 (Content-Type: {})",
                content_type
            ),
            data: vec![],
        });
    }

    let api_response: UsageApiResponse = response
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

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

// 获取用户额度信息
#[tauri::command]
async fn get_user_quota() -> Result<UserQuotaResult, String> {
    // 应用代理配置（如果已配置）
    apply_proxy_if_configured().await;

    // 读取全局配置
    let global_config = get_global_config()
        .await?
        .ok_or("请先配置用户ID和系统访问令牌")?;

    // 调用API
    let client = build_reqwest_client().map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;
    let url = "https://duckcoding.com/api/user/self";

    let response = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .header("Accept", "application/json, text/plain, */*")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("Referer", "https://duckcoding.com/")
        .header("Origin", "https://duckcoding.com")
        .header(
            "Authorization",
            format!("Bearer {}", global_config.system_token),
        )
        .header("New-Api-User", &global_config.user_id)
        .send()
        .await
        .map_err(|e| format!("获取用户信息失败: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        println!("❌ [get_user_quota] HTTP Error: {}", error_text);
        return Err(format!("获取用户信息失败 ({}): {}", status, error_text));
    }

    // 检查 Content-Type 是否为 JSON
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string()) // 克隆字符串，避免借用冲突
        .unwrap_or_default();

    if !content_type.contains("application/json") {
        return Err(format!(
            "服务器返回了非JSON格式的响应 (Content-Type: {})",
            content_type
        ));
    }

    let api_response: UserApiResponse = response
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    if !api_response.success {
        return Err(format!("API返回错误: {}", api_response.message));
    }

    let user_info = api_response.data.ok_or("未获取到用户信息")?;

    // 修正：API返回的quota是剩余额度，不是总额度
    // 正确计算：总额度 = 剩余额度 + 已用额度
    let remaining_quota = user_info.quota as f64 / 500000.0;
    let used_quota = user_info.used_quota as f64 / 500000.0;
    let total_quota = remaining_quota + used_quota;

    #[cfg(debug_assertions)]
    {
        println!(
            "Raw remaining: {}, converted: {}",
            user_info.quota, remaining_quota
        );
        println!(
            "Raw used: {}, converted: {}",
            user_info.used_quota, used_quota
        );
        println!("Total quota: {}", total_quota);
    }

    Ok(UserQuotaResult {
        success: true,
        message: "获取成功".to_string(),
        total_quota,
        used_quota,
        remaining_quota,
        request_count: user_info.request_count,
    })
}

#[tauri::command]
fn handle_close_action(window: WebviewWindow, action: String) -> Result<(), String> {
    match action.as_str() {
        "minimize" => {
            hide_window_to_tray(&window);
            Ok(())
        }
        "quit" => {
            window.app_handle().exit(0);
            Ok(())
        }
        other => Err(format!("未知的关闭操作: {}", other)),
    }
}

#[tauri::command]
fn get_claude_settings() -> Result<Value, String> {
    ConfigService::read_claude_settings().map_err(|e| e.to_string())
}

#[tauri::command]
fn save_claude_settings(settings: Value) -> Result<(), String> {
    ConfigService::save_claude_settings(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_claude_schema() -> Result<Value, String> {
    ConfigService::get_claude_schema().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_codex_settings() -> Result<CodexSettingsPayload, String> {
    ConfigService::read_codex_settings().map_err(|e| e.to_string())
}

#[tauri::command]
fn save_codex_settings(settings: Value, auth_token: Option<String>) -> Result<(), String> {
    ConfigService::save_codex_settings(&settings, auth_token).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_codex_schema() -> Result<Value, String> {
    ConfigService::get_codex_schema().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_gemini_settings() -> Result<GeminiSettingsPayload, String> {
    ConfigService::read_gemini_settings().map_err(|e| e.to_string())
}

#[tauri::command]
fn save_gemini_settings(settings: Value, env: GeminiEnvPayload) -> Result<(), String> {
    ConfigService::save_gemini_settings(&settings, &env).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_gemini_schema() -> Result<Value, String> {
    ConfigService::get_gemini_schema().map_err(|e| e.to_string())
}

// 辅助函数：检测当前配置匹配哪个profile
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
                        let auth_backup = config_dir.join(format!("auth.{}.json", profile));

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

#[tauri::command]
async fn get_active_config(tool: String) -> Result<ActiveConfig, String> {
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
                fs::read_to_string(&config_path).map_err(|e| format!("❌ 读取配置失败: {}", e))?;
            let config: Value =
                serde_json::from_str(&content).map_err(|e| format!("❌ 解析配置失败: {}", e))?;

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
                    .map_err(|e| format!("❌ 读取认证文件失败: {}", e))?;
                let auth: Value = serde_json::from_str(&content)
                    .map_err(|e| format!("❌ 解析认证文件失败: {}", e))?;

                if let Some(key) = auth.get("OPENAI_API_KEY").and_then(|v| v.as_str()) {
                    raw_api_key = key.to_string();
                    api_key = mask_api_key(key);
                }
            }

            // 读取 config.toml
            if config_path.exists() {
                let content = fs::read_to_string(&config_path)
                    .map_err(|e| format!("❌ 读取配置文件失败: {}", e))?;
                let config: toml::Value =
                    toml::from_str(&content).map_err(|e| format!("❌ 解析TOML失败: {}", e))?;

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
                .map_err(|e| format!("❌ 读取环境变量配置失败: {}", e))?;

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
        _ => Err(format!("❌ 未知的工具: {}", tool)),
    }
}

fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        return "****".to_string();
    }
    let prefix = &key[..4];
    let suffix = &key[key.len() - 4..];
    format!("{}...{}", prefix, suffix)
}

fn create_tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[&show_item, &PredefinedMenuItem::separator(app)?, &quit_item],
    )?;

    Ok(menu)
}

fn focus_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        println!("Focusing existing main window");
        restore_window_state(&window);
    } else {
        println!("Main window not found when trying to focus");
    }
}

fn restore_window_state<R: Runtime>(window: &WebviewWindow<R>) {
    println!(
        "Restoring window state, is_visible={:?}, is_minimized={:?}",
        window.is_visible(),
        window.is_minimized()
    );

    #[cfg(target_os = "macos")]
    #[allow(deprecated)]
    {
        use cocoa::appkit::NSApplication;
        use cocoa::base::nil;
        use cocoa::foundation::NSAutoreleasePool;

        unsafe {
            let _pool = NSAutoreleasePool::new(nil);
            let app_macos = NSApplication::sharedApplication(nil);
            app_macos.setActivationPolicy_(
                cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular,
            );
        }
        println!("macOS Dock icon restored");
    }

    if let Err(e) = window.show() {
        println!("Error showing window: {:?}", e);
    }
    if let Err(e) = window.unminimize() {
        println!("Error unminimizing window: {:?}", e);
    }
    if let Err(e) = window.set_focus() {
        println!("Error setting focus: {:?}", e);
    }

    #[cfg(target_os = "macos")]
    #[allow(deprecated)]
    {
        use cocoa::appkit::NSApplication;
        use cocoa::base::nil;
        use objc::runtime::YES;

        unsafe {
            let ns_app = NSApplication::sharedApplication(nil);
            ns_app.activateIgnoringOtherApps_(YES);
        }
        println!("macOS app activated");
    }
}

fn hide_window_to_tray<R: Runtime>(window: &WebviewWindow<R>) {
    println!("Hiding window to system tray");
    if let Err(e) = window.hide() {
        println!("Failed to hide window: {:?}", e);
    }

    #[cfg(target_os = "macos")]
    #[allow(deprecated)]
    {
        use cocoa::appkit::NSApplication;
        use cocoa::base::nil;
        use cocoa::foundation::NSAutoreleasePool;

        unsafe {
            let _pool = NSAutoreleasePool::new(nil);
            let app_macos = NSApplication::sharedApplication(nil);
            app_macos.setActivationPolicy_(
                cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory,
            );
        }
        println!("macOS Dock icon hidden");
    }
}

// Helper: 统一使用库中的 HTTP 客户端构建逻辑（支持 SOCKS5 等代理）
fn build_reqwest_client() -> Result<reqwest::Client, String> {
    duckcoding::http_client::build_client()
}

// 透明代理全局状态
struct TransparentProxyState {
    service: Arc<TokioMutex<TransparentProxyService>>,
}

// 透明代理相关的 Tauri Commands
#[derive(serde::Serialize)]
struct TransparentProxyStatus {
    running: bool,
    port: u16,
}

#[tauri::command]
async fn start_transparent_proxy(
    state: tauri::State<'_, TransparentProxyState>,
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
async fn stop_transparent_proxy(
    state: tauri::State<'_, TransparentProxyState>,
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
async fn get_transparent_proxy_status(
    state: tauri::State<'_, TransparentProxyState>,
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
async fn update_transparent_proxy_config(
    state: tauri::State<'_, TransparentProxyState>,
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

fn main() {
    // 创建透明代理服务实例
    let transparent_proxy_port = 8787; // 默认端口，实际会从配置读取
    let transparent_proxy_service = TransparentProxyService::new(transparent_proxy_port);
    let transparent_proxy_state = TransparentProxyState {
        service: Arc::new(TokioMutex::new(transparent_proxy_service)),
    };

    let builder = tauri::Builder::default()
        .manage(transparent_proxy_state)
        .setup(|app| {
            // 尝试在应用启动时加载全局配置并应用代理设置，确保子进程继承代理 env
            if let Ok(config_path) = get_global_config_path() {
                if config_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&config_path) {
                        if let Ok(cfg) = serde_json::from_str::<GlobalConfig>(&content) {
                            // 应用代理到环境变量（进程级）
                            duckcoding::ProxyService::apply_proxy_from_config(&cfg);
                            println!("Applied proxy from config at startup");
                        }
                    }
                }
            }

            // 设置工作目录到项目根目录（跨平台支持）
            if let Ok(resource_dir) = app.path().resource_dir() {
                println!("Resource dir: {:?}", resource_dir);

                if cfg!(debug_assertions) {
                    // 开发模式：resource_dir 是 src-tauri/target/debug
                    // 需要回到项目根目录（上三级）
                    let project_root = resource_dir
                        .parent() // target
                        .and_then(|p| p.parent()) // src-tauri
                        .and_then(|p| p.parent()) // 项目根目录
                        .unwrap_or(&resource_dir);

                    println!("Development mode, setting dir to: {:?}", project_root);
                    let _ = env::set_current_dir(project_root);
                } else {
                    // 生产模式：跨平台支持
                    let parent_dir = if cfg!(target_os = "macos") {
                        // macOS: .app/Contents/Resources/
                        resource_dir
                            .parent()
                            .and_then(|p| p.parent())
                            .unwrap_or(&resource_dir)
                    } else if cfg!(target_os = "windows") {
                        // Windows: 通常在应用程序目录
                        resource_dir.parent().unwrap_or(&resource_dir)
                    } else {
                        // Linux: 通常在 /usr/share/appname 或类似位置
                        resource_dir.parent().unwrap_or(&resource_dir)
                    };
                    println!("Production mode, setting dir to: {:?}", parent_dir);
                    let _ = env::set_current_dir(parent_dir);
                }
            }

            println!("Working directory: {:?}", env::current_dir());

            // 创建系统托盘菜单
            let tray_menu = create_tray_menu(app.handle())?;
            let app_handle2 = app.handle().clone();

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| {
                    println!("Tray menu event: {:?}", event.id);
                    match event.id.as_ref() {
                        "show" => {
                            println!("Show window requested from tray menu");
                            focus_main_window(app);
                        }
                        "quit" => {
                            println!("Quit requested from tray menu");
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(move |_tray, event| {
                    println!("Tray icon event received: {:?}", event);
                    match event {
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } => {
                            println!("Tray icon LEFT click detected");
                            focus_main_window(&app_handle2);
                        }
                        _ => {
                            // 不打印太多日志
                        }
                    }
                })
                .build(app)?;

            // 处理窗口关闭事件 - 最小化到托盘而不是退出
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();

                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        println!("Window close requested - prompting for action");
                        // 阻止默认关闭行为
                        api.prevent_close();
                        if let Err(err) = window_clone.emit(CLOSE_CONFIRM_EVENT, ()) {
                            println!(
                                "Failed to emit close confirmation event, fallback to hiding: {:?}",
                                err
                            );
                            hide_window_to_tray(&window_clone);
                        }
                    }
                });
            }

            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            println!(
                "Secondary instance detected, args: {:?}, cwd: {}",
                argv, cwd
            );

            if let Err(err) = app.emit(
                SINGLE_INSTANCE_EVENT,
                SingleInstancePayload {
                    args: argv.clone(),
                    cwd: cwd.clone(),
                },
            ) {
                println!("Failed to emit single-instance event: {:?}", err);
            }

            focus_main_window(app);
        }))
        .invoke_handler(tauri::generate_handler![
            check_installations,
            check_node_environment,
            install_tool,
            check_update,
            check_all_updates,
            update_tool,
            configure_api,
            list_profiles,
            switch_profile,
            delete_profile,
            get_active_config,
            save_global_config,
            get_global_config,
            generate_api_key_for_tool,
            get_usage_stats,
            get_user_quota,
            handle_close_action,
            // expose current proxy for debugging/testing
            get_current_proxy,
            apply_proxy_now,
            test_proxy_request,
            get_claude_settings,
            save_claude_settings,
            get_claude_schema,
            get_codex_settings,
            save_codex_settings,
            get_codex_schema,
            get_gemini_settings,
            save_gemini_settings,
            get_gemini_schema,
            // 透明代理相关命令
            start_transparent_proxy,
            stop_transparent_proxy,
            get_transparent_proxy_status,
            update_transparent_proxy_config,
            // 更新管理相关命令
            check_for_app_updates,
            download_app_update,
            install_app_update,
            get_app_update_status,
            rollback_app_update,
            get_current_app_version,
            restart_app_for_update,
            get_platform_info,
            get_recommended_package_format,
        ]);

    // 使用自定义事件循环处理 macOS Reopen 事件
    builder
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            #[cfg(not(target_os = "macos"))]
            {
                let _ = app_handle;
                let _ = event;
            }
            #[cfg(target_os = "macos")]
            #[allow(deprecated)]
            {
                use cocoa::appkit::NSApplication;
                use cocoa::base::nil;
                use cocoa::foundation::NSAutoreleasePool;
                use objc::runtime::YES;

                if let tauri::RunEvent::Reopen { .. } = event {
                    println!("macOS Reopen event detected");

                    if let Some(window) = app_handle.get_webview_window("main") {
                        unsafe {
                            let _pool = NSAutoreleasePool::new(nil);
                            let app_macos = NSApplication::sharedApplication(nil);
                            app_macos.setActivationPolicy_(cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular);
                        }

                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();

                        unsafe {
                            let ns_app = NSApplication::sharedApplication(nil);
                            ns_app.activateIgnoringOtherApps_(YES);
                        }

                        println!("Window restored from Dock/Cmd+Tab");
                    }
                }
            }
        });
}

// Tauri command: 获取当前进程的代理设置（用于前端调试）
#[tauri::command]
fn get_current_proxy() -> Result<Option<String>, String> {
    Ok(duckcoding::ProxyService::get_current_proxy())
}

// Add runtime command to re-apply proxy from saved config without recompiling
#[tauri::command]
fn apply_proxy_now() -> Result<Option<String>, String> {
    let config_path = get_global_config_path()?;
    if !config_path.exists() {
        return Err("config not found".to_string());
    }
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    let cfg: GlobalConfig =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse config: {}", e))?;

    duckcoding::ProxyService::apply_proxy_from_config(&cfg);
    Ok(duckcoding::ProxyService::get_current_proxy())
}

#[derive(serde::Deserialize)]
struct ProxyTestConfig {
    enabled: bool,
    proxy_type: String,
    host: String,
    port: String,
    username: Option<String>,
    password: Option<String>,
}

#[derive(serde::Serialize)]
struct TestProxyResult {
    success: bool,
    status: u16,
    url: Option<String>,
    error: Option<String>,
}

#[tauri::command]
async fn test_proxy_request(
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

// ==================== 更新管理相关命令 ====================

// 全局更新服务实例
static UPDATE_SERVICE: std::sync::OnceLock<std::sync::Arc<UpdateService>> =
    std::sync::OnceLock::new();

fn get_update_service() -> std::sync::Arc<UpdateService> {
    UPDATE_SERVICE
        .get_or_init(|| {
            let service = UpdateService::new();
            // 初始化更新服务
            let service_clone = service.clone();
            tokio::spawn(async move {
                if let Err(e) = service_clone.initialize().await {
                    eprintln!("Failed to initialize update service: {}", e);
                }
            });
            std::sync::Arc::new(service)
        })
        .clone()
}

#[tauri::command]
async fn check_for_app_updates() -> Result<UpdateInfo, String> {
    let service = get_update_service();
    service
        .check_for_updates()
        .await
        .map_err(|e| format!("Failed to check for updates: {}", e))
}

#[tauri::command]
async fn download_app_update(url: String, app: AppHandle) -> Result<String, String> {
    let service = get_update_service();
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    let _service_clone = service.clone();
    let window_clone = window.clone();

    service
        .download_update(&url, move |progress| {
            let _ = window_clone.emit("update-download-progress", &progress);
        })
        .await
        .map_err(|e| format!("Failed to download update: {}", e))
}

#[tauri::command]
async fn install_app_update(update_path: String) -> Result<(), String> {
    let service = get_update_service();
    service
        .install_update(&update_path)
        .await
        .map_err(|e| format!("Failed to install update: {}", e))
}

#[tauri::command]
async fn get_app_update_status() -> Result<UpdateStatus, String> {
    let service = get_update_service();
    Ok(service.get_status().await)
}

#[tauri::command]
async fn rollback_app_update() -> Result<(), String> {
    let service = get_update_service();
    service
        .rollback_update()
        .await
        .map_err(|e| format!("Failed to rollback update: {}", e))
}

#[tauri::command]
async fn get_current_app_version() -> Result<String, String> {
    let service = get_update_service();
    Ok(service.get_current_version().to_string())
}

#[tauri::command]
async fn restart_app_for_update(app: AppHandle) -> Result<(), String> {
    // 立即重启应用
    app.restart();
}

#[tauri::command]
async fn get_platform_info() -> Result<PlatformInfo, String> {
    let service = get_update_service();
    Ok(service.get_platform_info())
}

#[tauri::command]
async fn get_recommended_package_format() -> Result<PackageFormatInfo, String> {
    let service = get_update_service();
    Ok(service.get_recommended_package_format())
}
