//! Claude Code 配置管理模块

use super::types::ClaudeSettingsPayload;
use super::ToolConfigManager;
use crate::data::DataManager;
use crate::models::Tool;
use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use serde_json::{Map, Value};
use std::fs;

/// Claude Code 配置管理器
pub struct ClaudeConfigManager;

impl ToolConfigManager for ClaudeConfigManager {
    type Settings = Value;
    type Payload = ClaudeSettingsPayload;

    fn read_settings() -> Result<Self::Settings> {
        read_claude_settings()
    }

    fn save_settings(payload: &Self::Payload) -> Result<()> {
        save_claude_settings(&payload.settings, payload.extra_config.as_ref())
    }

    fn get_schema() -> Result<Value> {
        get_claude_schema()
    }
}

/// 读取 Claude Code 主配置文件（settings.json）
///
/// # Returns
///
/// 返回配置 JSON 对象，如果文件不存在则返回空对象
///
/// # Errors
///
/// 当文件读取失败或 JSON 解析失败时返回错误
pub fn read_claude_settings() -> Result<Value> {
    let tool = Tool::claude_code();
    let config_path = tool.config_dir.join(&tool.config_file);

    if !config_path.exists() {
        return Ok(Value::Object(Map::new()));
    }

    let manager = DataManager::new();
    let settings = manager
        .json_uncached()
        .read(&config_path)
        .context("读取 Claude Code 配置失败")?;

    Ok(settings)
}

/// 读取 Claude Code 附属配置文件（config.json）
///
/// # Returns
///
/// 返回配置 JSON 对象，如果文件不存在则返回空对象
///
/// # Errors
///
/// 当文件读取失败或 JSON 解析失败时返回错误
pub fn read_claude_extra_config() -> Result<Value> {
    let tool = Tool::claude_code();
    let extra_path = tool.config_dir.join("config.json");
    if !extra_path.exists() {
        return Ok(Value::Object(Map::new()));
    }
    let manager = DataManager::new();
    let json = manager
        .json_uncached()
        .read(&extra_path)
        .context("读取 Claude Code config.json 失败")?;
    Ok(json)
}

/// 保存 Claude Code 完整配置
///
/// # Arguments
///
/// * `settings` - 主配置（settings.json）内容
/// * `extra_config` - 可选的附属配置（config.json）内容
///
/// # Errors
///
/// 当配置不是有效的 JSON 对象或写入失败时返回错误
pub fn save_claude_settings(settings: &Value, extra_config: Option<&Value>) -> Result<()> {
    if !settings.is_object() {
        anyhow::bail!("Claude Code 配置必须是 JSON 对象");
    }

    let tool = Tool::claude_code();
    let config_dir = &tool.config_dir;
    let config_path = config_dir.join(&tool.config_file);
    let extra_config_path = config_dir.join("config.json");

    fs::create_dir_all(config_dir).context("创建 Claude Code 配置目录失败")?;

    let manager = DataManager::new();
    manager
        .json_uncached()
        .write(&config_path, settings)
        .context("写入 Claude Code 配置失败")?;

    if let Some(extra) = extra_config {
        if !extra.is_object() {
            anyhow::bail!("Claude Code config.json 必须是 JSON 对象");
        }
        manager
            .json_uncached()
            .write(&extra_config_path, extra)
            .context("写入 Claude Code config.json 失败")?;
    }

    Ok(())
}

/// 获取内置的 Claude Code JSON Schema
///
/// # Returns
///
/// 返回 JSON Schema 对象
///
/// # Errors
///
/// 当 Schema 解析失败时返回错误
pub fn get_claude_schema() -> Result<Value> {
    static CLAUDE_SCHEMA: OnceCell<Value> = OnceCell::new();

    let schema = CLAUDE_SCHEMA.get_or_try_init(|| {
        let raw = include_str!("../../../resources/claude_code_settings.schema.json");
        serde_json::from_str(raw).context("解析 Claude Code Schema 失败")
    })?;

    Ok(schema.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_claude_settings_writes_extra_config() -> Result<()> {
        // TODO: 需要更新测试逻辑
        unimplemented!("需要更新测试逻辑")
    }

    #[test]
    #[ignore = "需要使用 ProfileManager API 重写"]
    fn detect_external_changes_tracks_claude_extra_config() -> Result<()> {
        // TODO: 需要使用 ProfileManager API 重写此测试
        unimplemented!("需要使用 ProfileManager API 重写此测试")
    }

    #[test]
    #[ignore = "需要使用 ProfileManager API 重写"]
    fn apply_config_persists_claude_profile_and_state() -> Result<()> {
        // TODO: 需要使用 ProfileManager API 重写此测试
        unimplemented!("需要使用 ProfileManager API 重写此测试")
    }
}
