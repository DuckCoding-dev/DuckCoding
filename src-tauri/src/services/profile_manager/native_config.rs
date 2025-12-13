//! 原生配置文件同步逻辑（v2.1 - 简化版）

use super::types::*;
use crate::data::DataManager;
use crate::models::tool::Tool;
use anyhow::{anyhow, Result};
use serde_json::{Map, Value};
use toml_edit;

impl super::manager::ProfileManager {
    /// 将 Profile 应用到原生配置文件
    pub fn apply_profile_to_native(&self, tool_id: &str, profile_name: &str) -> Result<()> {
        let tool = Tool::by_id(tool_id).ok_or_else(|| anyhow!("未找到工具: {}", tool_id))?;

        match tool_id {
            "claude-code" => {
                let profile = self.get_claude_profile(profile_name)?;
                apply_claude_native(&tool, &profile)?;
            }
            "codex" => {
                let profile = self.get_codex_profile(profile_name)?;
                // 使用 profile_name 作为 provider 名称
                apply_codex_native(&tool, &profile, profile_name)?;
            }
            "gemini-cli" => {
                let profile = self.get_gemini_profile(profile_name)?;
                apply_gemini_native(&tool, &profile)?;
            }
            _ => return Err(anyhow!("不支持的工具: {}", tool_id)),
        }

        tracing::info!("已应用 Profile: {} / {}", tool_id, profile_name);
        Ok(())
    }

    /// 从原生配置捕获 Profile
    pub fn capture_profile_from_native(&self, tool_id: &str, profile_name: &str) -> Result<()> {
        let tool = Tool::by_id(tool_id).ok_or_else(|| anyhow!("未找到工具: {}", tool_id))?;

        match tool_id {
            "claude-code" => {
                let (api_key, base_url) = capture_claude_config(&tool)?;
                self.save_claude_profile(profile_name, api_key, base_url)?;
            }
            "codex" => {
                let (api_key, base_url, wire_api) = capture_codex_config(&tool)?;
                self.save_codex_profile(profile_name, api_key, base_url, Some(wire_api))?;
            }
            "gemini-cli" => {
                let (api_key, base_url, model) = capture_gemini_config(&tool)?;
                self.save_gemini_profile(profile_name, api_key, base_url, Some(model))?;
            }
            _ => return Err(anyhow!("不支持的工具: {}", tool_id)),
        }

        tracing::info!("已捕获 Profile: {} / {}", tool_id, profile_name);
        Ok(())
    }
}

// ==================== Claude Code ====================

fn apply_claude_native(tool: &Tool, profile: &ClaudeProfile) -> Result<()> {
    let manager = DataManager::new();
    let settings_path = tool.config_dir.join("settings.json");

    let mut settings: Value = if settings_path.exists() {
        manager.json_uncached().read(&settings_path)?
    } else {
        serde_json::json!({})
    };

    let obj = settings.as_object_mut().unwrap();
    if !obj.contains_key("env") {
        obj.insert("env".to_string(), Value::Object(Map::new()));
    }

    let env = obj.get_mut("env").unwrap().as_object_mut().unwrap();
    env.insert(
        "ANTHROPIC_AUTH_TOKEN".to_string(),
        Value::String(profile.api_key.clone()),
    );
    env.insert(
        "ANTHROPIC_BASE_URL".to_string(),
        Value::String(profile.base_url.clone()),
    );

    manager.json_uncached().write(&settings_path, &settings)?;
    Ok(())
}

fn capture_claude_config(tool: &Tool) -> Result<(String, String)> {
    let manager = DataManager::new();
    let settings_path = tool.config_dir.join("settings.json");

    let settings: Value = manager.json_uncached().read(&settings_path)?;
    let env = settings
        .get("env")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow!("缺少 env"))?;

    let api_key = env
        .get("ANTHROPIC_AUTH_TOKEN")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let base_url = env
        .get("ANTHROPIC_BASE_URL")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok((api_key, base_url))
}

// ==================== Codex ====================

