//! Profile v2.0 迁移
//!
//! 从两个旧系统迁移到新的双文件 JSON 系统：
//! 1. 工具原始配置目录（~/.claude、~/.codex、~/.gemini-cli）
//! 2. DuckCoding 旧多目录系统（~/.duckcoding/profiles/、active/、metadata/）
//!
//! 目标：profiles.json + active.json

use crate::data::DataManager;
use crate::services::migration_manager::migration_trait::{Migration, MigrationResult};
use crate::services::profile_manager::{
    ActiveProfile, ActiveStore, ClaudeProfile, CodexProfile, GeminiProfile, ProfilesStore,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

// ==================== 迁移专用类型定义 ====================

/// Profile 格式（迁移专用）
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProfileFormat {
    Json,
    Toml,
    Env,
    #[default]
    Unknown,
}

/// Profile 描述符（迁移专用）
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProfileDescriptor {
    pub tool_id: String,
    pub name: String,
    #[serde(default)]
    pub format: ProfileFormat,
    pub path: PathBuf,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
}

// ==================== 迁移专用路径函数 ====================

/// 返回旧的 profiles 目录路径（迁移专用）
fn profiles_root() -> Result<PathBuf> {
    let Some(home_dir) = dirs::home_dir() else {
        anyhow::bail!("无法获取用户主目录");
    };
    let profiles = home_dir.join(".duckcoding/profiles");
    Ok(profiles)
}

pub struct ProfileV2Migration;

impl Default for ProfileV2Migration {
    fn default() -> Self {
        Self::new()
    }
}

impl ProfileV2Migration {
    pub fn new() -> Self {
        Self
    }

    /// 检查是否存在旧数据（任一来源）
    fn has_old_data(&self) -> bool {
        // 检查 profiles/ 目录
        if let Ok(profiles_dir) = profiles_root() {
            if profiles_dir.exists()
                && profiles_dir
                    .read_dir()
                    .map(|entries| entries.count() > 0)
                    .unwrap_or(false)
            {
                return true;
            }
        }

        // 检查原始工具配置
        self.has_original_tool_configs()
    }

    /// 检查是否存在原始工具配置文件
    fn has_original_tool_configs(&self) -> bool {
        let Some(home_dir) = dirs::home_dir() else {
            return false;
        };

        // Claude Code: ~/.claude/settings.*.json
        if let Ok(entries) = fs::read_dir(home_dir.join(".claude")) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("settings.")
                    && name.ends_with(".json")
                    && name != "settings.json"
                {
                    return true;
                }
            }
        }

        // Codex: ~/.codex/config.*.toml
        if let Ok(entries) = fs::read_dir(home_dir.join(".codex")) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("config.") && name.ends_with(".toml") {
                    return true;
                }
            }
        }

        // Gemini CLI: ~/.gemini-cli/.env.*
        if let Ok(entries) = fs::read_dir(home_dir.join(".gemini-cli")) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(".env.") && name.len() > 5 {
                    return true;
                }
            }
        }

        false
    }

    /// 从原始工具配置迁移
    #[allow(clippy::type_complexity)]
    fn migrate_from_original_configs(
        &self,
    ) -> Result<(
        HashMap<String, ClaudeProfile>,
        HashMap<String, CodexProfile>,
        HashMap<String, GeminiProfile>,
    )> {
        let claude = self.migrate_claude_original().unwrap_or_default();
        let codex = self.migrate_codex_original().unwrap_or_default();
        let gemini = self.migrate_gemini_original().unwrap_or_default();

        Ok((claude, codex, gemini))
    }

    /// 迁移 Claude Code 原始配置（settings.{profile}.json）
    fn migrate_claude_original(&self) -> Result<HashMap<String, ClaudeProfile>> {
        let Some(home_dir) = dirs::home_dir() else {
            return Ok(HashMap::new());
        };
        let claude_dir = home_dir.join(".claude");

        if !claude_dir.exists() {
            return Ok(HashMap::new());
        }

        let mut profiles = HashMap::new();
        let manager = DataManager::new();

        for entry in fs::read_dir(&claude_dir).context("读取 .claude 目录失败")? {
            let entry = entry.context("读取目录项失败")?;
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };

            // 只处理 settings.{profile}.json，排除 settings.json
            if name == "settings.json" || !name.starts_with("settings.") || !name.ends_with(".json")
            {
                continue;
            }

            let profile_name = name
                .trim_start_matches("settings.")
                .trim_end_matches(".json")
                .to_string();

            if profile_name.is_empty() || profile_name.starts_with('.') {
                continue;
            }

            if let Ok(settings_value) = manager.json_uncached().read(&path) {
                let api_key = settings_value
                    .get("ANTHROPIC_AUTH_TOKEN")
                    .and_then(|v| v.as_str())
                    .or_else(|| {
                        settings_value
                            .get("env")
                            .and_then(|env| env.get("ANTHROPIC_AUTH_TOKEN"))
                            .and_then(|v| v.as_str())
                    })
                    .unwrap_or("")
                    .to_string();

                let base_url = settings_value
                    .get("ANTHROPIC_BASE_URL")
                    .and_then(|v| v.as_str())
                    .or_else(|| {
                        settings_value
                            .get("env")
                            .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
                            .and_then(|v| v.as_str())
                    })
                    .unwrap_or("")
                    .to_string();

                if !api_key.is_empty() {
                    let profile = ClaudeProfile {
                        api_key,
                        base_url,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                        raw_settings: Some(settings_value),
                        raw_config_json: None,
                    };
                    profiles.insert(profile_name.clone(), profile);
                    tracing::info!("已从原始 Claude Code 配置迁移 Profile: {}", profile_name);
                }
            }
        }

        Ok(profiles)
    }

    /// 迁移 Codex 原始配置（config.{profile}.toml + auth.{profile}.json）
    fn migrate_codex_original(&self) -> Result<HashMap<String, CodexProfile>> {
        let Some(home_dir) = dirs::home_dir() else {
            return Ok(HashMap::new());
        };
        let codex_dir = home_dir.join(".codex");

        if !codex_dir.exists() {
            return Ok(HashMap::new());
        }

        let mut profiles = HashMap::new();

        for entry in fs::read_dir(&codex_dir).context("读取 .codex 目录失败")? {
            let entry = entry.context("读取目录项失败")?;
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };

            // 只处理 config.{profile}.toml
            if !name.starts_with("config.") || !name.ends_with(".toml") {
                continue;
            }

            let profile_name = name
                .trim_start_matches("config.")
                .trim_end_matches(".toml")
                .to_string();

            if profile_name.is_empty() || profile_name.starts_with('.') {
                continue;
            }

            // 必须有配对的 auth.{profile}.json
            let auth_path = codex_dir.join(format!("auth.{}.json", profile_name));
            if !auth_path.exists() {
                continue;
            }

            // 读取 auth.json 获取 API Key
            let auth_content = fs::read_to_string(&auth_path).unwrap_or_default();
            let auth_data: serde_json::Value = serde_json::from_str(&auth_content)
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
            let api_key = auth_data
                .get("OPENAI_API_KEY")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // 读取 config.toml 获取 provider 和 base_url
            let mut base_url = String::new();
            let mut provider = None;
            let raw_config_toml = fs::read_to_string(&path).ok();

            if let Some(ref content) = raw_config_toml {
                if let Ok(toml::Value::Table(table)) = toml::from_str::<toml::Value>(content) {
                    provider = table
                        .get("model_provider")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    if let Some(toml::Value::Table(providers)) = table.get("model_providers") {
                        if let Some(provider_name) = provider
                            .clone()
                            .or_else(|| providers.keys().next().cloned())
                        {
                            if let Some(toml::Value::Table(provider_table)) =
                                providers.get(&provider_name)
                            {
                                if let Some(toml::Value::String(url)) =
                                    provider_table.get("base_url")
                                {
                                    base_url = url.clone();
                                }
                            }
                        }
                    }
                }
            }

            if base_url.is_empty() {
                base_url = "https://jp.duckcoding.com/v1".to_string();
            }

            if !api_key.is_empty() {
                let profile = CodexProfile {
                    api_key,
                    base_url,
                    wire_api: provider.unwrap_or_else(|| "responses".to_string()),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    raw_config_toml,
                    raw_auth_json: Some(auth_data),
                };
                profiles.insert(profile_name.clone(), profile);
                tracing::info!("已从原始 Codex 配置迁移 Profile: {}", profile_name);
            }
        }

        Ok(profiles)
    }

    /// 迁移 Gemini CLI 原始配置（.env.{profile}）
    fn migrate_gemini_original(&self) -> Result<HashMap<String, GeminiProfile>> {
        let Some(home_dir) = dirs::home_dir() else {
            return Ok(HashMap::new());
        };
        let gemini_dir = home_dir.join(".gemini-cli");

        if !gemini_dir.exists() {
            return Ok(HashMap::new());
        }

        let mut profiles = HashMap::new();

        for entry in fs::read_dir(&gemini_dir).context("读取 .gemini-cli 目录失败")? {
            let entry = entry.context("读取目录项失败")?;
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };

            // 只处理 .env.{profile}
            if !name.starts_with(".env.") || name.len() <= 5 {
                continue;
            }

            let profile_name = name.trim_start_matches(".env.").to_string();
            if profile_name.is_empty() || profile_name.starts_with('.') {
                continue;
            }

            let mut api_key = String::new();
            let mut base_url = String::new();
            let mut model: Option<String> = None; // 默认为空，只有文件中有值才设置
            let raw_env = fs::read_to_string(&path).ok();

            if let Some(ref content) = raw_env {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    if let Some((key, value)) = trimmed.split_once('=') {
                        match key.trim() {
                            "GEMINI_API_KEY" => api_key = value.trim().to_string(),
                            "GOOGLE_GEMINI_BASE_URL" => base_url = value.trim().to_string(),
                            "GEMINI_MODEL" => model = Some(value.trim().to_string()),
                            _ => {}
                        }
                    }
                }
            }

            if base_url.is_empty() {
                base_url = "https://generativelanguage.googleapis.com".to_string();
            }

            if !api_key.is_empty() {
                let profile = GeminiProfile {
                    api_key,
                    base_url,
                    model, // 保留从文件读取的值（可能是 None）
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    raw_settings: None,
                    raw_env,
                };
                profiles.insert(profile_name.clone(), profile);
                tracing::info!("已从原始 Gemini CLI 配置迁移 Profile: {}", profile_name);
            }
        }

        Ok(profiles)
    }

    /// 读取旧的 metadata/index.json
    fn read_old_index(&self) -> Result<OldProfileIndex> {
        let Some(home_dir) = dirs::home_dir() else {
            return Ok(OldProfileIndex::default());
        };
        let metadata_path = home_dir.join(".duckcoding/metadata/index.json");

        if !metadata_path.exists() {
            return Ok(OldProfileIndex::default());
        }

        let manager = DataManager::new();
        let value = manager
            .json()
            .read(&metadata_path)
            .context("读取旧 Profile Index 失败")?;
        serde_json::from_value(value).context("解析旧 Profile Index 失败")
    }

    /// 读取旧的 active/{tool}.json
    fn read_old_active_state(&self, tool_id: &str) -> Result<Option<ActiveProfile>> {
        let Some(home_dir) = dirs::home_dir() else {
            return Ok(None);
        };
        let active_path = home_dir.join(format!(".duckcoding/active/{}.json", tool_id));

        if !active_path.exists() {
            return Ok(None);
        }

        let manager = DataManager::new();
        let value = manager
            .json()
            .read(&active_path)
            .with_context(|| format!("读取旧激活状态失败: {:?}", active_path))?;

        let old_state: OldActiveState =
            serde_json::from_value(value).context("解析旧激活状态失败")?;

        Ok(old_state.profile_name.map(|name| ActiveProfile {
            profile: name,
            switched_at: old_state.last_synced_at.unwrap_or_else(Utc::now),
            native_checksum: old_state.native_checksum,
            dirty: old_state.dirty,
        }))
    }

    /// 从旧 profiles/ 目录读取单个 Profile
    fn read_old_profile(
        &self,
        descriptor: &ProfileDescriptor,
    ) -> Result<(ClaudeProfile, CodexProfile, GeminiProfile)> {
        let manager = DataManager::new();

        match descriptor.format {
            ProfileFormat::Json => {
                let value = manager
                    .json_uncached()
                    .read(&descriptor.path)
                    .with_context(|| format!("读取 Profile 文件失败: {:?}", descriptor.path))?;

                // 根据 tool_id 解析到对应类型
                match descriptor.tool_id.as_str() {
                    "claude-code" => {
                        let api_key = value
                            .get("api_key")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let base_url = value
                            .get("base_url")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let raw_settings = value.get("raw_settings").cloned();
                        let raw_config_json = value.get("raw_config_json").cloned();

                        Ok((
                            ClaudeProfile {
                                api_key,
                                base_url,
                                created_at: descriptor.created_at.unwrap_or_else(Utc::now),
                                updated_at: descriptor.updated_at.unwrap_or_else(Utc::now),
                                raw_settings,
                                raw_config_json,
                            },
                            CodexProfile::default_placeholder(),
                            GeminiProfile::default_placeholder(),
                        ))
                    }
                    "codex" => {
                        let api_key = value
                            .get("api_key")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let base_url = value
                            .get("base_url")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let provider = value
                            .get("provider")
                            .and_then(|v| v.as_str())
                            .unwrap_or("responses")
                            .to_string();
                        let raw_config_toml = value
                            .get("raw_config_toml")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let raw_auth_json = value.get("raw_auth_json").cloned();

                        Ok((
                            ClaudeProfile::default_placeholder(),
                            CodexProfile {
                                api_key,
                                base_url,
                                wire_api: provider,
                                created_at: descriptor.created_at.unwrap_or_else(Utc::now),
                                updated_at: descriptor.updated_at.unwrap_or_else(Utc::now),
                                raw_config_toml,
                                raw_auth_json,
                            },
                            GeminiProfile::default_placeholder(),
                        ))
                    }
                    "gemini-cli" => {
                        let api_key = value
                            .get("api_key")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let base_url = value
                            .get("base_url")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let model = value
                            .get("model")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()); // 只有存在时才设置，否则为 None
                        let raw_settings = value.get("raw_settings").cloned();
                        let raw_env = value
                            .get("raw_env")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        Ok((
                            ClaudeProfile::default_placeholder(),
                            CodexProfile::default_placeholder(),
                            GeminiProfile {
                                api_key,
                                base_url,
                                model, // 直接使用 Option<String>
                                created_at: descriptor.created_at.unwrap_or_else(Utc::now),
                                updated_at: descriptor.updated_at.unwrap_or_else(Utc::now),
                                raw_settings,
                                raw_env,
                            },
                        ))
                    }
                    _ => anyhow::bail!("未知的工具 ID: {}", descriptor.tool_id),
                }
            }
            _ => anyhow::bail!("不支持的格式: {:?}", descriptor.format),
        }
    }

    /// 执行完整迁移（合并两个来源）
    fn migrate_profiles(&self) -> Result<(usize, ProfilesStore, ActiveStore)> {
        let mut profiles_store = ProfilesStore::new();
        let mut active_store = ActiveStore::new();
        let mut migrated_count = 0;

        // 第一步：从原始工具配置迁移
        let (claude_profiles, codex_profiles, gemini_profiles) =
            self.migrate_from_original_configs()?;

        migrated_count += claude_profiles.len() + codex_profiles.len() + gemini_profiles.len();
        profiles_store.claude_code.extend(claude_profiles);
        profiles_store.codex.extend(codex_profiles);
        profiles_store.gemini_cli.extend(gemini_profiles);

        // 第二步：从 profiles/ 目录迁移（补充未迁移的）
        let old_index = self.read_old_index()?;
        for (tool_id, descriptors) in old_index.entries {
            for descriptor in descriptors {
                // 根据工具类型处理
                match tool_id.as_str() {
                    "claude-code" => {
                        if !profiles_store.claude_code.contains_key(&descriptor.name) {
                            if let Ok((claude, _, _)) = self.read_old_profile(&descriptor) {
                                profiles_store
                                    .claude_code
                                    .insert(descriptor.name.clone(), claude);
                                migrated_count += 1;
                                tracing::debug!(
                                    "已从 profiles/ 迁移 Claude Profile: {}",
                                    descriptor.name
                                );
                            }
                        }
                    }
                    "codex" => {
                        if !profiles_store.codex.contains_key(&descriptor.name) {
                            if let Ok((_, codex, _)) = self.read_old_profile(&descriptor) {
                                profiles_store.codex.insert(descriptor.name.clone(), codex);
                                migrated_count += 1;
                                tracing::debug!(
                                    "已从 profiles/ 迁移 Codex Profile: {}",
                                    descriptor.name
                                );
                            }
                        }
                    }
                    "gemini-cli" => {
                        if !profiles_store.gemini_cli.contains_key(&descriptor.name) {
                            if let Ok((_, _, gemini)) = self.read_old_profile(&descriptor) {
                                profiles_store
                                    .gemini_cli
                                    .insert(descriptor.name.clone(), gemini);
                                migrated_count += 1;
                                tracing::debug!(
                                    "已从 profiles/ 迁移 Gemini Profile: {}",
                                    descriptor.name
                                );
                            }
                        }
                    }
                    _ => {
                        tracing::warn!("未知的工具 ID: {}", tool_id);
                    }
                }
            }

            // 读取旧的激活状态
            if let Ok(Some(active_profile)) = self.read_old_active_state(&tool_id) {
                match tool_id.as_str() {
                    "claude-code" => active_store.claude_code = Some(active_profile.clone()),
                    "codex" => active_store.codex = Some(active_profile.clone()),
                    "gemini-cli" => active_store.gemini_cli = Some(active_profile.clone()),
                    _ => {}
                }
                tracing::debug!("已迁移激活状态: {} -> {}", tool_id, active_profile.profile);
            }
        }

        Ok((migrated_count, profiles_store, active_store))
    }

    /// 保存新数据到 profiles.json + active.json
    fn save_new_data(&self, profiles: &ProfilesStore, active: &ActiveStore) -> Result<()> {
        let Some(home_dir) = dirs::home_dir() else {
            anyhow::bail!("无法获取用户主目录");
        };
        let duckcoding_dir = home_dir.join(".duckcoding");
        fs::create_dir_all(&duckcoding_dir)
            .with_context(|| format!("创建 .duckcoding 目录失败: {:?}", duckcoding_dir))?;

        let manager = DataManager::new();

        // 保存 profiles.json
        let profiles_path = duckcoding_dir.join("profiles.json");
        let profiles_value = serde_json::to_value(profiles).context("序列化 ProfilesStore 失败")?;
        manager
            .json()
            .write(&profiles_path, &profiles_value)
            .with_context(|| format!("写入 profiles.json 失败: {:?}", profiles_path))?;

        // 保存 active.json
        let active_path = duckcoding_dir.join("active.json");
        let active_value = serde_json::to_value(active).context("序列化 ActiveStore 失败")?;
        manager
            .json()
            .write(&active_path, &active_value)
            .with_context(|| format!("写入 active.json 失败: {:?}", active_path))?;

        Ok(())
    }

    /// 清理旧目录（备份后删除）
    fn cleanup_legacy_directories(&self) -> Result<()> {
        let Some(home_dir) = dirs::home_dir() else {
            anyhow::bail!("无法获取用户主目录");
        };
        let duckcoding_dir = home_dir.join(".duckcoding");
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_dir = duckcoding_dir.join(format!("backup_profile_v1_{}", timestamp));

        // 备份并删除 profiles/ 目录
        let old_profiles = duckcoding_dir.join("profiles");
        if old_profiles.exists() {
            let backup_profiles = backup_dir.join("profiles");
            fs::create_dir_all(&backup_profiles).context("创建备份目录失败")?;
            copy_dir_all(&old_profiles, &backup_profiles).context("备份 profiles 目录失败")?;
            fs::remove_dir_all(&old_profiles).context("删除旧 profiles 目录失败")?;
            tracing::info!("已备份并删除旧 profiles 目录: {:?}", old_profiles);
        }

        // 备份并删除 active/ 目录
        let old_active = duckcoding_dir.join("active");
        if old_active.exists() {
            let backup_active = backup_dir.join("active");
            fs::create_dir_all(&backup_active).context("创建备份目录失败")?;
            copy_dir_all(&old_active, &backup_active).context("备份 active 目录失败")?;
            fs::remove_dir_all(&old_active).context("删除旧 active 目录失败")?;
            tracing::info!("已备份并删除旧 active 目录: {:?}", old_active);
        }

        // 备份并删除 metadata/ 目录
        let old_metadata = duckcoding_dir.join("metadata");
        if old_metadata.exists() {
            let backup_metadata = backup_dir.join("metadata");
            fs::create_dir_all(&backup_metadata).context("创建备份目录失败")?;
            copy_dir_all(&old_metadata, &backup_metadata).context("备份 metadata 目录失败")?;
            fs::remove_dir_all(&old_metadata).context("删除旧 metadata 目录失败")?;
            tracing::info!("已备份并删除旧 metadata 目录: {:?}", old_metadata);
        }

        if backup_dir.exists() {
            tracing::info!("旧配置文件已备份到: {:?}", backup_dir);
        }

        Ok(())
    }
}

