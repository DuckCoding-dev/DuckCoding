//! ProfileManager 核心实现（v2.1 - 简化版）

use super::types::*;
use crate::data::DataManager;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use std::path::PathBuf;

/// 系统保留的 Profile 名称前缀
const RESERVED_PREFIX: &str = "dc_proxy_";

/// 校验 Profile 名称是否使用保留前缀
fn validate_profile_name(name: &str) -> Result<()> {
    if name.starts_with(RESERVED_PREFIX) {
        return Err(anyhow!(
            "Profile 名称不能以 '{}' 开头（系统保留前缀）",
            RESERVED_PREFIX
        ));
    }
    Ok(())
}

pub struct ProfileManager {
    data_manager: DataManager,
    profiles_path: PathBuf,
    active_path: PathBuf,
}

impl ProfileManager {
    pub fn new() -> Result<Self> {
        let home_dir = dirs::home_dir().ok_or_else(|| anyhow!("无法获取用户主目录"))?;
        let duckcoding_dir = home_dir.join(".duckcoding");
        std::fs::create_dir_all(&duckcoding_dir)?;

        Ok(Self {
            data_manager: DataManager::new(),
            profiles_path: duckcoding_dir.join("profiles.json"),
            active_path: duckcoding_dir.join("active.json"),
        })
    }

    fn load_profiles_store(&self) -> Result<ProfilesStore> {
        if !self.profiles_path.exists() {
            return Ok(ProfilesStore::new());
        }
        let value = self.data_manager.json().read(&self.profiles_path)?;
        serde_json::from_value(value).context("反序列化 ProfilesStore 失败")
    }

    fn save_profiles_store(&self, store: &ProfilesStore) -> Result<()> {
        let value = serde_json::to_value(store)?;
        self.data_manager
            .json()
            .write(&self.profiles_path, &value)
            .map_err(Into::into)
    }

    pub fn load_active_store(&self) -> Result<ActiveStore> {
        if !self.active_path.exists() {
            return Ok(ActiveStore::new());
        }
        let value = self.data_manager.json().read(&self.active_path)?;
        serde_json::from_value(value).context("反序列化 ActiveStore 失败")
    }

    pub fn save_active_store(&self, store: &ActiveStore) -> Result<()> {
        let value = serde_json::to_value(store)?;
        self.data_manager
            .json()
            .write(&self.active_path, &value)
            .map_err(Into::into)
    }

    // ==================== Claude Code ====================

    pub fn save_claude_profile(&self, name: &str, api_key: String, base_url: String) -> Result<()> {
        // 保留字校验
        validate_profile_name(name)?;

        let mut store = self.load_profiles_store()?;

        let profile = if let Some(existing) = store.claude_code.get_mut(name) {
            // 更新模式：只更新非空字段
            if !api_key.is_empty() {
                existing.api_key = api_key;
            }
            if !base_url.is_empty() {
                existing.base_url = base_url;
            }
            existing.updated_at = Utc::now();
            existing.clone()
        } else {
            // 创建模式：必须有完整数据
            if api_key.is_empty() || base_url.is_empty() {
                return Err(anyhow!("创建 Profile 时 API Key 和 Base URL 不能为空"));
            }
            ClaudeProfile {
                api_key,
                base_url,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                raw_settings: None,
                raw_config_json: None,
            }
        };

        store.claude_code.insert(name.to_string(), profile);
        store.metadata.last_updated = Utc::now();
        self.save_profiles_store(&store)?;

        // 如果当前 profile 已激活，自动重新应用配置
        let active_store = self.load_active_store()?;
        if let Some(active) = active_store.get_active("claude-code") {
            if active.profile == name {
                tracing::info!("Profile {} 处于激活状态，自动重新应用配置", name);
                self.apply_to_native("claude-code", name)?;
            }
        }

        Ok(())
    }

