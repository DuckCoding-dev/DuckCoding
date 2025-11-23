use crate::models::Tool;
use anyhow::{anyhow, Context, Result};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use toml;
use toml_edit::{DocumentMut, Item, Table};

// Codex provider 配置必需字段
const CODEX_PROVIDER_REQUIRED_FIELDS: &[&str] =
    &["name", "base_url", "wire_api", "requires_openai_auth"];

/// 检查 Codex provider 配置是否完整（包含所有必需字段）
fn is_complete_provider_config(table: &toml_edit::Table) -> bool {
    CODEX_PROVIDER_REQUIRED_FIELDS
        .iter()
        .all(|field| table.contains_key(field))
}

#[derive(Serialize, Deserialize)]
pub struct CodexSettingsPayload {
    pub config: Value,
    #[serde(rename = "authToken")]
    pub auth_token: Option<String>,
}

fn merge_toml_tables(target: &mut Table, source: &Table) {
    let keys_to_remove: Vec<String> = target
        .iter()
        .map(|(key, _)| key.to_string())
        .filter(|key| !source.contains_key(key))
        .collect();
    for key in keys_to_remove {
        target.remove(&key);
    }

    for (key, item) in source.iter() {
        match item {
            Item::Table(source_table) => {
                let needs_new_table = match target.get(key) {
                    Some(existing) => !existing.is_table(),
                    None => true,
                };

                if needs_new_table {
                    let mut new_table = Table::new();
                    new_table.set_implicit(source_table.is_implicit());
                    target.insert(key, Item::Table(new_table));
                }

                if let Some(target_item) = target.get_mut(key) {
                    if let Some(target_table) = target_item.as_table_mut() {
                        target_table.set_implicit(source_table.is_implicit());
                        merge_toml_tables(target_table, source_table);
                        continue;
                    }
                }

                target.insert(key, item.clone());
            }
            Item::Value(source_value) => {
                let mut updated = false;
                if let Some(existing_item) = target.get_mut(key) {
                    if let Some(existing_value) = existing_item.as_value_mut() {
                        let prefix = existing_value.decor().prefix().cloned();
                        let suffix = existing_value.decor().suffix().cloned();
                        *existing_value = source_value.clone();
                        let decor = existing_value.decor_mut();
                        decor.clear();
                        if let Some(pref) = prefix {
                            decor.set_prefix(pref);
                        }
                        if let Some(suf) = suffix {
                            decor.set_suffix(suf);
                        }
                        updated = true;
                    }
                }

                if !updated {
                    target.insert(key, Item::Value(source_value.clone()));
                }
            }
            _ => {
                target.insert(key, item.clone());
            }
        }
    }
}