#[async_trait]
impl Migration for ProfileV2Migration {
    fn id(&self) -> &str {
        "profile_v2_migration"
    }

    fn name(&self) -> &str {
        "Profile v2.0 迁移（双文件系统）"
    }

    fn target_version(&self) -> &str {
        "1.4.0"
    }

    async fn execute(&self) -> Result<MigrationResult> {
        let start_time = Instant::now();
        tracing::info!("开始执行 Profile v2.0 迁移");

        // 检查是否有旧数据
        if !self.has_old_data() {
            tracing::info!("未检测到旧的 Profile 数据，跳过迁移");
            return Ok(MigrationResult {
                migration_id: self.id().to_string(),
                success: true,
                message: "未检测到旧数据，无需迁移".to_string(),
                records_migrated: 0,
                duration_secs: start_time.elapsed().as_secs_f64(),
            });
        }

        // 检查新数据是否已存在
        let Some(home_dir) = dirs::home_dir() else {
            anyhow::bail!("无法获取用户主目录");
        };
        let new_profiles_path = home_dir.join(".duckcoding/profiles.json");

        if new_profiles_path.exists() {
            tracing::warn!("检测到 profiles.json 已存在，跳过迁移");
            return Ok(MigrationResult {
                migration_id: self.id().to_string(),
                success: true,
                message: "新数据文件已存在，跳过迁移".to_string(),
                records_migrated: 0,
                duration_secs: start_time.elapsed().as_secs_f64(),
            });
        }

        // 执行迁移
        let (count, profiles_store, active_store) =
            self.migrate_profiles().context("迁移 Profile 数据失败")?;

        // 保存新数据
        self.save_new_data(&profiles_store, &active_store)
            .context("保存新 Profile 数据失败")?;

        // 清理旧数据
        self.cleanup_legacy_directories()
            .context("清理旧数据失败")?;

        let duration = start_time.elapsed().as_secs_f64();
        tracing::info!(
            "Profile v2.0 迁移完成：共迁移 {} 个 Profile，耗时 {:.2}s",
            count,
            duration
        );

        Ok(MigrationResult {
            migration_id: self.id().to_string(),
            success: true,
            message: format!(
                "成功迁移 {} 个 Profile 到新系统（profiles.json + active.json）",
                count
            ),
            records_migrated: count,
            duration_secs: duration,
        })
    }
}

