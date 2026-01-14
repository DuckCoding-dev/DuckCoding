//! Profile 管理 Tauri 命令（v2.1 - 简化版）

use super::error::AppResult;
use ::duckcoding::services::profile_manager::{AmpProfileSelection, ProfileDescriptor, ProfileRef};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Profile 管理器 State
pub struct ProfileManagerState {
    pub manager: Arc<RwLock<::duckcoding::services::profile_manager::ProfileManager>>,
}

/// Profile 输入数据（前端传递）
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ProfileInput {
    #[serde(rename = "claude-code")]
    Claude {
        api_key: String,
        base_url: String,
        #[serde(default)]
        pricing_template_id: Option<String>, // 🆕 Phase 6: 价格模板 ID
    },
    #[serde(rename = "codex")]
    Codex {
        api_key: String,
        base_url: String,
        wire_api: String,
        #[serde(default)]
        pricing_template_id: Option<String>, // 🆕 Phase 6: 价格模板 ID
    },
    #[serde(rename = "gemini-cli")]
    Gemini {
        api_key: String,
        base_url: String,
        #[serde(default)]
        model: Option<String>,
        #[serde(default)]
        pricing_template_id: Option<String>, // 🆕 Phase 6: 价格模板 ID
    },
}

/// 列出所有 Profile 描述符
#[tauri::command]
pub async fn pm_list_all_profiles(
    state: tauri::State<'_, ProfileManagerState>,
) -> AppResult<Vec<ProfileDescriptor>> {
    let manager = state.manager.read().await;
    Ok(manager.list_all_descriptors()?)
}

/// 列出指定工具的 Profile 名称
#[tauri::command]
pub async fn pm_list_tool_profiles(
    state: tauri::State<'_, ProfileManagerState>,
    tool_id: String,
) -> AppResult<Vec<String>> {
    let manager = state.manager.read().await;
    Ok(manager.list_profiles(&tool_id)?)
}

/// 获取指定 Profile（返回 JSON 供前端使用）
#[tauri::command]
pub async fn pm_get_profile(
    state: tauri::State<'_, ProfileManagerState>,
    tool_id: String,
    name: String,
) -> AppResult<serde_json::Value> {
    let manager = state.manager.read().await;

    let value = match tool_id.as_str() {
        "claude-code" => {
            let profile = manager.get_claude_profile(&name)?;
            serde_json::to_value(&profile)?
        }
        "codex" => {
            let profile = manager.get_codex_profile(&name)?;
            serde_json::to_value(&profile)?
        }
        "gemini-cli" => {
            let profile = manager.get_gemini_profile(&name)?;
            serde_json::to_value(&profile)?
        }
        _ => return Err(super::error::AppError::ToolNotFound { tool: tool_id }),
    };

    Ok(value)
}

/// 获取当前激活的 Profile（返回 JSON 供前端使用）
#[tauri::command]
pub async fn pm_get_active_profile(
    state: tauri::State<'_, ProfileManagerState>,
    tool_id: String,
) -> AppResult<Option<serde_json::Value>> {
    let manager = state.manager.read().await;
    let name = manager.get_active_profile_name(&tool_id)?;

    if let Some(profile_name) = name {
        drop(manager); // 释放读锁
        pm_get_profile(state, tool_id, profile_name).await.map(Some)
    } else {
        Ok(None)
    }
}

