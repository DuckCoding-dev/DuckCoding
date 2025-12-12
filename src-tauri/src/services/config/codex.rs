//! Codex 配置管理模块

use super::types::CodexSettingsPayload;
use super::utils::merge_toml_tables;
use super::ToolConfigManager;
use crate::data::DataManager;
use crate::models::Tool;
use anyhow::{anyhow, Context, Result};
use once_cell::sync::OnceCell;
use serde_json::{Map, Value};
use std::fs;
use toml;
use toml_edit::DocumentMut;

/// Codex 配置管理器
pub struct CodexConfigManager;

impl ToolConfigManager for CodexConfigManager {
    type Settings = CodexSettingsPayload;
    type Payload = CodexSettingsPayload;

    fn read_settings() -> Result<Self::Settings> {
        read_codex_settings()
    }

    fn save_settings(payload: &Self::Payload) -> Result<()> {
        save_codex_settings(&payload.config, payload.auth_token.clone())
    }

    fn get_schema() -> Result<Value> {
        get_codex_schema()
    }
}

/// 读取 Codex 配置（config.toml 和 auth.json）
///
/// # Returns
///
/// 返回包含配置和认证令牌的 Payload
///
/// # Errors
///
/// 当文件读取失败或解析失败时返回错误
pub fn read_codex_settings() -> Result<CodexSettingsPayload> {
    let tool = Tool::codex();
    let config_path = tool.config_dir.join(&tool.config_file);
    let auth_path = tool.config_dir.join("auth.json");
    let manager = DataManager::new();

    let config_value = if config_path.exists() {
        let doc = manager
            .toml()
            .read(&config_path)
            .context("读取 Codex config.toml 失败")?;
        serde_json::to_value(&doc).context("转换 Codex config.toml 为 JSON 失败")?
    } else {
        Value::Object(Map::new())
    };

    let auth_token = if auth_path.exists() {
        let auth = manager
            .json_uncached()
            .read(&auth_path)
            .context("读取 Codex auth.json 失败")?;
        auth.get("OPENAI_API_KEY")
            .and_then(|s| s.as_str().map(|s| s.to_string()))
    } else {
        None
    };

    Ok(CodexSettingsPayload {
        config: config_value,
        auth_token,
    })
}

/// 保存 Codex 配置和认证令牌
///
/// # Arguments
///
/// * `config` - 配置对象（将保存到 config.toml）
/// * `auth_token` - 可选的 OpenAI API Key（将保存到 auth.json）
///
/// # Errors
///
/// 当配置不是有效对象或写入失败时返回错误
pub fn save_codex_settings(config: &Value, auth_token: Option<String>) -> Result<()> {
    if !config.is_object() {
        anyhow::bail!("Codex 配置必须是对象结构");
    }

    let tool = Tool::codex();
    let config_path = tool.config_dir.join(&tool.config_file);
    let auth_path = tool.config_dir.join("auth.json");
    let manager = DataManager::new();

    fs::create_dir_all(&tool.config_dir).context("创建 Codex 配置目录失败")?;

    // 读取现有 TOML 文档以保留注释和格式
    let mut existing_doc = if config_path.exists() {
        manager
            .toml()
            .read_document(&config_path)
            .context("读取 Codex config.toml 失败")?
    } else {
        DocumentMut::new()
    };

    // 将新配置序列化为 TOML 并解析
    let new_toml_string = toml::to_string(config).context("序列化 Codex config 失败")?;
    let new_doc = new_toml_string
        .parse::<DocumentMut>()
        .map_err(|err| anyhow!("解析待写入 Codex 配置失败: {err}"))?;

    // 合并配置，保留注释
    merge_toml_tables(existing_doc.as_table_mut(), new_doc.as_table());

    manager
        .toml()
        .write(&config_path, &existing_doc)
        .context("写入 Codex config.toml 失败")?;

    // 保存认证令牌
    if let Some(token) = auth_token {
        let mut auth_data = if auth_path.exists() {
            manager
                .json_uncached()
                .read(&auth_path)
                .unwrap_or(Value::Object(Map::new()))
        } else {
            Value::Object(Map::new())
        };

        if let Value::Object(ref mut obj) = auth_data {
            obj.insert("OPENAI_API_KEY".to_string(), Value::String(token));
        }

        manager
            .json_uncached()
            .write(&auth_path, &auth_data)
            .context("写入 Codex auth.json 失败")?;
    }

    Ok(())
}

/// 获取 Codex 配置 JSON Schema
///
/// # Returns
///
/// 返回 JSON Schema 对象
///
/// # Errors
///
/// 当 Schema 解析失败时返回错误
pub fn get_codex_schema() -> Result<Value> {
    static CODEX_SCHEMA: OnceCell<Value> = OnceCell::new();
    let schema = CODEX_SCHEMA.get_or_try_init(|| {
        let raw = include_str!("../../../resources/codex_config.schema.json");
        serde_json::from_str(raw).context("解析 Codex Schema 失败")
    })?;

    Ok(schema.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "需要使用 ProfileManager API 重写"]
    fn apply_config_codex_sets_provider_and_auth() -> Result<()> {
        // TODO: 需要使用 ProfileManager API 重写此测试
        unimplemented!("需要使用 ProfileManager API 重写此测试")
    }

    #[test]
    #[ignore = "需要使用 ProfileManager API 重写"]
    fn detect_external_changes_tracks_codex_auth_file() -> Result<()> {
        // TODO: 需要使用 ProfileManager API 重写此测试
        unimplemented!("需要使用 ProfileManager API 重写此测试")
    }

    #[test]
    #[ignore = "需要使用 ProfileManager API 重写"]
    fn import_external_change_for_codex_writes_profile_and_state() -> Result<()> {
        // TODO: 需要使用 ProfileManager API 重写此测试
        unimplemented!("需要使用 ProfileManager API 重写此测试")
    }
}