// ==================== 辅助类型 ====================

/// 旧版 metadata/index.json 结构
#[derive(Debug, Clone, Deserialize, Default)]
struct OldProfileIndex {
    #[serde(default)]
    entries: HashMap<String, Vec<ProfileDescriptor>>,
}

/// 旧版 active/{tool}.json 结构
#[derive(Debug, Clone, Deserialize)]
struct OldActiveState {
    profile_name: Option<String>,
    native_checksum: Option<String>,
    last_synced_at: Option<DateTime<Utc>>,
    #[serde(default)]
    dirty: bool,
}

// ==================== 辅助函数 ====================

/// 递归复制目录
fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> Result<()> {
    fs::create_dir_all(dst).context("创建目标目录失败")?;
    for entry in fs::read_dir(src).context("读取源目录失败")? {
        let entry = entry.context("读取目录项失败")?;
        let ty = entry.file_type().context("获取文件类型失败")?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name())).context("复制文件失败")?;
        }
    }
    Ok(())
}

// ==================== Placeholder 实现 ====================

impl ClaudeProfile {
    fn default_placeholder() -> Self {
        Self {
            api_key: String::new(),
            base_url: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            raw_settings: None,
            raw_config_json: None,
        }
    }
}

impl CodexProfile {
    fn default_placeholder() -> Self {
        Self {
            api_key: String::new(),
            base_url: String::new(),
            wire_api: "responses".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            raw_config_toml: None,
            raw_auth_json: None,
        }
    }
}

impl GeminiProfile {
    fn default_placeholder() -> Self {
        Self {
            api_key: String::new(),
            base_url: String::new(),
            model: None, // 默认不设置 model
            created_at: Utc::now(),
            updated_at: Utc::now(),
            raw_settings: None,
            raw_env: None,
        }
    }
}
