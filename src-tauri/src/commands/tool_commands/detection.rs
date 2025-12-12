use crate::commands::tool_management::ToolRegistryState;
use crate::commands::types::ToolStatus;
use ::duckcoding::utils::{parse_version_string, CommandExecutor, ToolCandidate};

/// 扫描所有工具候选（用于自动扫描）
///
/// 工作流程：
/// 1. 委托给 ToolRegistry.scan_tool_candidates
/// 2. Registry 负责扫描路径、获取版本、检测安装器
///
/// 返回：工具候选列表
#[tauri::command]
pub async fn scan_all_tool_candidates(
    tool_id: String,
    registry_state: tauri::State<'_, ToolRegistryState>,
) -> Result<Vec<ToolCandidate>, String> {
    let registry = registry_state.registry.lock().await;
    registry
        .scan_tool_candidates(&tool_id)
        .await
        .map_err(|e| e.to_string())
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
/// 1. 委托给 ToolRegistry.detect_single_tool_with_cache
/// 2. Registry 负责检查数据库缓存、执行检测、保存结果
///
/// 返回：工具实例信息
#[tauri::command]
pub async fn detect_single_tool(
    tool_id: String,
    force_redetect: Option<bool>,
    registry_state: tauri::State<'_, ToolRegistryState>,
) -> Result<ToolStatus, String> {
    let registry = registry_state.registry.lock().await;
    registry
        .detect_single_tool_with_cache(&tool_id, force_redetect.unwrap_or(false))
        .await
        .map_err(|e| e.to_string())
}
