// Dashboard Commands
//
// 仪表板状态管理 Tauri 命令

use ::duckcoding::services::DashboardManager;
use anyhow::Result;
use tauri::State;

/// Dashboard 管理器 State
pub struct DashboardManagerState {
    pub manager: DashboardManager,
}

impl DashboardManagerState {
    pub fn new() -> Self {
        Self {
            manager: DashboardManager::new().expect("Failed to create DashboardManager"),
        }
    }
}

impl Default for DashboardManagerState {
    fn default() -> Self {
        Self::new()
    }
}

/// 获取工具实例选择
#[tauri::command]
pub async fn get_tool_instance_selection(
    tool_id: String,
    state: State<'_, DashboardManagerState>,
) -> Result<Option<String>, String> {
    if tool_id.is_empty() {
        return Err("工具 ID 不能为空".to_string());
    }

    state
        .manager
        .get_tool_instance_selection(&tool_id)
        .map_err(|e| format!("获取工具实例选择失败: {}", e))
}

/// 设置工具实例选择
#[tauri::command]
pub async fn set_tool_instance_selection(
    tool_id: String,
    instance_id: String,
    state: State<'_, DashboardManagerState>,
) -> Result<(), String> {
    // 验证参数
    if tool_id.is_empty() {
        return Err("工具 ID 不能为空".to_string());
    }
    if instance_id.is_empty() {
        return Err("实例 ID 不能为空".to_string());
    }

    state
        .manager
        .set_tool_instance_selection(tool_id, instance_id)
        .map_err(|e| format!("设置工具实例选择失败: {}", e))
}

/// 获取最后选中的供应商 ID
#[tauri::command]
pub async fn get_selected_provider_id(
    state: State<'_, DashboardManagerState>,
) -> Result<Option<String>, String> {
    state
        .manager
        .get_selected_provider_id()
        .map_err(|e| format!("获取选中供应商失败: {}", e))
}

/// 设置最后选中的供应商 ID
#[tauri::command]
pub async fn set_selected_provider_id(
    provider_id: Option<String>,
    state: State<'_, DashboardManagerState>,
) -> Result<(), String> {
    state
        .manager
        .set_selected_provider_id(provider_id)
        .map_err(|e| format!("设置选中供应商失败: {}", e))
}
