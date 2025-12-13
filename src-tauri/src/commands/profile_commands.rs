//! Profile 管理 Tauri 命令（v2.1 - 简化版）

use ::duckcoding::services::profile_manager::{ProfileDescriptor, ProfileManager};
use anyhow::Result;
use serde::Deserialize;

/// Profile 输入数据（前端传递）
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ProfileInput {
    Gemini {
        api_key: String,
        base_url: String,
        #[serde(default)]
        model: Option<String>,
    },
    Codex {
        api_key: String,
        base_url: String,
        wire_api: String, // 前端和后端都使用 wire_api
    },
    Claude {
        api_key: String,
        base_url: String,
    },
}

/// 列出所有 Profile 描述符
#[tauri::command]
pub async fn pm_list_all_profiles() -> Result<Vec<ProfileDescriptor>, String> {
    let manager = ProfileManager::new().map_err(|e| e.to_string())?;
    manager.list_all_descriptors().map_err(|e| e.to_string())
}

/// 列出指定工具的 Profile 名称
#[tauri::command]
pub async fn pm_list_tool_profiles(tool_id: String) -> Result<Vec<String>, String> {
    let manager = ProfileManager::new().map_err(|e| e.to_string())?;
    manager.list_profiles(&tool_id).map_err(|e| e.to_string())
}

/// 获取指定 Profile（返回 JSON 供前端使用）
#[tauri::command]
pub async fn pm_get_profile(tool_id: String, name: String) -> Result<serde_json::Value, String> {
    let manager = ProfileManager::new().map_err(|e| e.to_string())?;

    let value = match tool_id.as_str() {
        "claude-code" => {
            let profile = manager
                .get_claude_profile(&name)
                .map_err(|e| e.to_string())?;
            serde_json::to_value(&profile).map_err(|e| e.to_string())?
        }
        "codex" => {
            let profile = manager
                .get_codex_profile(&name)
                .map_err(|e| e.to_string())?;
            serde_json::to_value(&profile).map_err(|e| e.to_string())?
        }
        "gemini-cli" => {
            let profile = manager
                .get_gemini_profile(&name)
                .map_err(|e| e.to_string())?;
            serde_json::to_value(&profile).map_err(|e| e.to_string())?
        }
        _ => return Err(format!("不支持的工具 ID: {}", tool_id)),
    };

    Ok(value)
}

/// 获取当前激活的 Profile（返回 JSON 供前端使用）
#[tauri::command]
pub async fn pm_get_active_profile(tool_id: String) -> Result<Option<serde_json::Value>, String> {
    let manager = ProfileManager::new().map_err(|e| e.to_string())?;
    let name = manager
        .get_active_profile_name(&tool_id)
        .map_err(|e| e.to_string())?;

    if let Some(profile_name) = name {
        pm_get_profile(tool_id, profile_name).await.map(Some)
    } else {
        Ok(None)
    }
}

/// 保存 Profile（创建或更新）
#[tauri::command]
pub async fn pm_save_profile(
    tool_id: String,
    name: String,
    input: ProfileInput,
) -> Result<(), String> {
    let manager = ProfileManager::new().map_err(|e| e.to_string())?;

    match tool_id.as_str() {
        "claude-code" => {
            if let ProfileInput::Claude { api_key, base_url } = input {
                manager.save_claude_profile(&name, api_key, base_url)
            } else {
                Err(anyhow::anyhow!("Claude Code 需要 Claude Profile 数据"))
            }
        }
        "codex" => {
            if let ProfileInput::Codex {
                api_key,
                base_url,
                wire_api,
            } = input
            {
                manager.save_codex_profile(&name, api_key, base_url, Some(wire_api))
            } else {
                Err(anyhow::anyhow!("Codex 需要 Codex Profile 数据"))
            }
        }
        "gemini-cli" => {
            if let ProfileInput::Gemini {
                api_key,
                base_url,
                model,
            } = input
            {
                manager.save_gemini_profile(&name, api_key, base_url, model)
            } else {
                Err(anyhow::anyhow!("Gemini CLI 需要 Gemini Profile 数据"))
            }
        }
        _ => Err(anyhow::anyhow!("不支持的工具 ID: {}", tool_id)),
    }
    .map_err(|e| e.to_string())
}

/// 删除 Profile
#[tauri::command]
pub async fn pm_delete_profile(tool_id: String, name: String) -> Result<(), String> {
    let manager = ProfileManager::new().map_err(|e| e.to_string())?;
    manager
        .delete_profile(&tool_id, &name)
        .map_err(|e| e.to_string())
}

/// 激活 Profile
#[tauri::command]
pub async fn pm_activate_profile(tool_id: String, name: String) -> Result<(), String> {
    let manager = ProfileManager::new().map_err(|e| e.to_string())?;
    manager
        .activate_profile(&tool_id, &name)
        .map_err(|e| e.to_string())
}

/// 获取当前激活的 Profile 名称
#[tauri::command]
pub async fn pm_get_active_profile_name(tool_id: String) -> Result<Option<String>, String> {
    let manager = ProfileManager::new().map_err(|e| e.to_string())?;
    manager
        .get_active_profile_name(&tool_id)
        .map_err(|e| e.to_string())
}

/// 从原生配置文件捕获 Profile
#[tauri::command]
pub async fn pm_capture_from_native(tool_id: String, name: String) -> Result<(), String> {
    let manager = ProfileManager::new().map_err(|e| e.to_string())?;
    manager
        .capture_from_native(&tool_id, &name)
        .map_err(|e| e.to_string())
}
