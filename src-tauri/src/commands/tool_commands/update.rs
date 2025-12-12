use crate::commands::tool_management::ToolRegistryState;
use crate::commands::types::{ToolStatus, UpdateResult};
use ::duckcoding::models::Tool;
use ::duckcoding::services::VersionService;
use ::duckcoding::utils::config::apply_proxy_if_configured;

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

/// 检查工具更新（基于实例ID，使用配置的路径）
///
/// 工作流程：
/// 1. 委托给 ToolRegistry.check_update_for_instance
/// 2. Registry 负责获取实例信息、检测版本、更新数据库
///
/// 返回：更新信息
#[tauri::command]
pub async fn check_update_for_instance(
    instance_id: String,
    registry_state: tauri::State<'_, ToolRegistryState>,
) -> Result<UpdateResult, String> {
    let registry = registry_state.registry.lock().await;
    registry
        .check_update_for_instance(&instance_id)
        .await
        .map_err(|e| e.to_string())
}

/// 刷新数据库中所有工具的版本号（使用配置的路径检测）
///
/// 工作流程：
/// 1. 委托给 ToolRegistry.refresh_all_tool_versions
/// 2. Registry 负责检测所有本地工具版本并更新数据库
///
/// 返回：更新后的工具状态列表
#[tauri::command]
pub async fn refresh_all_tool_versions(
    registry_state: tauri::State<'_, ToolRegistryState>,
) -> Result<Vec<ToolStatus>, String> {
    let registry = registry_state.registry.lock().await;
    registry
        .refresh_all_tool_versions()
        .await
        .map_err(|e| e.to_string())
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

/// 更新工具实例（使用配置的安装器路径）
///
/// 工作流程：
/// 1. 委托给 ToolRegistry.update_instance
/// 2. Registry 负责从数据库获取实例信息
/// 3. 使用 InstallerService 执行更新
/// 4. 更新数据库中的版本号
///
/// 返回：更新结果
#[tauri::command]
pub async fn update_tool_instance(
    instance_id: String,
    force: Option<bool>,
    registry_state: tauri::State<'_, ToolRegistryState>,
) -> Result<UpdateResult, String> {
    let registry = registry_state.registry.lock().await;
    registry
        .update_instance(&instance_id, force.unwrap_or(false))
        .await
        .map_err(|e| e.to_string())
}
