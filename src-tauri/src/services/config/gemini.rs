//! Gemini CLI 配置管理模块

use super::types::{GeminiEnvPayload, GeminiSettingsPayload};
use super::ToolConfigManager;
use crate::data::DataManager;
use crate::models::Tool;
use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Gemini CLI 配置管理器
pub struct GeminiConfigManager;

impl ToolConfigManager for GeminiConfigManager {
    type Settings = GeminiSettingsPayload;
    type Payload = GeminiSettingsPayload;

    fn read_settings() -> Result<Self::Settings> {
        read_gemini_settings()
    }

    fn save_settings(payload: &Self::Payload) -> Result<()> {
        save_gemini_settings(&payload.settings, &payload.env)
    }

    fn get_schema() -> Result<Value> {
        get_gemini_schema()
    }
}

/// 读取 Gemini CLI 配置（settings.json 和 .env）
///
/// # Returns
///
/// 返回包含配置和环境变量的 Payload
///
/// # Errors
///
/// 当文件读取失败或解析失败时返回错误
pub fn read_gemini_settings() -> Result<GeminiSettingsPayload> {
    let tool = Tool::gemini_cli();
    let settings_path = tool.config_dir.join(&tool.config_file);
    let env_path = tool.config_dir.join(".env");
    let manager = DataManager::new();

    let settings = if settings_path.exists() {
        manager
            .json_uncached()
            .read(&settings_path)
            .context("读取 Gemini CLI 配置失败")?
    } else {
        Value::Object(Map::new())
    };

    let env = read_gemini_env(&env_path)?;

    Ok(GeminiSettingsPayload { settings, env })
}

/// 保存 Gemini CLI 配置和环境变量
///
/// # Arguments
///
/// * `settings` - 配置对象（将保存到 settings.json）
/// * `env` - 环境变量（将保存到 .env）
///
/// # Errors
///
/// 当配置不是有效对象或写入失败时返回错误
pub fn save_gemini_settings(settings: &Value, env: &GeminiEnvPayload) -> Result<()> {
    if !settings.is_object() {
        anyhow::bail!("Gemini CLI 配置必须是 JSON 对象");
    }

    let tool = Tool::gemini_cli();
    let config_dir = &tool.config_dir;
    let settings_path = config_dir.join(&tool.config_file);
    let env_path = config_dir.join(".env");
    let manager = DataManager::new();

    fs::create_dir_all(config_dir).context("创建 Gemini CLI 配置目录失败")?;

    manager
        .json_uncached()
        .write(&settings_path, settings)
        .context("写入 Gemini CLI 配置失败")?;

    let mut env_pairs = read_env_pairs(&env_path)?;
    env_pairs.insert("GEMINI_API_KEY".to_string(), env.api_key.clone());
    env_pairs.insert("GOOGLE_GEMINI_BASE_URL".to_string(), env.base_url.clone());
    env_pairs.insert(
        "GEMINI_MODEL".to_string(),
        if env.model.trim().is_empty() {
            "gemini-2.5-pro".to_string()
        } else {
            env.model.clone()
        },
    );
    write_env_pairs(&env_path, &env_pairs).context("写入 Gemini CLI .env 失败")?;

    Ok(())
}

/// 获取 Gemini CLI 配置 JSON Schema
///
/// # Returns
///
/// 返回 JSON Schema 对象
///
/// # Errors
///
/// 当 Schema 解析失败时返回错误
pub fn get_gemini_schema() -> Result<Value> {
    static GEMINI_SCHEMA: OnceCell<Value> = OnceCell::new();
    let schema = GEMINI_SCHEMA.get_or_try_init(|| {
        let raw = include_str!("../../../resources/gemini_cli_settings.schema.json");
        serde_json::from_str(raw).context("解析 Gemini CLI Schema 失败")
    })?;

    Ok(schema.clone())
}

/// 读取 .env 文件并解析为 GeminiEnvPayload
fn read_gemini_env(path: &Path) -> Result<GeminiEnvPayload> {
    if !path.exists() {
        return Ok(GeminiEnvPayload {
            model: "gemini-2.5-pro".to_string(),
            ..GeminiEnvPayload::default()
        });
    }

    let env_pairs = read_env_pairs(path)?;
    Ok(GeminiEnvPayload {
        api_key: env_pairs.get("GEMINI_API_KEY").cloned().unwrap_or_default(),
        base_url: env_pairs
            .get("GOOGLE_GEMINI_BASE_URL")
            .cloned()
            .unwrap_or_default(),
        model: env_pairs
            .get("GEMINI_MODEL")
            .cloned()
            .unwrap_or_else(|| "gemini-2.5-pro".to_string()),
    })
}

/// 读取 .env 文件为键值对
fn read_env_pairs(path: &Path) -> Result<HashMap<String, String>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let manager = DataManager::new();
    manager.env().read(path).map_err(|e| anyhow::anyhow!(e))
}

/// 写入键值对到 .env 文件
fn write_env_pairs(path: &Path, pairs: &HashMap<String, String>) -> Result<()> {
    let manager = DataManager::new();
    manager
        .env()
        .write(path, pairs)
        .map_err(|e| anyhow::anyhow!(e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "需要使用 ProfileManager API 重写"]
    fn apply_config_gemini_sets_model_and_env() -> Result<()> {
        // TODO: 需要使用 ProfileManager API 重写此测试
        unimplemented!("需要使用 ProfileManager API 重写此测试")
    }

    #[test]
    #[ignore = "需要使用 ProfileManager API 重写"]
    fn detect_external_changes_tracks_gemini_env_file() -> Result<()> {
        // TODO: 需要使用 ProfileManager API 重写此测试
        unimplemented!("需要使用 ProfileManager API 重写此测试")
    }
}