fn apply_codex_native(tool: &Tool, profile: &CodexProfile, provider_name: &str) -> Result<()> {
    let manager = DataManager::new();
    let config_path = tool.config_dir.join("config.toml");
    let auth_path = tool.config_dir.join("auth.json");

    let mut doc = if config_path.exists() {
        manager.toml().read_document(&config_path)?
    } else {
        toml_edit::DocumentMut::new()
    };

    let root_table = doc.as_table_mut();

    // 设置默认值
    if !root_table.contains_key("model") {
        root_table.insert("model", toml_edit::value("gpt-5-codex"));
    }
    if !root_table.contains_key("model_reasoning_effort") {
        root_table.insert("model_reasoning_effort", toml_edit::value("high"));
    }
    if !root_table.contains_key("network_access") {
        root_table.insert("network_access", toml_edit::value("enabled"));
    }

    // 设置 model_provider 为 profile_name
    root_table.insert("model_provider", toml_edit::value(provider_name));

    // 处理 base_url
    let normalized = profile.base_url.trim_end_matches('/');
    let base_url_with_v1 = if normalized.ends_with("/v1") {
        normalized.to_string()
    } else {
        format!("{}/v1", normalized)
    };

    // 创建或更新 model_providers 表
    if !root_table.contains_key("model_providers") {
        let mut table = toml_edit::Table::new();
        table.set_implicit(false);
        root_table.insert("model_providers", toml_edit::Item::Table(table));
    }

    let providers_table = root_table
        .get_mut("model_providers")
        .unwrap()
        .as_table_mut()
        .unwrap();

    // 检查或创建 provider
    if !providers_table.contains_key(provider_name) {
        let mut table = toml_edit::Table::new();
        table.set_implicit(false);
        // 初始化所有必要字段
        table.insert("name", toml_edit::value(provider_name));
        table.insert("base_url", toml_edit::value(&base_url_with_v1));
        table.insert("wire_api", toml_edit::value(&profile.wire_api));
        table.insert("requires_openai_auth", toml_edit::value(true));
        providers_table.insert(provider_name, toml_edit::Item::Table(table));
        tracing::info!("创建新的 Codex provider: {}", provider_name);
    } else {
        // provider 已存在，检查是否需要更新
        let provider_table = providers_table
            .get_mut(provider_name)
            .unwrap()
            .as_table_mut()
            .unwrap();

        let current_base_url = provider_table
            .get("base_url")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let current_wire_api = provider_table
            .get("wire_api")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if current_base_url != base_url_with_v1 || current_wire_api != profile.wire_api {
            provider_table.insert("name", toml_edit::value(provider_name));
            provider_table.insert("base_url", toml_edit::value(&base_url_with_v1));
            provider_table.insert("wire_api", toml_edit::value(&profile.wire_api));
            provider_table.insert("requires_openai_auth", toml_edit::value(true));
            tracing::info!("更新 Codex provider 配置: {}", provider_name);
        }
    }

    manager.toml().write(&config_path, &doc)?;

    // 应用 auth.json
    let mut auth = if auth_path.exists() {
        manager.json_uncached().read(&auth_path)?
    } else {
        serde_json::json!({})
    };

    auth.as_object_mut().unwrap().insert(
        "OPENAI_API_KEY".to_string(),
        Value::String(profile.api_key.clone()),
    );
    manager.json_uncached().write(&auth_path, &auth)?;

    Ok(())
}

fn capture_codex_config(tool: &Tool) -> Result<(String, String, String)> {
    let manager = DataManager::new();
    let config_path = tool.config_dir.join("config.toml");
    let auth_path = tool.config_dir.join("auth.json");

    // 读取 API Key
    let auth: Value = manager.json_uncached().read(&auth_path)?;
    let api_key = auth
        .get("OPENAI_API_KEY")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // 读取当前 model_provider
    let doc = manager.toml().read_document(&config_path)?;
    let current_provider = doc
        .get("model_provider")
        .and_then(|v| v.as_str())
        .unwrap_or("responses");

    let mut base_url = String::new();
    let mut wire_api = "responses".to_string();

    // 从当前 provider 读取配置
    if let Some(providers) = doc.get("model_providers").and_then(|v| v.as_table()) {
        if let Some(p_table) = providers.get(current_provider).and_then(|v| v.as_table()) {
            if let Some(url) = p_table.get("base_url").and_then(|v| v.as_str()) {
                base_url = url.to_string();
            }
            if let Some(api) = p_table.get("wire_api").and_then(|v| v.as_str()) {
                wire_api = api.to_string();
            }
        }
    }

    Ok((api_key, base_url, wire_api))
}

// ==================== Gemini CLI ====================

fn apply_gemini_native(tool: &Tool, profile: &GeminiProfile) -> Result<()> {
    let manager = DataManager::new();
    let env_path = tool.config_dir.join(".env");

    manager
        .env()
        .set(&env_path, "GEMINI_API_KEY", &profile.api_key)?;
    manager
        .env()
        .set(&env_path, "GOOGLE_GEMINI_BASE_URL", &profile.base_url)?;

    // 只在 model 有值时才写入
    if let Some(ref model) = profile.model {
        manager.env().set(&env_path, "GEMINI_MODEL", model)?;
    }

    Ok(())
}

fn capture_gemini_config(tool: &Tool) -> Result<(String, String, String)> {
    let manager = DataManager::new();
    let env_path = tool.config_dir.join(".env");

    let env_lines = manager.env().read_raw(&env_path)?;
    let mut api_key = String::new();
    let mut base_url = String::new();
    let mut model = "gemini-2.0-flash-exp".to_string();

    for line in &env_lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                "GEMINI_API_KEY" => api_key = value.trim().to_string(),
                "GOOGLE_GEMINI_BASE_URL" => base_url = value.trim().to_string(),
                "GEMINI_MODEL" => model = value.trim().to_string(),
                _ => {}
            }
        }
    }

    Ok((api_key, base_url, model))
}
