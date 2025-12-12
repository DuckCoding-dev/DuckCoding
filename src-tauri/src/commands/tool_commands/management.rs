use crate::commands::tool_management::ToolRegistryState;
use crate::commands::types::ToolStatus;
use ::duckcoding::models::InstallMethod;

/// 手动添加工具实例（保存用户指定的路径）
///
/// 工作流程：
/// 1. 委托给 ToolRegistry.add_tool_instance
/// 2. Registry 负责路径验证、冲突检查、数据库保存
///
/// 返回：工具状态信息
#[tauri::command]
pub async fn add_manual_tool_instance(
    tool_id: String,
    path: String,
    install_method: String, // "npm" | "brew" | "official" | "other"
    installer_path: Option<String>,
    registry_state: tauri::State<'_, ToolRegistryState>,
) -> Result<ToolStatus, String> {
    // 解析安装方法
    let parsed_method = match install_method.as_str() {
        "npm" => InstallMethod::Npm,
        "brew" => InstallMethod::Brew,
        "official" => InstallMethod::Official,
        "other" => InstallMethod::Other,
        _ => return Err(format!("未知的安装方法: {}", install_method)),
    };

    // 委托给 ToolRegistry
    let registry = registry_state.registry.lock().await;
    registry
        .add_tool_instance(&tool_id, &path, parsed_method, installer_path)
        .await
        .map_err(|e| e.to_string())
}
