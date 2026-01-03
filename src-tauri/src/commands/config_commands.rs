// 配置管理相关命令

use super::error::{AppError, AppResult};
use serde_json::Value;

use ::duckcoding::services::config::{
    self, claude, codex, gemini, ClaudeSettingsPayload, CodexSettingsPayload, ExternalConfigChange,
    GeminiEnvPayload, GeminiSettingsPayload, ImportExternalChangeResult,
};
use ::duckcoding::services::proxy::config::apply_global_proxy;
use ::duckcoding::utils::config::{read_global_config, write_global_config};
use ::duckcoding::GlobalConfig;
use ::duckcoding::Tool;

// ==================== 类型定义 ====================

// ==================== Token 生成类型 ====================

#[derive(serde::Deserialize, Debug)]
struct TokenData {
    id: i64,
    key: String,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    group: String,
}

#[derive(serde::Deserialize, Debug)]
struct ApiResponse {
    success: bool,
    message: String,
    data: Option<Vec<TokenData>>,
}

#[derive(serde::Serialize)]
pub struct GenerateApiKeyResult {
    success: bool,
    message: String,
    api_key: Option<String>,
}

// ==================== 辅助函数 ====================

fn build_reqwest_client() -> Result<reqwest::Client, String> {
    ::duckcoding::http_client::build_client()
}

// ==================== Tauri 命令 ====================

/// 检测外部配置变更
#[tauri::command]
pub async fn get_external_changes() -> Result<Vec<ExternalConfigChange>, String> {
    config::detect_external_changes().map_err(|e| e.to_string())
}

/// 确认外部变更（清除脏标记并刷新 checksum）
#[tauri::command]
pub async fn ack_external_change(tool: String) -> AppResult<()> {
    let tool_obj =
        Tool::by_id(&tool).ok_or_else(|| AppError::ToolNotFound { tool: tool.clone() })?;
    Ok(config::acknowledge_external_change(&tool_obj)?)
}

/// 将外部修改导入集中仓
#[tauri::command]
pub async fn import_native_change(
    tool: String,
    profile: String,
    as_new: bool,
) -> AppResult<ImportExternalChangeResult> {
    let tool_obj =
        Tool::by_id(&tool).ok_or_else(|| AppError::ToolNotFound { tool: tool.clone() })?;
    Ok(config::import_external_change(&tool_obj, &profile, as_new)?)
}

#[tauri::command]
pub async fn save_global_config(config: GlobalConfig) -> Result<(), String> {
    write_global_config(&config)
}

#[tauri::command]
pub async fn get_global_config() -> Result<Option<GlobalConfig>, String> {
    read_global_config()
}

