use ::duckcoding::utils::{scan_installer_paths, InstallerCandidate};

/// 扫描工具路径的安装器
///
/// 工作流程：
/// 1. 从工具路径提取目录
/// 2. 在同级目录扫描安装器（npm、brew 等）
/// 3. 在上级目录扫描安装器
/// 4. 返回候选列表（按优先级排序）
///
/// 返回：安装器候选列表
#[tauri::command]
pub async fn scan_installer_for_tool_path(
    tool_path: String,
) -> Result<Vec<InstallerCandidate>, String> {
    Ok(scan_installer_paths(&tool_path))
}
