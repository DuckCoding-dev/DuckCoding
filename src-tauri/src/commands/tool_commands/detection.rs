use super::utils::parse_version_string;
use crate::commands::tool_management::ToolRegistryState;
use crate::commands::types::ToolStatus;
use ::duckcoding::models::{InstallMethod, ToolType};
use ::duckcoding::services::tool::ToolInstanceDB;
use ::duckcoding::utils::{
    scan_installer_paths, scan_tool_executables, CommandExecutor, ToolCandidate,
};
use std::process::Command;

/// 扫描所有工具候选（用于自动扫描）
///
/// 工作流程：
/// 1. 使用硬编码路径列表查找所有工具实例
/// 2. 对每个找到的工具：获取版本、检测安装方法、扫描安装器
/// 3. 返回候选列表供用户选择
///
/// 返回：工具候选列表
#[tauri::command]
pub async fn scan_all_tool_candidates(tool_id: String) -> Result<Vec<ToolCandidate>, String> {
    // 1. 扫描所有工具路径
    let tool_paths = scan_tool_executables(&tool_id);
    let mut candidates = Vec::new();

    // 2. 对每个工具路径：获取版本和安装器
    for tool_path in tool_paths {
        // 获取版本
        let version_cmd = format!("{} --version", tool_path);

        #[cfg(target_os = "windows")]
        let output = Command::new("cmd").arg("/C").arg(&version_cmd).output();

        #[cfg(not(target_os = "windows"))]
        let output = Command::new("sh").arg("-c").arg(&version_cmd).output();

        let version = match output {
            Ok(out) if out.status.success() => {
                let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
                parse_version_string(&raw)
            }
            _ => continue, // 版本获取失败，跳过此候选
        };

        // 扫描安装器
        let installer_candidates = scan_installer_paths(&tool_path);
        let installer_path = installer_candidates.first().map(|c| c.path.clone());
        let install_method = installer_candidates
            .first()
            .map(|c| c.installer_type.clone())
            .unwrap_or(InstallMethod::Official);

        candidates.push(ToolCandidate {
            tool_path: tool_path.clone(),
            installer_path,
            install_method,
            version,
        });
    }

    Ok(candidates)
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
) -> Result<ToolStatus, String> {
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
                Some(parse_version_string(&version_str))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    Ok(ToolStatus {
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
) -> Result<ToolStatus, String> {
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
            return Ok(ToolStatus {
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
    Ok(ToolStatus {
        id: tool_id.clone(),
        name: instance.tool_name.clone(),
        installed: instance.installed,
        version: instance.version.clone(),
    })
}