    pub fn get_claude_profile(&self, name: &str) -> Result<ClaudeProfile> {
        let store = self.load_profiles_store()?;
        store
            .claude_code
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow!("Claude Profile 不存在: {}", name))
    }

    pub fn delete_claude_profile(&self, name: &str) -> Result<()> {
        let mut store = self.load_profiles_store()?;
        store.claude_code.remove(name);
        store.metadata.last_updated = Utc::now();
        self.save_profiles_store(&store)
    }

    pub fn list_claude_profiles(&self) -> Result<Vec<String>> {
        let store = self.load_profiles_store()?;
        Ok(store
            .claude_code
            .keys()
            .filter(|name| !name.starts_with(RESERVED_PREFIX))
            .cloned()
            .collect())
    }

    // ==================== Codex ====================

    pub fn save_codex_profile(
        &self,
        name: &str,
        api_key: String,
        base_url: String,
        wire_api: Option<String>,
    ) -> Result<()> {
        // 保留字校验
        validate_profile_name(name)?;

        let mut store = self.load_profiles_store()?;

        let profile = if let Some(existing) = store.codex.get_mut(name) {
            // 更新模式：只更新非空字段
            if !api_key.is_empty() {
                existing.api_key = api_key;
            }
            if !base_url.is_empty() {
                existing.base_url = base_url;
            }
            if let Some(w) = wire_api {
                existing.wire_api = w;
            }
            existing.updated_at = Utc::now();
            existing.clone()
        } else {
            // 创建模式：必须有完整数据
            if api_key.is_empty() || base_url.is_empty() {
                return Err(anyhow!("创建 Profile 时 API Key 和 Base URL 不能为空"));
            }
            CodexProfile {
                api_key,
                base_url,
                wire_api: wire_api.unwrap_or_else(|| "responses".to_string()),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                raw_config_toml: None,
                raw_auth_json: None,
            }
        };

        store.codex.insert(name.to_string(), profile);
        store.metadata.last_updated = Utc::now();
        self.save_profiles_store(&store)?;

        // 如果当前 profile 已激活，自动重新应用配置
        let active_store = self.load_active_store()?;
        if let Some(active) = active_store.get_active("codex") {
            if active.profile == name {
                tracing::info!("Profile {} 处于激活状态，自动重新应用配置", name);
                self.apply_to_native("codex", name)?;
            }
        }

        Ok(())
    }

    pub fn get_codex_profile(&self, name: &str) -> Result<CodexProfile> {
        let store = self.load_profiles_store()?;
        store
            .codex
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow!("Codex Profile 不存在: {}", name))
    }

    pub fn delete_codex_profile(&self, name: &str) -> Result<()> {
        let mut store = self.load_profiles_store()?;
        store.codex.remove(name);
        store.metadata.last_updated = Utc::now();
        self.save_profiles_store(&store)
    }

    pub fn list_codex_profiles(&self) -> Result<Vec<String>> {
        let store = self.load_profiles_store()?;
        Ok(store
            .codex
            .keys()
            .filter(|name| !name.starts_with(RESERVED_PREFIX))
            .cloned()
            .collect())
    }

    // ==================== Gemini CLI ====================

    pub fn save_gemini_profile(
        &self,
        name: &str,
        api_key: String,
        base_url: String,
        model: Option<String>,
    ) -> Result<()> {
        // 保留字校验
        validate_profile_name(name)?;

        let mut store = self.load_profiles_store()?;

        let profile = if let Some(existing) = store.gemini_cli.get_mut(name) {
            // 更新模式：只更新非空字段
            if !api_key.is_empty() {
                existing.api_key = api_key;
            }
            if !base_url.is_empty() {
                existing.base_url = base_url;
            }
            if let Some(m) = model {
                if !m.is_empty() {
                    existing.model = Some(m);
                }
            }
            existing.updated_at = Utc::now();
            existing.clone()
        } else {
            // 创建模式：必须有完整数据
            if api_key.is_empty() || base_url.is_empty() {
                return Err(anyhow!("创建 Profile 时 API Key 和 Base URL 不能为空"));
            }
            GeminiProfile {
                api_key,
                base_url,
                model: model.filter(|m| !m.is_empty()),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                raw_settings: None,
                raw_env: None,
            }
        };

        store.gemini_cli.insert(name.to_string(), profile);
        store.metadata.last_updated = Utc::now();
        self.save_profiles_store(&store)?;

        // 如果当前 profile 已激活，自动重新应用配置
        let active_store = self.load_active_store()?;
        if let Some(active) = active_store.get_active("gemini-cli") {
            if active.profile == name {
                tracing::info!("Profile {} 处于激活状态，自动重新应用配置", name);
                self.apply_to_native("gemini-cli", name)?;
            }
        }

        Ok(())
    }

    pub fn get_gemini_profile(&self, name: &str) -> Result<GeminiProfile> {
        let store = self.load_profiles_store()?;
        store
            .gemini_cli
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow!("Gemini Profile 不存在: {}", name))
    }

    pub fn delete_gemini_profile(&self, name: &str) -> Result<()> {
        let mut store = self.load_profiles_store()?;
        store.gemini_cli.remove(name);
        store.metadata.last_updated = Utc::now();
        self.save_profiles_store(&store)
    }

    pub fn list_gemini_profiles(&self) -> Result<Vec<String>> {
        let store = self.load_profiles_store()?;
        Ok(store
            .gemini_cli
            .keys()
            .filter(|name| !name.starts_with(RESERVED_PREFIX))
            .cloned()
            .collect())
    }

    // ==================== 通用列表 ====================

    pub fn list_all_descriptors(&self) -> Result<Vec<ProfileDescriptor>> {
        let profiles_store = self.load_profiles_store()?;
        let active_store = self.load_active_store()?;
        let mut descriptors = Vec::new();

        // Claude Code
        let active_claude = active_store.get_active("claude-code");
        for (name, profile) in &profiles_store.claude_code {
            if name.starts_with(RESERVED_PREFIX) {
                continue; // 跳过内置 Profile
            }
            descriptors.push(ProfileDescriptor::from_claude(name, profile, active_claude));
        }

        // Codex
        let active_codex = active_store.get_active("codex");
        for (name, profile) in &profiles_store.codex {
            if name.starts_with(RESERVED_PREFIX) {
                continue; // 跳过内置 Profile
            }
            descriptors.push(ProfileDescriptor::from_codex(name, profile, active_codex));
        }

        // Gemini CLI
        let active_gemini = active_store.get_active("gemini-cli");
        for (name, profile) in &profiles_store.gemini_cli {
            if name.starts_with(RESERVED_PREFIX) {
                continue; // 跳过内置 Profile
            }
            descriptors.push(ProfileDescriptor::from_gemini(name, profile, active_gemini));
        }

        Ok(descriptors)
    }

    pub fn list_profiles(&self, tool_id: &str) -> Result<Vec<String>> {
        match tool_id {
            "claude-code" => self.list_claude_profiles(),
            "codex" => self.list_codex_profiles(),
            "gemini-cli" => self.list_gemini_profiles(),
            _ => Err(anyhow!("不支持的工具 ID: {}", tool_id)),
        }
    }

    // ==================== 激活管理 ====================

    pub fn activate_profile(&self, tool_id: &str, profile_name: &str) -> Result<()> {
        // 验证 Profile 存在
        let store = self.load_profiles_store()?;
        let exists = match tool_id {
            "claude-code" => store.claude_code.contains_key(profile_name),
            "codex" => store.codex.contains_key(profile_name),
            "gemini-cli" => store.gemini_cli.contains_key(profile_name),
            _ => return Err(anyhow!("不支持的工具 ID: {}", tool_id)),
        };

        if !exists {
            return Err(anyhow!("Profile 不存在: {} / {}", tool_id, profile_name));
        }

        // 更新 active.json
        let mut active_store = self.load_active_store()?;
        active_store.set_active(tool_id, profile_name.to_string());
        self.save_active_store(&active_store)?;

        // 应用到原生配置文件
        self.apply_to_native(tool_id, profile_name)?;

        Ok(())
    }

    pub fn get_active_profile_name(&self, tool_id: &str) -> Result<Option<String>> {
        let active_store = self.load_active_store()?;
        Ok(active_store
            .get_active(tool_id)
            .map(|ap| ap.profile.clone()))
    }

    pub fn get_active_state(&self, tool_id: &str) -> Result<Option<ActiveProfile>> {
        let active_store = self.load_active_store()?;
        Ok(active_store.get_active(tool_id).cloned())
    }

    pub fn mark_active_dirty(&self, tool_id: &str, dirty: bool) -> Result<()> {
        let mut active_store = self.load_active_store()?;
        if let Some(active) = active_store.get_active_mut(tool_id) {
            active.dirty = dirty;
        }
        self.save_active_store(&active_store)
    }

    pub fn update_active_sync_state(
        &self,
        tool_id: &str,
        checksum: Option<String>,
        dirty: bool,
    ) -> Result<()> {
        let mut active_store = self.load_active_store()?;
        if let Some(active) = active_store.get_active_mut(tool_id) {
            active.native_checksum = checksum;
            active.dirty = dirty;
        }
        self.save_active_store(&active_store)
    }

    fn apply_to_native(&self, tool_id: &str, profile_name: &str) -> Result<()> {
        self.apply_profile_to_native(tool_id, profile_name)
    }

    pub fn capture_from_native(&self, tool_id: &str, profile_name: &str) -> Result<()> {
        self.capture_profile_from_native(tool_id, profile_name)
    }

    // ==================== 内部方法（跳过保留字校验） ====================

    /// 内部方法：保存 Claude Profile（跳过保留字校验，用于系统内置 Profile）
    pub fn save_claude_profile_internal(
        &self,
        name: &str,
        api_key: String,
        base_url: String,
    ) -> Result<()> {
        let mut store = self.load_profiles_store()?;

        let profile = if let Some(existing) = store.claude_code.get_mut(name) {
            // 更新模式：只更新非空字段
            if !api_key.is_empty() {
                existing.api_key = api_key;
            }
            if !base_url.is_empty() {
                existing.base_url = base_url;
            }
            existing.updated_at = Utc::now();
            existing.clone()
        } else {
            // 创建模式：必须有完整数据
            if api_key.is_empty() || base_url.is_empty() {
                return Err(anyhow!("创建 Profile 时 API Key 和 Base URL 不能为空"));
            }
            ClaudeProfile {
                api_key,
                base_url,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                raw_settings: None,
                raw_config_json: None,
            }
        };

        store.claude_code.insert(name.to_string(), profile);
        store.metadata.last_updated = Utc::now();
        self.save_profiles_store(&store)?;

        tracing::debug!("已创建/更新内置 Profile: {}", name);
        Ok(())
    }

    /// 内部方法：保存 Codex Profile（跳过保留字校验，用于系统内置 Profile）
    pub fn save_codex_profile_internal(
        &self,
        name: &str,
        api_key: String,
        base_url: String,
        wire_api: Option<String>,
    ) -> Result<()> {
        let mut store = self.load_profiles_store()?;

        let profile = if let Some(existing) = store.codex.get_mut(name) {
            // 更新模式：只更新非空字段
            if !api_key.is_empty() {
                existing.api_key = api_key;
            }
            if !base_url.is_empty() {
                existing.base_url = base_url;
            }
            if let Some(w) = wire_api {
                existing.wire_api = w;
            }
            existing.updated_at = Utc::now();
            existing.clone()
        } else {
            // 创建模式：必须有完整数据
            if api_key.is_empty() || base_url.is_empty() {
                return Err(anyhow!("创建 Profile 时 API Key 和 Base URL 不能为空"));
            }
            CodexProfile {
                api_key,
                base_url,
                wire_api: wire_api.unwrap_or_else(|| "responses".to_string()),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                raw_config_toml: None,
                raw_auth_json: None,
            }
        };

        store.codex.insert(name.to_string(), profile);
        store.metadata.last_updated = Utc::now();
        self.save_profiles_store(&store)?;

        tracing::debug!("已创建/更新内置 Profile: {}", name);
        Ok(())
    }

    /// 内部方法：保存 Gemini Profile（跳过保留字校验，用于系统内置 Profile）
    pub fn save_gemini_profile_internal(
        &self,
        name: &str,
        api_key: String,
        base_url: String,
        model: Option<String>,
    ) -> Result<()> {
        let mut store = self.load_profiles_store()?;

        let profile = if let Some(existing) = store.gemini_cli.get_mut(name) {
            // 更新模式：只更新非空字段
            if !api_key.is_empty() {
                existing.api_key = api_key;
            }
            if !base_url.is_empty() {
                existing.base_url = base_url;
            }
            if let Some(m) = model {
                if !m.is_empty() {
                    existing.model = Some(m);
                }
            }
            existing.updated_at = Utc::now();
            existing.clone()
        } else {
            // 创建模式：必须有完整数据
            if api_key.is_empty() || base_url.is_empty() {
                return Err(anyhow!("创建 Profile 时 API Key 和 Base URL 不能为空"));
            }
            GeminiProfile {
                api_key,
                base_url,
                model: model.filter(|m| !m.is_empty()),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                raw_settings: None,
                raw_env: None,
            }
        };

        store.gemini_cli.insert(name.to_string(), profile);
        store.metadata.last_updated = Utc::now();
        self.save_profiles_store(&store)?;

        tracing::debug!("已创建/更新内置 Profile: {}", name);
        Ok(())
    }

    // ==================== 删除 ====================

    pub fn delete_profile(&self, tool_id: &str, name: &str) -> Result<()> {
        match tool_id {
            "claude-code" => self.delete_claude_profile(name),
            "codex" => self.delete_codex_profile(name),
            "gemini-cli" => self.delete_gemini_profile(name),
            _ => Err(anyhow!("不支持的工具 ID: {}", tool_id)),
        }
    }
}

impl Default for ProfileManager {
    fn default() -> Self {
        Self::new().expect("创建 ProfileManager 失败")
    }
}