fn set_table_value(table: &mut Table, key: &str, value: Item) {
    match value {
        Item::Value(new_value) => {
            if let Some(item) = table.get_mut(key) {
                if let Some(existing) = item.as_value_mut() {
                    *existing = new_value;
                    return;
                }
            }
            table.insert(key, Item::Value(new_value));
        }
        other => {
            table.insert(key, other);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GeminiEnvPayload {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

#[derive(Serialize, Deserialize)]
pub struct GeminiSettingsPayload {
    pub settings: Value,
    pub env: GeminiEnvPayload,
}
/// 配置服务
pub struct ConfigService;

impl ConfigService {
    /// 应用配置（增量更新）
    pub fn apply_config(
        tool: &Tool,
        api_key: &str,
        base_url: &str,
        profile_name: Option<&str>,
    ) -> Result<()> {
        match tool.id.as_str() {
            "claude-code" => Self::apply_claude_config(tool, api_key, base_url)?,
            "codex" => Self::apply_codex_config(tool, api_key, base_url)?,
            "gemini-cli" => Self::apply_gemini_config(tool, api_key, base_url)?,
            _ => anyhow::bail!("未知工具: {}", tool.id),
        }

        // 保存命名配置的备份副本
        if let Some(profile) = profile_name {
            Self::save_backup(tool, profile)?;
        }

        Ok(())
    }

    /// Claude Code 配置
    fn apply_claude_config(tool: &Tool, api_key: &str, base_url: &str) -> Result<()> {
        let config_path = tool.config_dir.join(&tool.config_file);

        // 读取现有配置
        let mut settings = if config_path.exists() {
            let content = fs::read_to_string(&config_path).context("读取配置文件失败")?;
            serde_json::from_str::<Value>(&content).unwrap_or(Value::Object(Map::new()))
        } else {
            Value::Object(Map::new())
        };

        // 确保有 env 字段
        if !settings.is_object() {
            settings = serde_json::json!({});
        }

        let obj = settings.as_object_mut().unwrap();
        if !obj.contains_key("env") {
            obj.insert("env".to_string(), Value::Object(Map::new()));
        }

        // 只更新 API 相关字段
        let env = obj.get_mut("env").unwrap().as_object_mut().unwrap();
        env.insert(
            tool.env_vars.api_key.clone(),
            Value::String(api_key.to_string()),
        );
        env.insert(
            tool.env_vars.base_url.clone(),
            Value::String(base_url.to_string()),
        );

        // 确保目录存在
        fs::create_dir_all(&tool.config_dir)?;

        // 写入配置
        let json = serde_json::to_string_pretty(&settings)?;
        fs::write(&config_path, json)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&config_path)?;
            let mut perms = metadata.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&config_path, perms)?;
        }

        Ok(())
    }

    /// CodeX 配置（使用 toml_edit 保留注释和格式）
    fn apply_codex_config(tool: &Tool, api_key: &str, base_url: &str) -> Result<()> {
        let config_path = tool.config_dir.join(&tool.config_file);
        let auth_path = tool.config_dir.join("auth.json");

        // 确保目录存在
        fs::create_dir_all(&tool.config_dir)?;

        // 读取现有 config.toml（使用 toml_edit 保留注释）
        let mut doc = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            content
                .parse::<toml_edit::DocumentMut>()
                .map_err(|err| anyhow!("解析 Codex config.toml 失败: {err}"))?
        } else {
            toml_edit::DocumentMut::new()
        };
        let root_table = doc.as_table_mut();

        // 判断 provider 类型
        let is_duckcoding = base_url.contains("duckcoding");
        let provider_key = if is_duckcoding {
            "duckcoding"
        } else {
            "custom"
        };

        // 只更新必要字段（保留用户自定义配置和注释）
        if !root_table.contains_key("model") {
            set_table_value(root_table, "model", toml_edit::value("gpt-5-codex"));
        }
        if !root_table.contains_key("model_reasoning_effort") {
            set_table_value(
                root_table,
                "model_reasoning_effort",
                toml_edit::value("high"),
            );
        }
        if !root_table.contains_key("network_access") {
            set_table_value(root_table, "network_access", toml_edit::value("enabled"));
        }

        // 更新 model_provider
        set_table_value(root_table, "model_provider", toml_edit::value(provider_key));

        let normalized_base = base_url.trim_end_matches('/');
        let base_url_with_v1 = if normalized_base.ends_with("/v1") {
            normalized_base.to_string()
        } else {
            format!("{normalized_base}/v1")
        };

        // 增量更新 model_providers 表
        if !root_table
            .get("model_providers")
            .map(|item| item.is_table())
            .unwrap_or(false)
        {
            let mut table = toml_edit::Table::new();
            table.set_implicit(false);
            root_table.insert("model_providers", toml_edit::Item::Table(table));
        }

        let providers_table = root_table
            .get_mut("model_providers")
            .and_then(|item| item.as_table_mut())
            .ok_or_else(|| anyhow!("解析 codex 配置失败：model_providers 不是表结构"))?;

        if !providers_table.contains_key(provider_key) {
            let mut table = toml_edit::Table::new();
            table.set_implicit(false);
            providers_table.insert(provider_key, toml_edit::Item::Table(table));
        }

        if let Some(provider_table) = providers_table
            .get_mut(provider_key)
            .and_then(|item| item.as_table_mut())
        {
            provider_table.insert("name", toml_edit::value(provider_key));
            provider_table.insert("base_url", toml_edit::value(base_url_with_v1));
            provider_table.insert("wire_api", toml_edit::value("responses"));
            provider_table.insert("requires_openai_auth", toml_edit::value(true));
        } else {
            anyhow::bail!("解析 codex 配置失败：无法写入 model_providers.{provider_key}");
        }

        // 写入 config.toml（保留注释和格式）
        fs::write(&config_path, doc.to_string())?;

        // 更新 auth.json（增量）
        let mut auth_data = if auth_path.exists() {
            let content = fs::read_to_string(&auth_path)?;
            serde_json::from_str::<Value>(&content).unwrap_or(Value::Object(Map::new()))
        } else {
            Value::Object(Map::new())
        };

        if let Value::Object(ref mut auth_obj) = auth_data {
            auth_obj.insert(
                "OPENAI_API_KEY".to_string(),
                Value::String(api_key.to_string()),
            );
        }

        fs::write(&auth_path, serde_json::to_string_pretty(&auth_data)?)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for path in [&config_path, &auth_path] {
                if path.exists() {
                    let metadata = fs::metadata(path)?;
                    let mut perms = metadata.permissions();
                    perms.set_mode(0o600);
                    fs::set_permissions(path, perms)?;
                }
            }
        }

        Ok(())
    }

    /// Gemini CLI 配置
    fn apply_gemini_config(tool: &Tool, api_key: &str, base_url: &str) -> Result<()> {
        let env_path = tool.config_dir.join(".env");
        let settings_path = tool.config_dir.join(&tool.config_file);

        // 确保目录存在
        fs::create_dir_all(&tool.config_dir)?;

        // 读取现有 .env
        let mut env_vars = HashMap::new();
        if env_path.exists() {
            let content = fs::read_to_string(&env_path)?;
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    if let Some((key, value)) = trimmed.split_once('=') {
                        env_vars.insert(key.trim().to_string(), value.trim().to_string());
                    }
                }
            }
        }

        // 更新 API 相关字段
        env_vars.insert("GOOGLE_GEMINI_BASE_URL".to_string(), base_url.to_string());
        env_vars.insert("GEMINI_API_KEY".to_string(), api_key.to_string());
        if !env_vars.contains_key("GEMINI_MODEL") {
            env_vars.insert("GEMINI_MODEL".to_string(), "gemini-2.5-pro".to_string());
        }

        // 写入 .env
        let env_content: Vec<String> = env_vars.iter().map(|(k, v)| format!("{k}={v}")).collect();
        fs::write(&env_path, env_content.join("\n") + "\n")?;

        // 读取并更新 settings.json
        let mut settings = if settings_path.exists() {
            let content = fs::read_to_string(&settings_path)?;
            serde_json::from_str::<Value>(&content).unwrap_or(Value::Object(Map::new()))
        } else {
            Value::Object(Map::new())
        };

        if let Value::Object(ref mut obj) = settings {
            if !obj.contains_key("ide") {
                obj.insert("ide".to_string(), serde_json::json!({"enabled": true}));
            }
            if !obj.contains_key("security") {
                obj.insert(
                    "security".to_string(),
                    serde_json::json!({
                        "auth": {"selectedType": "gemini-api-key"}
                    }),
                );
            }
        }

        fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for path in [&env_path, &settings_path] {
                if path.exists() {
                    let metadata = fs::metadata(path)?;
                    let mut perms = metadata.permissions();
                    perms.set_mode(0o600);
                    fs::set_permissions(path, perms)?;
                }
            }
        }

        Ok(())
    }

    /// 保存备份配置
    pub fn save_backup(tool: &Tool, profile_name: &str) -> Result<()> {
        match tool.id.as_str() {
            "claude-code" => Self::backup_claude(tool, profile_name)?,
            "codex" => Self::backup_codex(tool, profile_name)?,
            "gemini-cli" => Self::backup_gemini(tool, profile_name)?,
            _ => anyhow::bail!("未知工具: {}", tool.id),
        }
        Ok(())
    }

    fn backup_claude(tool: &Tool, profile_name: &str) -> Result<()> {
        let config_path = tool.config_dir.join(&tool.config_file);
        let backup_path = tool.backup_path(profile_name);

        if !config_path.exists() {
            anyhow::bail!("配置文件不存在，无法备份");
        }

        // 读取当前配置，只提取 API 相关字段
        let content = fs::read_to_string(&config_path).context("读取配置文件失败")?;
        let settings: Value = serde_json::from_str(&content).context("解析配置文件失败")?;

        // 只保存 API 相关字段
        let backup_data = serde_json::json!({
            "ANTHROPIC_AUTH_TOKEN": settings
                .get("env")
                .and_then(|env| env.get("ANTHROPIC_AUTH_TOKEN"))
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            "ANTHROPIC_BASE_URL": settings
                .get("env")
                .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
        });

        // 写入备份（仅包含 API 字段）
        fs::write(&backup_path, serde_json::to_string_pretty(&backup_data)?)?;

        Ok(())
    }

    fn backup_codex(tool: &Tool, profile_name: &str) -> Result<()> {
        let config_path = tool.config_dir.join("config.toml");
        let auth_path = tool.config_dir.join("auth.json");

        let backup_config = tool.config_dir.join(format!("config.{profile_name}.toml"));
        let backup_auth = tool.config_dir.join(format!("auth.{profile_name}.json"));

        // 读取 auth.json 中的 API Key
        let api_key = if auth_path.exists() {
            let content = fs::read_to_string(&auth_path)?;
            let auth: Value = serde_json::from_str(&content)?;
            auth.get("OPENAI_API_KEY")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        } else {
            String::new()
        };

        // 只保存 API 相关字段到备份
        let backup_auth_data = serde_json::json!({
            "OPENAI_API_KEY": api_key
        });
        fs::write(
            &backup_auth,
            serde_json::to_string_pretty(&backup_auth_data)?,
        )?;

        // 对于 config.toml，只备份当前使用的 provider 的完整配置
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            if let Ok(doc) = content.parse::<toml_edit::DocumentMut>() {
                let mut backup_doc = toml_edit::DocumentMut::new();

                // 获取当前使用的 model_provider
                let current_provider_name = doc
                    .get("model_provider")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("配置文件缺少 model_provider 字段"))?;

                // 只备份当前 provider 的完整配置
                if let Some(providers) = doc.get("model_providers").and_then(|p| p.as_table()) {
                    if let Some(current_provider) = providers.get(current_provider_name) {
                        println!(
                            "📦 备份 Codex 配置：provider = {current_provider_name}, profile = {profile_name}"
                        );
                        let mut backup_providers = toml_edit::Table::new();
                        backup_providers.insert(current_provider_name, current_provider.clone());
                        backup_doc
                            .insert("model_providers", toml_edit::Item::Table(backup_providers));
                    } else {
                        anyhow::bail!("未找到 model_provider '{current_provider_name}' 的配置");
                    }
                } else {
                    anyhow::bail!("配置文件缺少 model_providers 表");
                }

                // 保存当前的 model_provider 选择
                backup_doc.insert("model_provider", toml_edit::value(current_provider_name));

                fs::write(&backup_config, backup_doc.to_string())?;
            }
        }

        Ok(())
    }

    fn backup_gemini(tool: &Tool, profile_name: &str) -> Result<()> {
        let env_path = tool.config_dir.join(".env");
        let backup_env = tool.config_dir.join(format!(".env.{profile_name}"));

        if !env_path.exists() {
            anyhow::bail!("配置文件不存在，无法备份");
        }

        // 读取 .env 文件，只提取 API 相关字段
        let content = fs::read_to_string(&env_path)?;
        let mut api_key = String::new();
        let mut base_url = String::new();
        let mut model = String::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = trimmed.split_once('=') {
                match key.trim() {
                    "GEMINI_API_KEY" => api_key = value.trim().to_string(),
                    "GOOGLE_GEMINI_BASE_URL" => base_url = value.trim().to_string(),
                    "GEMINI_MODEL" => model = value.trim().to_string(),
                    _ => {}
                }
            }
        }

        // 只保存 API 相关字段
        let backup_content = format!(
            "GEMINI_API_KEY={api_key}\nGOOGLE_GEMINI_BASE_URL={base_url}\nGEMINI_MODEL={model}\n"
        );

        fs::write(&backup_env, backup_content)?;

        Ok(())
    }

    /// 列出所有保存的配置
    pub fn list_profiles(tool: &Tool) -> Result<Vec<String>> {
        if !tool.config_dir.exists() {
            return Ok(vec![]);
        }

        let entries = fs::read_dir(&tool.config_dir)?;
        let mut profiles = Vec::new();

        // 时间戳格式正则: YYYYMMDD-HHMMSS
        let timestamp_pattern = regex::Regex::new(r"^\d{8}-\d{6}$").unwrap();

        for entry in entries {
            let entry = entry?;
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();

            match tool.id.as_str() {
                "claude-code" => {
                    // 排除主配置文件本身 (settings.json)
                    if filename_str == tool.config_file {
                        continue;
                    }

                    if filename_str.starts_with("settings.") && filename_str.ends_with(".json") {
                        let profile = filename_str
                            .trim_start_matches("settings.")
                            .trim_end_matches(".json")
                            .to_string();

                        if !profile.is_empty()
                            && !profile.starts_with('.')
                            && !timestamp_pattern.is_match(&profile)
                        {
                            profiles.push(profile);
                        }
                    }
                }
                "codex" => {
                    // 排除主配置文件本身 (config.toml、auth.json)
                    if filename_str == tool.config_file || filename_str == "auth.json" {
                        continue;
                    }

                    let profile = if filename_str.starts_with("config.")
                        && filename_str.ends_with(".toml")
                    {
                        Some(
                            filename_str
                                .trim_start_matches("config.")
                                .trim_end_matches(".toml")
                                .to_string(),
                        )
                    } else if filename_str.starts_with("auth.") && filename_str.ends_with(".json") {
                        Some(
                            filename_str
                                .trim_start_matches("auth.")
                                .trim_end_matches(".json")
                                .to_string(),
                        )
                    } else {
                        None
                    };

                    if let Some(profile) = profile {
                        if !profile.is_empty()
                            && !profile.starts_with('.')
                            && !timestamp_pattern.is_match(&profile)
                        {
                            profiles.push(profile);
                        }
                    }
                }
                "gemini-cli" => {
                    // 排除主配置文件 (.env)
                    if filename_str == tool.config_file {
                        continue;
                    }

                    if filename_str.starts_with(".env.") {
                        let profile = filename_str.trim_start_matches(".env.").to_string();

                        if !profile.is_empty()
                            && !profile.starts_with('.')
                            && !timestamp_pattern.is_match(&profile)
                        {
                            profiles.push(profile);
                        }
                    }
                }
                _ => {}
            }
        }

        profiles.sort();
        profiles.dedup();
        Ok(profiles)
    }

    /// 激活指定的配置
    pub fn activate_profile(tool: &Tool, profile_name: &str) -> Result<()> {
        match tool.id.as_str() {
            "claude-code" => Self::activate_claude(tool, profile_name)?,
            "codex" => Self::activate_codex(tool, profile_name)?,
            "gemini-cli" => Self::activate_gemini(tool, profile_name)?,
            _ => anyhow::bail!("未知工具: {}", tool.id),
        }
        Ok(())
    }

    fn activate_claude(tool: &Tool, profile_name: &str) -> Result<()> {
        let backup_path = tool.backup_path(profile_name);
        let active_path = tool.config_dir.join(&tool.config_file);

        if !backup_path.exists() {
            anyhow::bail!("配置文件不存在: {backup_path:?}");
        }

        // 读取备份的 API 字段（兼容新旧格式）
        let backup_content = fs::read_to_string(&backup_path).context("读取备份配置失败")?;
        let backup_data: Value =
            serde_json::from_str(&backup_content).context("解析备份配置失败")?;

        // 兼容旧格式：先尝试顶层字段（新格式），再尝试 env 下（旧格式）
        let api_key = backup_data
            .get("ANTHROPIC_AUTH_TOKEN")
            .and_then(|v| v.as_str())
            .or_else(|| {
                backup_data
                    .get("env")
                    .and_then(|env| env.get("ANTHROPIC_AUTH_TOKEN"))
                    .and_then(|v| v.as_str())
            })
            .ok_or_else(|| {
                anyhow::anyhow!("备份配置格式错误：缺少 API Key\n\n请重新保存配置以更新格式")
            })?;

        let base_url = backup_data
            .get("ANTHROPIC_BASE_URL")
            .and_then(|v| v.as_str())
            .or_else(|| {
                backup_data
                    .get("env")
                    .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
                    .and_then(|v| v.as_str())
            })
            .ok_or_else(|| {
                anyhow::anyhow!("备份配置格式错误：缺少 Base URL\n\n请重新保存配置以更新格式")
            })?;

        // 读取当前配置（保留其他字段）
        let mut settings = if active_path.exists() {
            let content = fs::read_to_string(&active_path).context("读取当前配置失败")?;
            serde_json::from_str::<Value>(&content).unwrap_or(Value::Object(Map::new()))
        } else {
            Value::Object(Map::new())
        };

        // 只更新 env 中的 API 字段，保留其他配置
        if !settings.is_object() {
            settings = serde_json::json!({});
        }

        let obj = settings.as_object_mut().unwrap();
        if !obj.contains_key("env") {
            obj.insert("env".to_string(), Value::Object(Map::new()));
        }

        let env = obj.get_mut("env").unwrap().as_object_mut().unwrap();
        env.insert(
            "ANTHROPIC_AUTH_TOKEN".to_string(),
            Value::String(api_key.to_string()),
        );
        env.insert(
            "ANTHROPIC_BASE_URL".to_string(),
            Value::String(base_url.to_string()),
        );

        // 写回配置（保留其他字段）
        fs::write(&active_path, serde_json::to_string_pretty(&settings)?)?;

        Ok(())
    }

    fn activate_codex(tool: &Tool, profile_name: &str) -> Result<()> {
        let backup_config = tool.config_dir.join(format!("config.{profile_name}.toml"));
        let backup_auth = tool.config_dir.join(format!("auth.{profile_name}.json"));

        let active_config = tool.config_dir.join("config.toml");
        let active_auth = tool.config_dir.join("auth.json");

        if !backup_auth.exists() {
            anyhow::bail!("配置文件不存在: {backup_auth:?}");
        }

        // 读取备份的 API Key
        let backup_auth_content = fs::read_to_string(&backup_auth)?;
        let backup_auth_data: Value = serde_json::from_str(&backup_auth_content)?;
        let api_key = backup_auth_data
            .get("OPENAI_API_KEY")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("备份配置中缺少 API Key"))?;

        // 增量更新 auth.json（保留其他字段）
        let mut auth_data = if active_auth.exists() {
            let content = fs::read_to_string(&active_auth)?;
            serde_json::from_str::<Value>(&content).unwrap_or(Value::Object(Map::new()))
        } else {
            Value::Object(Map::new())
        };

        if let Value::Object(ref mut auth_obj) = auth_data {
            auth_obj.insert(
                "OPENAI_API_KEY".to_string(),
                Value::String(api_key.to_string()),
            );
        }

        fs::write(&active_auth, serde_json::to_string_pretty(&auth_data)?)?;

        // 读取备份的 config.toml（base_url 和 model_provider）
        if backup_config.exists() {
            let backup_config_content = fs::read_to_string(&backup_config)?;
            let backup_doc = backup_config_content.parse::<toml_edit::DocumentMut>()?;

            // 读取当前 config.toml（保留其他配置）
            let mut active_doc = if active_config.exists() {
                let content = fs::read_to_string(&active_config)?;
                content
                    .parse::<toml_edit::DocumentMut>()
                    .unwrap_or_else(|_| toml_edit::DocumentMut::new())
            } else {
                toml_edit::DocumentMut::new()
            };

            // 只更新 model_providers 中的配置（保留其他字段）
            if let Some(backup_providers) =
                backup_doc.get("model_providers").and_then(|p| p.as_table())
            {
                if !active_doc.contains_key("model_providers") {
                    active_doc["model_providers"] = toml_edit::table();
                }

                // 获取 model_providers 表的可变引用
                if let Some(active_providers) = active_doc
                    .get_mut("model_providers")
                    .and_then(|p| p.as_table_mut())
                {
                    for (key, backup_provider) in backup_providers.iter() {
                        if let Some(backup_provider_table) = backup_provider.as_table() {
                            if backup_provider_table.get("base_url").is_some() {
                                // 如果 provider 不存在，需要创建
                                if !active_providers.contains_key(key) {
                                    // 检查备份文件格式：新格式包含完整字段，旧格式只有 base_url
                                    if is_complete_provider_config(backup_provider_table) {
                                        // 新格式：完整配置，直接复制
                                        active_providers.insert(key, backup_provider.clone());
                                    } else {
                                        // 旧格式：只有 base_url，需要补全必要字段（向后兼容）
                                        let mut new_provider = toml_edit::Table::new();
                                        new_provider.insert("name", toml_edit::value(key));
                                        new_provider.insert(
                                            "base_url",
                                            backup_provider_table.get("base_url").unwrap().clone(),
                                        );
                                        new_provider
                                            .insert("wire_api", toml_edit::value("responses"));
                                        new_provider
                                            .insert("requires_openai_auth", toml_edit::value(true));
                                        active_providers
                                            .insert(key, toml_edit::Item::Table(new_provider));
                                    }
                                } else {
                                    // 如果已存在，只更新 base_url（保留用户自定义配置）
                                    if let Some(active_provider) =
                                        active_providers.get_mut(key).and_then(|p| p.as_table_mut())
                                    {
                                        if let Some(base_url) =
                                            backup_provider_table.get("base_url")
                                        {
                                            active_provider.insert("base_url", base_url.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 更新 model_provider 选择（如果备份中有）
            if let Some(provider) = backup_doc.get("model_provider") {
                active_doc["model_provider"] = provider.clone();
            }

            // 写回 config.toml（保留其他字段和注释）
            fs::write(&active_config, active_doc.to_string())?;
        }

        Ok(())
    }

    fn activate_gemini(tool: &Tool, profile_name: &str) -> Result<()> {
        let backup_env = tool.config_dir.join(format!(".env.{profile_name}"));
        let active_env = tool.config_dir.join(".env");

        if !backup_env.exists() {
            anyhow::bail!("配置文件不存在: {backup_env:?}");
        }

        // 读取备份的 API 字段
        let backup_content = fs::read_to_string(&backup_env)?;
        let mut backup_api_key = String::new();
        let mut backup_base_url = String::new();
        let mut backup_model = String::new();

        for line in backup_content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = trimmed.split_once('=') {
                match key.trim() {
                    "GEMINI_API_KEY" => backup_api_key = value.trim().to_string(),
                    "GOOGLE_GEMINI_BASE_URL" => backup_base_url = value.trim().to_string(),
                    "GEMINI_MODEL" => backup_model = value.trim().to_string(),
                    _ => {}
                }
            }
        }

        // 读取当前 .env（保留其他字段）
        let mut env_vars = HashMap::new();
        if active_env.exists() {
            let content = fs::read_to_string(&active_env)?;
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    if let Some((key, value)) = trimmed.split_once('=') {
                        env_vars.insert(key.trim().to_string(), value.trim().to_string());
                    }
                }
            }
        }

        // 只更新 API 相关字段
        env_vars.insert("GEMINI_API_KEY".to_string(), backup_api_key);
        env_vars.insert("GOOGLE_GEMINI_BASE_URL".to_string(), backup_base_url);
        env_vars.insert("GEMINI_MODEL".to_string(), backup_model);

        // 写回 .env（保留其他字段）
        let env_content: Vec<String> = env_vars.iter().map(|(k, v)| format!("{k}={v}")).collect();
        fs::write(&active_env, env_content.join("\n") + "\n")?;

        Ok(())
    }

    /// 删除配置
    pub fn delete_profile(tool: &Tool, profile_name: &str) -> Result<()> {
        match tool.id.as_str() {
            "claude-code" => {
                let backup_path = tool.backup_path(profile_name);
                if backup_path.exists() {
                    fs::remove_file(backup_path)?;
                }
            }
            "codex" => {
                let backup_config = tool.config_dir.join(format!("config.{profile_name}.toml"));
                let backup_auth = tool.config_dir.join(format!("auth.{profile_name}.json"));

                if backup_config.exists() {
                    fs::remove_file(backup_config)?;
                }
                if backup_auth.exists() {
                    fs::remove_file(backup_auth)?;
                }
            }
            "gemini-cli" => {
                let backup_env = tool.config_dir.join(format!(".env.{profile_name}"));

                if backup_env.exists() {
                    fs::remove_file(backup_env)?;
                }
                // 注意：不再删除 settings.json 备份，因为新版本不再备份它
            }
            _ => anyhow::bail!("未知工具: {}", tool.id),
        }

        Ok(())
    }

    /// 读取 Claude Code 完整配置
    pub fn read_claude_settings() -> Result<Value> {
        let tool = Tool::claude_code();
        let config_path = tool.config_dir.join(&tool.config_file);

        if !config_path.exists() {
            return Ok(Value::Object(Map::new()));
        }

        let content = fs::read_to_string(&config_path).context("读取 Claude Code 配置失败")?;
        if content.trim().is_empty() {
            return Ok(Value::Object(Map::new()));
        }

        let settings: Value = serde_json::from_str(&content)
            .map_err(|err| anyhow!("解析 Claude Code 配置失败: {err}"))?;

        Ok(settings)
    }

    /// 保存 Claude Code 完整配置
    pub fn save_claude_settings(settings: &Value) -> Result<()> {
        if !settings.is_object() {
            anyhow::bail!("Claude Code 配置必须是 JSON 对象");
        }

        let tool = Tool::claude_code();
        let config_dir = &tool.config_dir;
        let config_path = config_dir.join(&tool.config_file);

        fs::create_dir_all(config_dir).context("创建 Claude Code 配置目录失败")?;
        let json = serde_json::to_string_pretty(settings)?;
        fs::write(&config_path, json).context("写入 Claude Code 配置失败")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&config_path)?;
            let mut perms = metadata.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&config_path, perms)?;
        }

        Ok(())
    }

    /// 获取内置的 Claude Code JSON Schema
    pub fn get_claude_schema() -> Result<Value> {
        static CLAUDE_SCHEMA: OnceCell<Value> = OnceCell::new();

        let schema = CLAUDE_SCHEMA.get_or_try_init(|| {
            let raw = include_str!("../../resources/claude_code_settings.schema.json");
            serde_json::from_str(raw).context("解析 Claude Code Schema 失败")
        })?;

        Ok(schema.clone())
    }

    /// 读取 Codex config.toml 和 auth.json
    pub fn read_codex_settings() -> Result<CodexSettingsPayload> {
        let tool = Tool::codex();
        let config_path = tool.config_dir.join(&tool.config_file);
        let auth_path = tool.config_dir.join("auth.json");

        let config_value = if config_path.exists() {
            let content =
                fs::read_to_string(&config_path).context("读取 Codex config.toml 失败")?;
            let toml_value: toml::Value = toml::from_str(&content)
                .map_err(|err| anyhow!("解析 Codex config.toml 失败: {err}"))?;
            serde_json::to_value(toml_value).context("转换 Codex config.toml 为 JSON 失败")?
        } else {
            Value::Object(Map::new())
        };

        let auth_token = if auth_path.exists() {
            let content = fs::read_to_string(&auth_path).context("读取 Codex auth.json 失败")?;
            let auth: Value = serde_json::from_str(&content)
                .map_err(|err| anyhow!("解析 Codex auth.json 失败: {err}"))?;
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

    /// 保存 Codex 配置和 auth.json
    pub fn save_codex_settings(config: &Value, auth_token: Option<String>) -> Result<()> {
        if !config.is_object() {
            anyhow::bail!("Codex 配置必须是对象结构");
        }

        let tool = Tool::codex();
        let config_path = tool.config_dir.join(&tool.config_file);
        let auth_path = tool.config_dir.join("auth.json");
        fs::create_dir_all(&tool.config_dir).context("创建 Codex 配置目录失败")?;

        let mut existing_doc = if config_path.exists() {
            let content =
                fs::read_to_string(&config_path).context("读取 Codex config.toml 失败")?;
            content
                .parse::<DocumentMut>()
                .map_err(|err| anyhow!("解析 Codex config.toml 失败: {err}"))?
        } else {
            DocumentMut::new()
        };

        let new_toml_string = toml::to_string(config).context("序列化 Codex config 失败")?;
        let new_doc = new_toml_string
            .parse::<DocumentMut>()
            .map_err(|err| anyhow!("解析待写入 Codex 配置失败: {err}"))?;

        merge_toml_tables(existing_doc.as_table_mut(), new_doc.as_table());

        fs::write(&config_path, existing_doc.to_string()).context("写入 Codex config.toml 失败")?;

        if let Some(token) = auth_token {
            let mut auth_data = if auth_path.exists() {
                let content = fs::read_to_string(&auth_path).unwrap_or_default();
                serde_json::from_str::<Value>(&content).unwrap_or(Value::Object(Map::new()))
            } else {
                Value::Object(Map::new())
            };

            if let Value::Object(ref mut obj) = auth_data {
                obj.insert("OPENAI_API_KEY".to_string(), Value::String(token));
            }

            fs::write(&auth_path, serde_json::to_string_pretty(&auth_data)?)
                .context("写入 Codex auth.json 失败")?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let metadata = fs::metadata(&auth_path)?;
                let mut perms = metadata.permissions();
                perms.set_mode(0o600);
                fs::set_permissions(&auth_path, perms)?;
            }
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&config_path)?;
            let mut perms = metadata.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&config_path, perms)?;
        }

        Ok(())
    }

    /// 获取 Codex config schema
    pub fn get_codex_schema() -> Result<Value> {
        static CODEX_SCHEMA: OnceCell<Value> = OnceCell::new();
        let schema = CODEX_SCHEMA.get_or_try_init(|| {
            let raw = include_str!("../../resources/codex_config.schema.json");
            serde_json::from_str(raw).context("解析 Codex Schema 失败")
        })?;

        Ok(schema.clone())
    }

    /// 读取 Gemini CLI 配置与 .env
    pub fn read_gemini_settings() -> Result<GeminiSettingsPayload> {
        let tool = Tool::gemini_cli();
        let settings_path = tool.config_dir.join(&tool.config_file);
        let env_path = tool.config_dir.join(".env");

        let settings = if settings_path.exists() {
            let content = fs::read_to_string(&settings_path).context("读取 Gemini CLI 配置失败")?;
            if content.trim().is_empty() {
                Value::Object(Map::new())
            } else {
                serde_json::from_str(&content)
                    .map_err(|err| anyhow!("解析 Gemini CLI 配置失败: {err}"))?
            }
        } else {
            Value::Object(Map::new())
        };

        let env = Self::read_gemini_env(&env_path)?;

        Ok(GeminiSettingsPayload { settings, env })
    }

    /// 保存 Gemini CLI 配置与 .env
    pub fn save_gemini_settings(settings: &Value, env: &GeminiEnvPayload) -> Result<()> {
        if !settings.is_object() {
            anyhow::bail!("Gemini CLI 配置必须是 JSON 对象");
        }

        let tool = Tool::gemini_cli();
        let config_dir = &tool.config_dir;
        let settings_path = config_dir.join(&tool.config_file);
        let env_path = config_dir.join(".env");
        fs::create_dir_all(config_dir).context("创建 Gemini CLI 配置目录失败")?;

        let json = serde_json::to_string_pretty(settings)?;
        fs::write(&settings_path, json).context("写入 Gemini CLI 配置失败")?;

        let mut env_pairs = Self::read_env_pairs(&env_path)?;
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
        Self::write_env_pairs(&env_path, &env_pairs).context("写入 Gemini CLI .env 失败")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for path in [&settings_path, &env_path] {
                if path.exists() {
                    let metadata = fs::metadata(path)?;
                    let mut perms = metadata.permissions();
                    perms.set_mode(0o600);
                    fs::set_permissions(path, perms)?;
                }
            }
        }

        Ok(())
    }

    /// 获取 Gemini CLI JSON Schema
    pub fn get_gemini_schema() -> Result<Value> {
        static GEMINI_SCHEMA: OnceCell<Value> = OnceCell::new();
        let schema = GEMINI_SCHEMA.get_or_try_init(|| {
            let raw = include_str!("../../resources/gemini_cli_settings.schema.json");
            serde_json::from_str(raw).context("解析 Gemini CLI Schema 失败")
        })?;

        Ok(schema.clone())
    }

    fn read_gemini_env(path: &Path) -> Result<GeminiEnvPayload> {
        if !path.exists() {
            return Ok(GeminiEnvPayload {
                model: "gemini-2.5-pro".to_string(),
                ..GeminiEnvPayload::default()
            });
        }

        let env_pairs = Self::read_env_pairs(path)?;
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

    fn read_env_pairs(path: &Path) -> Result<HashMap<String, String>> {
        let mut pairs = HashMap::new();
        if path.exists() {
            let content = fs::read_to_string(path)?;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = trimmed.split_once('=') {
                    pairs.insert(key.trim().to_string(), value.trim().to_string());
                }
            }
        }
        Ok(pairs)
    }

    fn write_env_pairs(path: &Path, pairs: &HashMap<String, String>) -> Result<()> {
        let mut items: Vec<_> = pairs.iter().collect();
        items.sort_by(|a, b| a.0.cmp(b.0));
        let mut content = String::new();
        for (idx, (key, value)) in items.iter().enumerate() {
            if idx > 0 {
                content.push('\n');
            }
            content.push_str(key);
            content.push('=');
            content.push_str(value);
        }
        content.push('\n');
        fs::write(path, content)?;
        Ok(())
    }
}