#[tauri::command]
pub async fn generate_api_key_for_tool(tool: String) -> Result<GenerateApiKeyResult, String> {
    // 应用代理配置（如果已配置）
    apply_global_proxy().ok();

    // 读取全局配置
    let global_config = get_global_config()
        .await?
        .ok_or("请先配置用户ID和系统访问令牌")?;

    // 检查已废弃的用户凭证（现由 Provider 系统管理）
    let user_id = global_config
        .user_id
        .as_ref()
        .ok_or("请先配置用户ID（已废弃，建议使用供应商管理功能）")?;
    let system_token = global_config
        .system_token
        .as_ref()
        .ok_or("请先配置系统访问令牌（已废弃，建议使用供应商管理功能）")?;

    // 根据工具名称获取配置
    let (name, group) = match tool.as_str() {
        "claude-code" => ("Claude Code一键创建", "Claude Code专用"),
        "codex" => ("CodeX一键创建", "CodeX专用"),
        "gemini-cli" => ("Gemini CLI一键创建", "Gemini CLI专用"),
        _ => return Err(format!("Unknown tool: {tool}")),
    };

    // 创建token
    let client = build_reqwest_client().map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    let create_url = "https://duckcoding.com/api/token";

    let create_body = serde_json::json!({
        "remain_quota": 500000,
        "expired_time": -1,
        "unlimited_quota": true,
        "model_limits_enabled": false,
        "model_limits": "",
        "name": name,
        "group": group,
        "allow_ips": ""
    });

    let create_response = client
        .post(create_url)
        .header("Authorization", format!("Bearer {}", system_token))
        .header("New-Api-User", user_id)
        .header("Content-Type", "application/json")
        .json(&create_body)
        .send()
        .await
        .map_err(|e| format!("创建token失败: {e}"))?;

    if !create_response.status().is_success() {
        let status = create_response.status();
        let error_text = create_response.text().await.unwrap_or_default();
        return Ok(GenerateApiKeyResult {
            success: false,
            message: format!("创建token失败 ({status}): {error_text}"),
            api_key: None,
        });
    }

    // 等待一小段时间让服务器处理
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // 搜索刚创建的token
    let search_url = format!(
        "https://duckcoding.com/api/token/search?keyword={}",
        urlencoding::encode(name)
    );

    let search_response = client
        .get(&search_url)
        .header("Authorization", format!("Bearer {}", system_token))
        .header("New-Api-User", user_id)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("搜索token失败: {e}"))?;

    if !search_response.status().is_success() {
        return Ok(GenerateApiKeyResult {
            success: false,
            message: "创建成功但获取API Key失败，请稍后在DuckCoding控制台查看".to_string(),
            api_key: None,
        });
    }

    let api_response: ApiResponse = search_response
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {e}"))?;

    if !api_response.success {
        return Ok(GenerateApiKeyResult {
            success: false,
            message: format!("API返回错误: {}", api_response.message),
            api_key: None,
        });
    }

    // 获取id最大的token（最新创建的）
    if let Some(mut data) = api_response.data {
        if !data.is_empty() {
            // 按id降序排序，取第一个（id最大的）
            data.sort_by(|a, b| b.id.cmp(&a.id));
            let token = &data[0];
            let api_key = format!("sk-{}", token.key);
            return Ok(GenerateApiKeyResult {
                success: true,
                message: "API Key生成成功".to_string(),
                api_key: Some(api_key),
            });
        }
    }

    Ok(GenerateApiKeyResult {
        success: false,
        message: "未找到生成的token".to_string(),
        api_key: None,
    })
}

#[tauri::command]
pub fn get_claude_settings() -> Result<ClaudeSettingsPayload, String> {
    claude::read_claude_settings()
        .map(|settings| {
            let extra = claude::read_claude_extra_config().ok();
            ClaudeSettingsPayload {
                settings,
                extra_config: extra,
            }
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_claude_settings(settings: Value, extra_config: Option<Value>) -> Result<(), String> {
    claude::save_claude_settings(&settings, extra_config.as_ref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_claude_schema() -> Result<Value, String> {
    claude::get_claude_schema().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_codex_settings() -> Result<CodexSettingsPayload, String> {
    codex::read_codex_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_codex_settings(settings: Value, auth_token: Option<String>) -> Result<(), String> {
    codex::save_codex_settings(&settings, auth_token).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_codex_schema() -> Result<Value, String> {
    codex::get_codex_schema().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_gemini_settings() -> Result<GeminiSettingsPayload, String> {
    gemini::read_gemini_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_gemini_settings(settings: Value, env: GeminiEnvPayload) -> Result<(), String> {
    gemini::save_gemini_settings(&settings, &env).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_gemini_schema() -> Result<Value, String> {
    gemini::get_gemini_schema().map_err(|e| e.to_string())
}

// ==================== 单实例模式配置命令 ====================

/// 获取单实例模式配置状态
#[tauri::command]
pub async fn get_single_instance_config() -> Result<bool, String> {
    let config = read_global_config()
        .map_err(|e| format!("读取配置失败: {e}"))?
        .ok_or("配置文件不存在")?;
    Ok(config.single_instance_enabled)
}

/// 更新单实例模式配置（需要重启应用生效）
#[tauri::command]
pub async fn update_single_instance_config(enabled: bool) -> Result<(), String> {
    let mut config = read_global_config()
        .map_err(|e| format!("读取配置失败: {e}"))?
        .ok_or("配置文件不存在")?;

    config.single_instance_enabled = enabled;

    write_global_config(&config).map_err(|e| format!("保存配置失败: {e}"))?;

    tracing::info!(enabled = enabled, "单实例模式配置已更新（需重启生效）");

    Ok(())
}
