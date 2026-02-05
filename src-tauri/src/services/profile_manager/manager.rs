//! ProfileManager 核心实现（v2.1 - 简化版）

use super::types::*;
use crate::data::DataManager;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use fs2::FileExt;
use std::fs::File;
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

    pub fn load_profiles_store(&self) -> Result<ProfilesStore> {
        if !self.profiles_path.exists() {
            return Ok(ProfilesStore::new());
        }
        let value = self.data_manager.json().read(&self.profiles_path)?;
        serde_json::from_value(value).context("反序列化 ProfilesStore 失败")
    }

    pub fn save_profiles_store(&self, store: &ProfilesStore) -> Result<()> {
        // 创建锁文件（与 profiles.json 同目录）
        let lock_path = self.profiles_path.with_extension("lock");
        let lock_file = File::create(&lock_path).context("创建锁文件失败")?;

        // 获取排他锁（阻塞等待其他写操作完成）
        lock_file.lock_exclusive().context("获取文件锁失败")?;

        // 执行写入（受锁保护）
        let value = serde_json::to_value(store)?;
        self.data_manager
            .json()
            .write(&self.profiles_path, &value)?;

        // 锁在 lock_file drop 时自动释放
        Ok(())
    }

    pub fn load_active_store(&self) -> Result<ActiveStore> {
        if !self.active_path.exists() {
            return Ok(ActiveStore::new());
        }
        let value = self.data_manager.json().read(&self.active_path)?;
        serde_json::from_value(value).context("反序列化 ActiveStore 失败")
    }

    pub fn save_active_store(&self, store: &ActiveStore) -> Result<()> {
        // 创建锁文件（与 active.json 同目录）
        let lock_path = self.active_path.with_extension("lock");
        let lock_file = File::create(&lock_path).context("创建锁文件失败")?;

        // 获取排他锁（阻塞等待其他写操作完成）
        lock_file.lock_exclusive().context("获取文件锁失败")?;

        // 执行写入（受锁保护）
        let value = serde_json::to_value(store)?;
        self.data_manager.json().write(&self.active_path, &value)?;

        // 锁在 lock_file drop 时自动释放
        Ok(())
    }

    // ==================== Claude Code ====================

    pub fn save_claude_profile(&self, name: &str, api_key: String, base_url: String) -> Result<()> {
        self.save_claude_profile_with_template(name, api_key, base_url, None)
    }

    /// 保存 Claude Profile（支持价格模板）
    pub fn save_claude_profile_with_template(
        &self,
        name: &str,
        api_key: String,
        base_url: String,
        pricing_template_id: Option<String>,
    ) -> Result<()> {
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
            // Phase 6: 更新价格模板 ID（允许清空）
            existing.pricing_template_id = pricing_template_id;
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
                source: ProfileSource::Custom,
                pricing_template_id, // Phase 6: 价格模板 ID
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
        let mut names: Vec<String> = store
            .claude_code
            .keys()
            .filter(|name| !name.starts_with(RESERVED_PREFIX))
            .cloned()
            .collect();
        names.sort();
        Ok(names)
    }

    // ==================== Codex ====================

    pub fn save_codex_profile(
        &self,
        name: &str,
        api_key: String,
        base_url: String,
        wire_api: Option<String>,
    ) -> Result<()> {
        self.save_codex_profile_with_template(name, api_key, base_url, wire_api, None)
    }

    /// 保存 Codex Profile（支持价格模板）
    pub fn save_codex_profile_with_template(
        &self,
        name: &str,
        api_key: String,
        base_url: String,
        wire_api: Option<String>,
        pricing_template_id: Option<String>,
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
            // Phase 6: 更新价格模板 ID（允许清空）
            existing.pricing_template_id = pricing_template_id;
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
                source: ProfileSource::Custom,
                pricing_template_id, // Phase 6: 价格模板 ID
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
        let mut names: Vec<String> = store
            .codex
            .keys()
            .filter(|name| !name.starts_with(RESERVED_PREFIX))
            .cloned()
            .collect();
        names.sort();
        Ok(names)
    }

    // ==================== Gemini CLI ====================

    pub fn save_gemini_profile(
        &self,
        name: &str,
        api_key: String,
        base_url: String,
        model: Option<String>,
    ) -> Result<()> {
        self.save_gemini_profile_with_template(name, api_key, base_url, model, None)
    }

    /// 保存 Gemini Profile（支持价格模板）
    pub fn save_gemini_profile_with_template(
        &self,
        name: &str,
        api_key: String,
        base_url: String,
        model: Option<String>,
        pricing_template_id: Option<String>,
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
            // Phase 6: 更新价格模板 ID（允许清空）
            existing.pricing_template_id = pricing_template_id;
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
                source: ProfileSource::Custom,
                pricing_template_id, // Phase 6: 价格模板 ID
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
        let mut names: Vec<String> = store
            .gemini_cli
            .keys()
            .filter(|name| !name.starts_with(RESERVED_PREFIX))
            .cloned()
            .collect();
        names.sort();
        Ok(names)
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

        // HashMap 遍历顺序不稳定：这里做显式排序，保证前端展示稳定
        fn tool_rank(tool_id: &str) -> u8 {
            match tool_id {
                "claude-code" => 0,
                "codex" => 1,
                "gemini-cli" => 2,
                _ => 99,
            }
        }

        descriptors.sort_by(|a, b| {
            tool_rank(&a.tool_id)
                .cmp(&tool_rank(&b.tool_id))
                // 同工具内：激活的 Profile 置顶
                .then_with(|| b.is_active.cmp(&a.is_active))
                // 其余按创建时间倒序（新建的更靠前），再按名称稳定排序
                .then_with(|| b.created_at.cmp(&a.created_at))
                .then_with(|| a.name.cmp(&b.name))
        });

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

        // 读取应用后的配置并保存快照（为每个工具读取所有配置文件）
        let tool = crate::models::Tool::by_id(tool_id)
            .ok_or_else(|| anyhow!("未找到工具: {}", tool_id))?;
        let manager = crate::data::DataManager::new();
        let mut files_snapshot = std::collections::HashMap::new();

        for filename in tool.config_files() {
            let config_path = tool.config_dir.join(&filename);
            if !config_path.exists() {
                continue;
            }

            let content = if filename.ends_with(".json") {
                // JSON 文件：直接读取
                manager.json_uncached().read(&config_path)?
            } else if filename.ends_with(".toml") {
                // TOML 文件：读取后转换为 JSON
                let doc = manager.toml().read_document(&config_path)?;
                let toml_str = doc.to_string();
                let toml_value: toml::Value =
                    toml::from_str(&toml_str).map_err(|e| anyhow!("TOML 解析失败: {}", e))?;
                serde_json::to_value(toml_value)?
            } else if filename.ends_with(".env") || filename == ".env" {
                // ENV 文件：读取为 HashMap 后转换为 JSON
                let env_map = manager.env().read(&config_path)?;
                serde_json::to_value(env_map)?
            } else {
                continue;
            };

            files_snapshot.insert(filename.clone(), content);
        }

        if !files_snapshot.is_empty() {
            let snapshot = serde_json::to_value(files_snapshot)?;
            self.save_native_snapshot(tool_id, snapshot)?;
            tracing::debug!("已保存 Profile 快照: {} / {}", tool_id, profile_name);
        }

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
                source: ProfileSource::Custom,
                pricing_template_id: None,
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
                source: ProfileSource::Custom,
                pricing_template_id: None,
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
                source: ProfileSource::Custom,
                pricing_template_id: None,
            }
        };

        store.gemini_cli.insert(name.to_string(), profile);
        store.metadata.last_updated = Utc::now();
        self.save_profiles_store(&store)?;

        tracing::debug!("已创建/更新内置 Profile: {}", name);
        Ok(())
    }

    // ==================== 导入状态检测 ====================

    /// 检测令牌是否已导入到任何工具
    ///
    /// 遍历所有工具的 Profile，检查是否存在相同 provider_id 和 remote_token_id 的导入记录
    pub fn check_import_status(
        &self,
        provider_id: &str,
        remote_token_id: i64,
    ) -> Result<Vec<super::types::TokenImportStatus>> {
        const TOOLS: [&str; 3] = ["claude-code", "codex", "gemini-cli"];
        let store = self.load_profiles_store()?;
        let mut results = Vec::new();

        for &tool_id in &TOOLS {
            let mut is_imported = false;
            let mut imported_profile_name = None;

            // 根据工具类型检查对应的 Profile 集合
            match tool_id {
                "claude-code" => {
                    for (name, profile) in &store.claude_code {
                        if let ProfileSource::ImportedFromProvider {
                            provider_id: pid,
                            remote_token_id: tid,
                            ..
                        } = &profile.source
                        {
                            if pid == provider_id && *tid == remote_token_id {
                                is_imported = true;
                                imported_profile_name = Some(name.clone());
                                break;
                            }
                        }
                    }
                }
                "codex" => {
                    for (name, profile) in &store.codex {
                        if let ProfileSource::ImportedFromProvider {
                            provider_id: pid,
                            remote_token_id: tid,
                            ..
                        } = &profile.source
                        {
                            if pid == provider_id && *tid == remote_token_id {
                                is_imported = true;
                                imported_profile_name = Some(name.clone());
                                break;
                            }
                        }
                    }
                }
                "gemini-cli" => {
                    for (name, profile) in &store.gemini_cli {
                        if let ProfileSource::ImportedFromProvider {
                            provider_id: pid,
                            remote_token_id: tid,
                            ..
                        } = &profile.source
                        {
                            if pid == provider_id && *tid == remote_token_id {
                                is_imported = true;
                                imported_profile_name = Some(name.clone());
                                break;
                            }
                        }
                    }
                }
                _ => unreachable!("TOOLS 数组只包含已知工具 ID"),
            }

            results.push(super::types::TokenImportStatus {
                tool_id: tool_id.to_string(),
                is_imported,
                imported_profile_name,
            });
        }

        Ok(results)
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

    // ==================== 快照管理 ====================

    /// 保存原生配置快照到 ActiveProfile
    ///
    /// # Arguments
    ///
    /// * `tool_id` - 工具 ID
    /// * `snapshot` - 配置快照（JSON Value）
    ///
    /// # Errors
    ///
    /// 当保存失败时返回错误
    pub fn save_native_snapshot(&self, tool_id: &str, snapshot: serde_json::Value) -> Result<()> {
        let mut active_store = self.load_active_store()?;

        // 获取激活的 Profile 并更新快照
        if let Some(active) = active_store.get_active_mut(tool_id) {
            active.native_snapshot = Some(snapshot);
            active.last_synced_at = chrono::Utc::now();
            self.save_active_store(&active_store)
        } else {
            Err(anyhow!("工具 {} 没有激活的 Profile", tool_id))
        }
    }

    /// 获取原生配置快照
    ///
    /// # Arguments
    ///
    /// * `tool_id` - 工具 ID
    ///
    /// # Returns
    ///
    /// 返回配置快照，如果不存在返回 None
    pub fn get_native_snapshot(&self, tool_id: &str) -> Result<Option<serde_json::Value>> {
        let active_store = self.load_active_store()?;
        Ok(active_store
            .get_active(tool_id)
            .and_then(|a| a.native_snapshot.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tempfile::TempDir;

    fn test_manager(temp_dir: &TempDir) -> ProfileManager {
        ProfileManager {
            data_manager: DataManager::new(),
            profiles_path: temp_dir.path().join("profiles.json"),
            active_path: temp_dir.path().join("active.json"),
        }
    }

    #[test]
    fn test_list_profiles_sorted_and_filters_reserved() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let manager = test_manager(&temp_dir);

        let mut store = ProfilesStore::new();
        let created_at = Utc.timestamp_opt(1, 0).single().unwrap();
        let updated_at = created_at;

        store.claude_code.insert(
            "b".to_string(),
            ClaudeProfile {
                api_key: "k".to_string(),
                base_url: "u".to_string(),
                source: ProfileSource::Custom,
                created_at,
                updated_at,
                raw_settings: None,
                raw_config_json: None,
                pricing_template_id: None,
            },
        );
        store.claude_code.insert(
            "a".to_string(),
            ClaudeProfile {
                api_key: "k".to_string(),
                base_url: "u".to_string(),
                source: ProfileSource::Custom,
                created_at,
                updated_at,
                raw_settings: None,
                raw_config_json: None,
                pricing_template_id: None,
            },
        );
        store.claude_code.insert(
            "dc_proxy_internal".to_string(),
            ClaudeProfile {
                api_key: "k".to_string(),
                base_url: "u".to_string(),
                source: ProfileSource::Custom,
                created_at,
                updated_at,
                raw_settings: None,
                raw_config_json: None,
                pricing_template_id: None,
            },
        );

        manager.save_profiles_store(&store)?;

        let names = manager.list_claude_profiles()?;
        assert_eq!(names, vec!["a".to_string(), "b".to_string()]);
        Ok(())
    }

    #[test]
    fn test_list_all_descriptors_sorted_stable() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let manager = test_manager(&temp_dir);

        let mut store = ProfilesStore::new();

        let t1 = Utc.timestamp_opt(1, 0).single().unwrap();
        let t2 = Utc.timestamp_opt(2, 0).single().unwrap();
        let t3 = Utc.timestamp_opt(3, 0).single().unwrap();

        // 同工具内：激活置顶，其他按 created_at 倒序
        store.claude_code.insert(
            "p_middle".to_string(),
            ClaudeProfile {
                api_key: "k".to_string(),
                base_url: "u".to_string(),
                source: ProfileSource::Custom,
                created_at: t2,
                updated_at: t2,
                raw_settings: None,
                raw_config_json: None,
                pricing_template_id: None,
            },
        );
        store.claude_code.insert(
            "p_new".to_string(),
            ClaudeProfile {
                api_key: "k".to_string(),
                base_url: "u".to_string(),
                source: ProfileSource::Custom,
                created_at: t3,
                updated_at: t3,
                raw_settings: None,
                raw_config_json: None,
                pricing_template_id: None,
            },
        );
        store.claude_code.insert(
            "p_active_old".to_string(),
            ClaudeProfile {
                api_key: "k".to_string(),
                base_url: "u".to_string(),
                source: ProfileSource::Custom,
                created_at: t1,
                updated_at: t1,
                raw_settings: None,
                raw_config_json: None,
                pricing_template_id: None,
            },
        );

        // 其他工具插入一个，验证 tool_id rank 的稳定性（claude -> codex -> gemini）
        store.codex.insert(
            "codex_one".to_string(),
            CodexProfile {
                api_key: "k".to_string(),
                base_url: "u".to_string(),
                wire_api: "responses".to_string(),
                source: ProfileSource::Custom,
                created_at: t2,
                updated_at: t2,
                raw_config_toml: None,
                raw_auth_json: None,
                pricing_template_id: None,
            },
        );
        store.gemini_cli.insert(
            "gemini_one".to_string(),
            GeminiProfile {
                api_key: "k".to_string(),
                base_url: "u".to_string(),
                model: None,
                source: ProfileSource::Custom,
                created_at: t2,
                updated_at: t2,
                raw_settings: None,
                raw_env: None,
                pricing_template_id: None,
            },
        );

        manager.save_profiles_store(&store)?;

        let mut active = ActiveStore::new();
        active.set_active("claude-code", "p_active_old".to_string());
        manager.save_active_store(&active)?;

        let descriptors = manager.list_all_descriptors()?;
        let names: Vec<String> = descriptors.iter().map(|d| d.name.clone()).collect();

        assert_eq!(
            names,
            vec![
                "p_active_old".to_string(),
                "p_new".to_string(),
                "p_middle".to_string(),
                "codex_one".to_string(),
                "gemini_one".to_string()
            ]
        );

        Ok(())
    }
}

impl Default for ProfileManager {
    fn default() -> Self {
        Self::new().expect("创建 ProfileManager 失败")
    }
}

// ==================== AMP Profile Selection ====================

impl ProfileManager {
    /// 获取 AMP Profile 选择的存储路径
    fn amp_selection_path(&self) -> PathBuf {
        self.profiles_path
            .parent()
            .unwrap()
            .join("amp_selection.json")
    }

    /// 获取 AMP Profile 选择
    pub fn get_amp_selection(&self) -> Result<AmpProfileSelection> {
        let path = self.amp_selection_path();
        if !path.exists() {
            return Ok(AmpProfileSelection::default());
        }
        let value = self.data_manager.json().read(&path)?;
        serde_json::from_value(value).context("反序列化 AmpProfileSelection 失败")
    }

    /// 保存 AMP Profile 选择
    pub fn save_amp_selection(&self, selection: &AmpProfileSelection) -> Result<()> {
        let path = self.amp_selection_path();

        // 验证引用的 profile 存在
        let store = self.load_profiles_store()?;

        if let Some(ref claude) = selection.claude {
            if !store.claude_code.contains_key(&claude.profile_name) {
                return Err(anyhow!("Claude Profile 不存在: {}", claude.profile_name));
            }
        }

        if let Some(ref codex) = selection.codex {
            if !store.codex.contains_key(&codex.profile_name) {
                return Err(anyhow!("Codex Profile 不存在: {}", codex.profile_name));
            }
        }

        if let Some(ref gemini) = selection.gemini {
            if !store.gemini_cli.contains_key(&gemini.profile_name) {
                return Err(anyhow!("Gemini Profile 不存在: {}", gemini.profile_name));
            }
        }

        let value = serde_json::to_value(selection)?;
        self.data_manager.json().write(&path, &value)?;
        Ok(())
    }

    /// 解析 AMP Profile 选择，返回实际的 Profile 数据
    pub fn resolve_amp_selection(
        &self,
    ) -> Result<(
        Option<ClaudeProfile>,
        Option<CodexProfile>,
        Option<GeminiProfile>,
    )> {
        let selection = self.get_amp_selection()?;
        let store = self.load_profiles_store()?;

        let claude = selection
            .claude
            .and_then(|r| store.claude_code.get(&r.profile_name).cloned());
        let codex = selection
            .codex
            .and_then(|r| store.codex.get(&r.profile_name).cloned());
        let gemini = selection
            .gemini
            .and_then(|r| store.gemini_cli.get(&r.profile_name).cloned());

        Ok((claude, codex, gemini))
    }
}
