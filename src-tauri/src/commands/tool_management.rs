use duckcoding::models::{SSHConfig, ToolInstance};
use duckcoding::services::tool::ToolRegistry;
use duckcoding::utils::WSLExecutor;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 工具注册表 State
pub struct ToolRegistryState {
    pub registry: Arc<Mutex<ToolRegistry>>,
}

/// 检查数据库中是否已有本地工具数据（用于判断是否需要执行首次检测）
#[tauri::command]
pub async fn has_tools_in_database(
    state: tauri::State<'_, ToolRegistryState>,
) -> Result<bool, String> {
    let registry = state.registry.lock().await;
    registry
        .has_local_tools_in_db()
        .await
        .map_err(|e| format!("检查数据库失败: {}", e))
}

/// 检测本地工具并保存到数据库（用于新手引导）
#[tauri::command]
pub async fn detect_and_save_tools(
    state: tauri::State<'_, ToolRegistryState>,
) -> Result<Vec<ToolInstance>, String> {
    let registry = state.registry.lock().await;
    registry
        .detect_and_persist_local_tools()
        .await
        .map_err(|e| format!("检测工具失败: {}", e))
}

/// 获取所有工具实例（按工具ID分组）- 只从数据库读取
#[tauri::command]
pub async fn get_tool_instances(
    state: tauri::State<'_, ToolRegistryState>,
) -> Result<HashMap<String, Vec<ToolInstance>>, String> {
    let registry = state.registry.lock().await;
    registry
        .get_all_grouped()
        .await
        .map_err(|e| format!("获取工具实例失败: {}", e))
}

/// 刷新工具实例状态（重新检测并更新数据库）
#[tauri::command]
pub async fn refresh_tool_instances(
    state: tauri::State<'_, ToolRegistryState>,
) -> Result<HashMap<String, Vec<ToolInstance>>, String> {
    let registry = state.registry.lock().await;
    registry
        .refresh_all()
        .await
        .map_err(|e| format!("刷新工具实例失败: {}", e))
}

/// 列出所有可用的WSL发行版
#[tauri::command]
pub async fn list_wsl_distributions() -> Result<Vec<String>, String> {
    WSLExecutor::list_distributions().map_err(|e| format!("列出WSL发行版失败: {}", e))
}

/// 添加WSL工具实例
#[tauri::command]
pub async fn add_wsl_tool_instance(
    state: tauri::State<'_, ToolRegistryState>,
    base_id: String,
    distro_name: String,
) -> Result<ToolInstance, String> {
    let registry = state.registry.lock().await;
    registry
        .add_wsl_instance(&base_id, &distro_name)
        .await
        .map_err(|e| format!("添加WSL实例失败: {}", e))
}

/// 添加SSH工具实例（本期仅存储配置）
#[tauri::command]
pub async fn add_ssh_tool_instance(
    state: tauri::State<'_, ToolRegistryState>,
    base_id: String,
    ssh_config: SSHConfig,
) -> Result<ToolInstance, String> {
    let registry = state.registry.lock().await;
    registry
        .add_ssh_instance(&base_id, ssh_config)
        .await
        .map_err(|e| format!("添加SSH实例失败: {}", e))
}

/// 删除工具实例（仅SSH类型）
#[tauri::command]
pub async fn delete_tool_instance(
    state: tauri::State<'_, ToolRegistryState>,
    instance_id: String,
) -> Result<(), String> {
    let registry = state.registry.lock().await;
    registry
        .delete_instance(&instance_id)
        .await
        .map_err(|e| format!("删除工具实例失败: {}", e))
}
