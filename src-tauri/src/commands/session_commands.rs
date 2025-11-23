// 会话管理 Tauri 命令

use duckcoding::services::session::{SessionListResponse, SESSION_MANAGER};

/// 获取会话列表
#[tauri::command]
pub async fn get_session_list(
    tool_id: String,
    page: usize,
    page_size: usize,
) -> Result<SessionListResponse, String> {
    SESSION_MANAGER
        .get_session_list(&tool_id, page, page_size)
        .map_err(|e| format!("Failed to get session list: {e}"))
}

/// 删除单个会话
#[tauri::command]
pub async fn delete_session(session_id: String) -> Result<(), String> {
    SESSION_MANAGER
        .delete_session(&session_id)
        .map_err(|e| format!("Failed to delete session: {e}"))
}

/// 清空指定工具的所有会话
#[tauri::command]
pub async fn clear_all_sessions(tool_id: String) -> Result<(), String> {
    SESSION_MANAGER
        .clear_sessions(&tool_id)
        .map_err(|e| format!("Failed to clear sessions: {e}"))
}

/// 更新会话配置
#[tauri::command]
pub async fn update_session_config(
    session_id: String,
    config_name: String,
    custom_profile_name: Option<String>,
    url: String,
    api_key: String,
) -> Result<(), String> {
    SESSION_MANAGER
        .update_session_config(
            &session_id,
            &config_name,
            custom_profile_name.as_deref(),
            &url,
            &api_key,
        )
        .map_err(|e| format!("Failed to update session config: {e}"))
}

/// 更新会话备注
#[tauri::command]
pub async fn update_session_note(session_id: String, note: Option<String>) -> Result<(), String> {
    SESSION_MANAGER
        .update_session_note(&session_id, note.as_deref())
        .map_err(|e| format!("Failed to update session note: {e}"))
}