/// 保存 Profile（创建或更新）
#[tauri::command]
pub async fn pm_save_profile(
    state: tauri::State<'_, ProfileManagerState>,
    tool_id: String,
    name: String,
    input: ProfileInput,
) -> AppResult<()> {
    let manager = state.manager.write().await; // 写锁

    match tool_id.as_str() {
        "claude-code" => {
            if let ProfileInput::Claude {
                api_key,
                base_url,
                pricing_template_id,
            } = input
            {
                Ok(manager.save_claude_profile_with_template(
                    &name,
                    api_key,
                    base_url,
                    pricing_template_id,
                )?)
            } else {
                Err(super::error::AppError::ValidationError {
                    field: "input".to_string(),
                    reason: "Claude Code 需要 Claude Profile 数据".to_string(),
                })
            }
        }
        "codex" => {
            if let ProfileInput::Codex {
                api_key,
                base_url,
                wire_api,
                pricing_template_id,
            } = input
            {
                Ok(manager.save_codex_profile_with_template(
                    &name,
                    api_key,
                    base_url,
                    Some(wire_api),
                    pricing_template_id,
                )?)
            } else {
                Err(super::error::AppError::ValidationError {
                    field: "input".to_string(),
                    reason: "Codex 需要 Codex Profile 数据".to_string(),
                })
            }
        }
        "gemini-cli" => {
            if let ProfileInput::Gemini {
                api_key,
                base_url,
                model,
                pricing_template_id,
            } = input
            {
                Ok(manager.save_gemini_profile_with_template(
                    &name,
                    api_key,
                    base_url,
                    model,
                    pricing_template_id,
                )?)
            } else {
                Err(super::error::AppError::ValidationError {
                    field: "input".to_string(),
                    reason: "Gemini CLI 需要 Gemini Profile 数据".to_string(),
                })
            }
        }
        _ => Err(super::error::AppError::ToolNotFound { tool: tool_id }),
    }
}

/// 删除 Profile
#[tauri::command]
pub async fn pm_delete_profile(
    state: tauri::State<'_, ProfileManagerState>,
    tool_id: String,
    name: String,
) -> AppResult<()> {
    let manager = state.manager.write().await;
    Ok(manager.delete_profile(&tool_id, &name)?)
}

/// 激活 Profile
#[tauri::command]
pub async fn pm_activate_profile(
    state: tauri::State<'_, ProfileManagerState>,
    tool_id: String,
    name: String,
) -> AppResult<()> {
    let manager = state.manager.write().await;
    Ok(manager.activate_profile(&tool_id, &name)?)
}

/// 获取当前激活的 Profile 名称
#[tauri::command]
pub async fn pm_get_active_profile_name(
    state: tauri::State<'_, ProfileManagerState>,
    tool_id: String,
) -> AppResult<Option<String>> {
    let manager = state.manager.read().await;
    Ok(manager.get_active_profile_name(&tool_id)?)
}

/// 从原生配置文件捕获 Profile
#[tauri::command]
pub async fn pm_capture_from_native(
    state: tauri::State<'_, ProfileManagerState>,
    tool_id: String,
    name: String,
) -> AppResult<()> {
    let manager = state.manager.write().await;
    Ok(manager.capture_from_native(&tool_id, &name)?)
}

// ==================== AMP Profile Selection ====================

/// AMP Profile 选择输入（前端传递）
#[derive(Debug, Deserialize)]
pub struct AmpSelectionInput {
    pub claude: Option<ProfileRefInput>,
    pub codex: Option<ProfileRefInput>,
    pub gemini: Option<ProfileRefInput>,
}

#[derive(Debug, Deserialize)]
pub struct ProfileRefInput {
    pub tool_id: String,
    pub profile_name: String,
}

/// 获取 AMP Profile 选择
#[tauri::command]
pub async fn pm_get_amp_selection(
    state: tauri::State<'_, ProfileManagerState>,
) -> AppResult<AmpProfileSelection> {
    let manager = state.manager.read().await;
    Ok(manager.get_amp_selection()?)
}

/// 保存 AMP Profile 选择
#[tauri::command]
pub async fn pm_save_amp_selection(
    state: tauri::State<'_, ProfileManagerState>,
    input: AmpSelectionInput,
) -> AppResult<()> {
    let manager = state.manager.write().await;

    let selection = AmpProfileSelection {
        claude: input.claude.map(|r| ProfileRef {
            tool_id: r.tool_id,
            profile_name: r.profile_name,
        }),
        codex: input.codex.map(|r| ProfileRef {
            tool_id: r.tool_id,
            profile_name: r.profile_name,
        }),
        gemini: input.gemini.map(|r| ProfileRef {
            tool_id: r.tool_id,
            profile_name: r.profile_name,
        }),
        updated_at: chrono::Utc::now(),
    };

    Ok(manager.save_amp_selection(&selection)?)
}
