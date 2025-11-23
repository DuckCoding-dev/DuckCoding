use crate::commands::types::{InstallResult, NodeEnvironment, ToolStatus, UpdateResult};
use ::duckcoding::models::{InstallMethod, Tool};
use ::duckcoding::services::{InstallerService, ToolStatusCache, VersionService};
use ::duckcoding::utils::config::apply_proxy_if_configured;
use ::duckcoding::utils::platform::PlatformInfo;
use std::process::Command;
use std::sync::Arc;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// 工具状态缓存 State
pub struct ToolStatusCacheState {
    pub cache: Arc<ToolStatusCache>,
}

/// 检查所有工具的安装状态（使用缓存 + 并行检测）
#[tauri::command]
pub async fn check_installations(
    state: tauri::State<'_, ToolStatusCacheState>,
) -> Result<Vec<ToolStatus>, String> {
    Ok(state.cache.get_all_status().await)
}

/// 刷新工具状态（清除缓存并重新检测）
#[tauri::command]
pub async fn refresh_tool_status(
    state: tauri::State<'_, ToolStatusCacheState>,
) -> Result<Vec<ToolStatus>, String> {
    // 清除缓存
    state.cache.clear().await;
    // 重新检测
    Ok(state.cache.get_all_status().await)
}

/// 检测 Node.js 和 npm 环境
#[tauri::command]
pub async fn check_node_environment() -> Result<NodeEnvironment, String> {
    let enhanced_path = PlatformInfo::current().build_enhanced_path();
    let run_command = |cmd: &str| -> Result<std::process::Output, std::io::Error> {
        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .env("PATH", &enhanced_path)
                .arg("/C")
                .arg(cmd)
                .creation_flags(0x08000000) // CREATE_NO_WINDOW - 隐藏终端窗口
                .output()
        }
        #[cfg(not(target_os = "windows"))]
        {
            Command::new("sh")
                .env("PATH", &enhanced_path)
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

/// 安装指定工具
#[tauri::command]
pub async fn install_tool(
    state: tauri::State<'_, ToolStatusCacheState>,
    tool: String,
    method: String,
    force: Option<bool>,
) -> Result<InstallResult, String> {
    // 应用代理配置（如果已配置）
    apply_proxy_if_configured();

    let force = force.unwrap_or(false);
    #[cfg(debug_assertions)]
    println!("Installing {tool} via {method} (using InstallerService, force={force})");

    // 获取工具定义
    let tool_obj =
        Tool::by_id(&tool).ok_or_else(|| "❌ 未知的工具\n\n请联系开发者报告此问题".to_string())?;

    // 转换安装方法
    let install_method = match method.as_str() {
        "npm" => InstallMethod::Npm,
        "brew" => InstallMethod::Brew,
        "official" => InstallMethod::Official,
        _ => return Err(format!("❌ 未知的安装方法: {method}")),
    };

    // 使用 InstallerService 安装
    let installer = InstallerService::new();

    match installer.install(&tool_obj, &install_method, force).await {
        Ok(_) => {
            // 安装成功，清除该工具的缓存
            state.cache.clear_tool(&tool).await;

            // 构造成功消息
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

/// 检查工具更新（不执行更新）
#[tauri::command]
pub async fn check_update(tool: String) -> Result<UpdateResult, String> {
    // 应用代理配置（如果已配置）
    apply_proxy_if_configured();

    #[cfg(debug_assertions)]
    println!("Checking updates for {tool} (using VersionService)");

    let tool_obj = Tool::by_id(&tool).ok_or_else(|| format!("未知工具: {tool}"))?;

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
                message: format!("无法检查更新: {e}"),
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

/// 批量检查所有工具更新
#[tauri::command]
pub async fn check_all_updates() -> Result<Vec<UpdateResult>, String> {
    // 应用代理配置（如果已配置）
    apply_proxy_if_configured();

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

/// 更新指定工具
#[tauri::command]
pub async fn update_tool(
    state: tauri::State<'_, ToolStatusCacheState>,
    tool: String,
    force: Option<bool>,
) -> Result<UpdateResult, String> {
    // 应用代理配置（如果已配置）
    apply_proxy_if_configured();

    let force = force.unwrap_or(false);
    #[cfg(debug_assertions)]
    println!("Updating {tool} (using InstallerService, force={force})");

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
            // 更新成功，清除该工具的缓存
            state.cache.clear_tool(&tool).await;

            // 获取新版本
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
