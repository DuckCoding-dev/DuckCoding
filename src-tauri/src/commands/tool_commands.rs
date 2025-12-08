use crate::commands::tool_management::ToolRegistryState;
use crate::commands::types::{InstallResult, NodeEnvironment, ToolStatus, UpdateResult};
use ::duckcoding::models::{InstallMethod, Tool};
use ::duckcoding::services::{InstallerService, VersionService};
use ::duckcoding::utils::config::apply_proxy_if_configured;
use ::duckcoding::utils::platform::PlatformInfo;
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// 检查所有工具的安装状态（新架构：优先从数据库读取）
///
/// 工作流程：
/// 1. 检查数据库是否有数据
/// 2. 如果没有 → 执行首次检测并保存到数据库
/// 3. 从数据库读取并返回轻量级 ToolStatus
///
/// 性能：数据库读取 < 10ms，首次检测约 1.3s
#[tauri::command]
pub async fn check_installations(
    registry_state: tauri::State<'_, ToolRegistryState>,
) -> Result<Vec<ToolStatus>, String> {
    let registry = registry_state.registry.lock().await;
    registry
        .get_local_tool_status()
        .await
        .map_err(|e| format!("检查工具状态失败: {}", e))
}

/// 刷新工具状态（仅从数据库读取，不重新检测）
///
/// 修改说明：不再自动检测所有工具，仅返回数据库中已有的工具状态
/// 如需添加新工具或验证已有工具，请使用：
/// - 添加新工具：工具管理页面 → 添加实例
/// - 验证单个工具：使用 detect_single_tool 命令
#[tauri::command]
pub async fn refresh_tool_status(
    registry_state: tauri::State<'_, ToolRegistryState>,
) -> Result<Vec<ToolStatus>, String> {
    let registry = registry_state.registry.lock().await;
    registry
        .get_local_tool_status()
        .await
        .map_err(|e| format!("获取工具状态失败: {}", e))
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
    tool: String,
    method: String,
    force: Option<bool>,
) -> Result<InstallResult, String> {
    // 应用代理配置（如果已配置）
    apply_proxy_if_configured();

    let force = force.unwrap_or(false);
    #[cfg(debug_assertions)]
    tracing::debug!(tool = %tool, method = %method, force = force, "安装工具（使用InstallerService）");

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
            // 安装成功（前端会调用 refresh_tool_status 更新数据库）

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
    tracing::debug!(tool = %tool, "检查更新（使用VersionService）");

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
    tracing::debug!("批量检查所有工具更新");

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
pub async fn update_tool(tool: String, force: Option<bool>) -> Result<UpdateResult, String> {
    // 应用代理配置（如果已配置）
    apply_proxy_if_configured();

    let force = force.unwrap_or(false);
    #[cfg(debug_assertions)]
    tracing::debug!(tool = %tool, force = force, "更新工具（使用InstallerService）");

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
            // 更新成功（前端会调用 refresh_tool_status 更新数据库）

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

/// 验证用户指定的工具路径是否有效
///
/// 工作流程：
/// 1. 检查文件是否存在
/// 2. 执行 --version 命令
/// 3. 解析版本号
///
/// 返回：版本号字符串
#[tauri::command]
pub async fn validate_tool_path(_tool_id: String, path: String) -> Result<String, String> {
    use std::path::PathBuf;
    use std::process::Command;

    let path_buf = PathBuf::from(&path);

    // 检查文件是否存在
    if !path_buf.exists() {
        return Err(format!("路径不存在: {}", path));
    }

    // 检查是否是文件
    if !path_buf.is_file() {
        return Err(format!("路径不是文件: {}", path));
    }

    // 执行 --version 命令
    let version_cmd = format!("{} --version", path);

    #[cfg(target_os = "windows")]
    let output = Command::new("cmd")
        .arg("/C")
        .arg(&version_cmd)
        .output()
        .map_err(|e| format!("执行命令失败: {}", e))?;

    #[cfg(not(target_os = "windows"))]
    let output = Command::new("sh")
        .arg("-c")
        .arg(&version_cmd)
        .output()
        .map_err(|e| format!("执行命令失败: {}", e))?;

    if !output.status.success() {
        return Err(format!("命令执行失败，退出码: {}", output.status));
    }

    // 解析版本号
    let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if version_str.is_empty() {
        return Err("无法获取版本信息".to_string());
    }

    // 简单验证：版本号应该包含数字
    if !version_str.chars().any(|c| c.is_numeric()) {
        return Err(format!("无效的版本信息: {}", version_str));
    }

    Ok(version_str)
}

/// 手动添加工具实例（保存用户指定的路径）
///
/// 工作流程：
/// 1. 验证路径有效性
/// 2. 创建 ToolInstance
/// 3. 保存到数据库
///
/// 返回：工具状态信息
#[tauri::command]
pub async fn add_manual_tool_instance(
    tool_id: String,
    path: String,
    _registry_state: tauri::State<'_, ToolRegistryState>,
) -> Result<crate::commands::types::ToolStatus, String> {
    use ::duckcoding::models::{InstallMethod, ToolInstance, ToolType};
    use ::duckcoding::services::tool::ToolInstanceDB;

    // 1. 验证路径
    let version = validate_tool_path(tool_id.clone(), path.clone()).await?;

    // 2. 创建工具显示名称
    let tool_name = match tool_id.as_str() {
        "claude-code" => "Claude Code",
        "codex" => "CodeX",
        "gemini-cli" => "Gemini CLI",
        _ => &tool_id,
    };

    // 3. 创建 ToolInstance
    let instance_id = format!("{}-local-manual", tool_id);
    let now = chrono::Utc::now().timestamp();
    let instance = ToolInstance {
        instance_id: instance_id.clone(),
        base_id: tool_id.clone(),
        tool_name: tool_name.to_string(),
        tool_type: ToolType::Local,
        install_method: Some(InstallMethod::Npm), // 手动添加默认为 Npm，可后续优化
        installed: true,
        version: Some(version.clone()),
        install_path: Some(path.clone()),
        wsl_distro: None,
        ssh_config: None,
        is_builtin: false, // 手动添加的工具不是内置的
        created_at: now,
        updated_at: now,
    };

    // 4. 保存到数据库
    let db = ToolInstanceDB::new().map_err(|e| format!("初始化数据库失败: {}", e))?;

    db.add_instance(&instance)
        .map_err(|e| format!("保存到数据库失败: {}", e))?;

    // 5. 返回 ToolStatus 格式
    Ok(crate::commands::types::ToolStatus {
        id: tool_id.clone(),
        name: tool_name.to_string(),
        installed: true,
        version: Some(version),
    })
}

/// 检测单个工具但不保存（仅用于预览）
///
/// 工作流程：
/// 1. 简化版检测：直接调用命令检查工具是否存在
/// 2. 返回检测结果（不保存到数据库）
///
/// 返回：工具状态信息
#[tauri::command]
pub async fn detect_tool_without_save(
    tool_id: String,
    _registry_state: tauri::State<'_, ToolRegistryState>,
) -> Result<crate::commands::types::ToolStatus, String> {
    use ::duckcoding::utils::CommandExecutor;

    let command_executor = CommandExecutor::new();

    // 根据工具ID确定检测命令和名称
    let (check_cmd, tool_name) = match tool_id.as_str() {
        "claude-code" => ("claude", "Claude Code"),
        "codex" => ("codex", "CodeX"),
        "gemini-cli" => ("gemini", "Gemini CLI"),
        _ => return Err(format!("未知工具ID: {}", tool_id)),
    };

    // 检测工具是否存在
    let installed = command_executor.command_exists_async(check_cmd).await;

    let version = if installed {
        // 获取版本
        let version_cmd = format!("{} --version", check_cmd);
        let result = command_executor.execute_async(&version_cmd).await;
        if result.success {
            let version_str = result.stdout.trim().to_string();
            if !version_str.is_empty() {
                Some(version_str)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    Ok(crate::commands::types::ToolStatus {
        id: tool_id.clone(),
        name: tool_name.to_string(),
        installed,
        version,
    })
}

/// 检测单个工具并保存到数据库
///
/// 工作流程：
/// 1. 先查询数据库中是否已有该工具的实例
/// 2. 如果已有且已安装，直接返回（除非 force_redetect = true）
/// 3. 如果没有或需要重新检测，执行单工具检测（会先删除旧实例）
///
/// 返回：工具实例信息
#[tauri::command]
pub async fn detect_single_tool(
    tool_id: String,
    force_redetect: Option<bool>,
    registry_state: tauri::State<'_, ToolRegistryState>,
) -> Result<crate::commands::types::ToolStatus, String> {
    use ::duckcoding::models::ToolType;
    use ::duckcoding::services::tool::ToolInstanceDB;

    let force = force_redetect.unwrap_or(false);

    if !force {
        // 1. 先查询数据库中是否已有该工具的本地实例
        let db = ToolInstanceDB::new().map_err(|e| format!("初始化数据库失败: {}", e))?;
        let all_instances = db
            .get_all_instances()
            .map_err(|e| format!("读取数据库失败: {}", e))?;

        // 查找该工具的本地实例
        if let Some(existing) = all_instances.iter().find(|inst| {
            inst.base_id == tool_id && inst.tool_type == ToolType::Local && inst.installed
        }) {
            // 如果已有实例且已安装，直接返回
            tracing::info!("工具 {} 已在数据库中，直接返回", existing.tool_name);
            return Ok(crate::commands::types::ToolStatus {
                id: tool_id.clone(),
                name: existing.tool_name.clone(),
                installed: true,
                version: existing.version.clone(),
            });
        }
    }

    // 2. 执行单工具检测（会删除旧实例避免重复）
    let registry = registry_state.registry.lock().await;
    let instance = registry
        .detect_and_persist_single_tool(&tool_id)
        .await
        .map_err(|e| format!("检测失败: {}", e))?;

    // 3. 返回 ToolStatus 格式
    Ok(crate::commands::types::ToolStatus {
        id: tool_id.clone(),
        name: instance.tool_name.clone(),
        installed: instance.installed,
        version: instance.version.clone(),
    })
}
