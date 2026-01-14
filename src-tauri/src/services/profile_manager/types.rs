//! Profile 管理数据类型定义（v2.1 - 简化版）
//!
//! 设计原则：工具分组即类型，使用具体结构体替代 enum

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ==================== AMP Profile Selection ====================

/// AMP Profile 引用（指向某工具的某个 profile）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileRef {
    pub tool_id: String,
    pub profile_name: String,
}

/// AMP Profile 选择（引用其他工具的 profile）
/// AMP 不创建独立 profile，而是从 3 个工具中选择
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmpProfileSelection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude: Option<ProfileRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex: Option<ProfileRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gemini: Option<ProfileRef>,
    pub updated_at: DateTime<Utc>,
}

impl Default for AmpProfileSelection {
    fn default() -> Self {
        Self {
            claude: None,
            codex: None,
            gemini: None,
            updated_at: Utc::now(),
        }
    }
}

// ==================== Profile 来源标记 ====================

/// Profile 来源类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(tag = "type")]
pub enum ProfileSource {
    /// 用户自定义创建
    #[default]
    Custom,
    /// 从供应商远程令牌导入
    ImportedFromProvider {
        /// 供应商 ID
        provider_id: String,
        /// 供应商名称
        provider_name: String,
        /// 远程令牌 ID
        remote_token_id: i64,
        /// 远程令牌名称
        remote_token_name: String,
        /// 所属分组
        group: String,
        /// 导入时间（Unix 时间戳）
        imported_at: i64,
    },
}

// ==================== 具体 Profile 类型 ====================

/// Claude Code Profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeProfile {
    pub api_key: String,
    pub base_url: String,
    #[serde(default)]
    pub source: ProfileSource,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_settings: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_config_json: Option<serde_json::Value>,
    /// 价格模板 ID（用于成本计算）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pricing_template_id: Option<String>,
}

/// Codex Profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexProfile {
    pub api_key: String,
    pub base_url: String,
    #[serde(default = "default_codex_wire_api")]
    pub wire_api: String, // "responses" 或 "chat"
    #[serde(default)]
    pub source: ProfileSource,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_config_toml: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_auth_json: Option<serde_json::Value>,
    /// 价格模板 ID（用于成本计算）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pricing_template_id: Option<String>,
}

fn default_codex_wire_api() -> String {
    "responses".to_string()
}

/// Gemini CLI Profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiProfile {
    pub api_key: String,
    pub base_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default)]
    pub source: ProfileSource,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_settings: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_env: Option<String>,
    /// 价格模板 ID（用于成本计算）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pricing_template_id: Option<String>,
}

// ==================== profiles.json 结构 ====================

/// profiles.json 顶层结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilesStore {
    pub version: String,
    #[serde(rename = "claude-code")]
    pub claude_code: HashMap<String, ClaudeProfile>,
    pub codex: HashMap<String, CodexProfile>,
    #[serde(rename = "gemini-cli")]
    pub gemini_cli: HashMap<String, GeminiProfile>,
    pub metadata: ProfilesMetadata,
}

impl ProfilesStore {
    /// 创建空的 ProfilesStore
    pub fn new() -> Self {
        Self {
            version: "2.0.0".to_string(),
            claude_code: HashMap::new(),
            codex: HashMap::new(),
            gemini_cli: HashMap::new(),
            metadata: ProfilesMetadata {
                last_updated: Utc::now(),
            },
        }
    }

    /// 获取指定工具的 Profile（通用接口）
    pub fn get_tool_profiles(&self, tool_id: &str) -> Option<Vec<(String, String, String)>> {
        match tool_id {
            "claude-code" => Some(
                self.claude_code
                    .iter()
                    .map(|(name, p)| (name.clone(), p.api_key.clone(), p.base_url.clone()))
                    .collect(),
            ),
            "codex" => Some(
                self.codex
                    .iter()
                    .map(|(name, p)| (name.clone(), p.api_key.clone(), p.base_url.clone()))
                    .collect(),
            ),
            "gemini-cli" => Some(
                self.gemini_cli
                    .iter()
                    .map(|(name, p)| (name.clone(), p.api_key.clone(), p.base_url.clone()))
                    .collect(),
            ),
            _ => None,
        }
    }
}

impl Default for ProfilesStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilesMetadata {
    pub last_updated: DateTime<Utc>,
}

// ==================== active.json 结构 ====================

/// active.json 顶层结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveStore {
    pub version: String,
    #[serde(rename = "claude-code")]
    pub claude_code: Option<ActiveProfile>,
    pub codex: Option<ActiveProfile>,
    #[serde(rename = "gemini-cli")]
    pub gemini_cli: Option<ActiveProfile>,
    pub metadata: ActiveMetadata,
}

impl ActiveStore {
    pub fn new() -> Self {
        Self {
            version: "2.0.0".to_string(),
            claude_code: None,
            codex: None,
            gemini_cli: None,
            metadata: ActiveMetadata {
                last_updated: Utc::now(),
            },
        }
    }

    pub fn get_active(&self, tool_id: &str) -> Option<&ActiveProfile> {
        match tool_id {
            "claude-code" => self.claude_code.as_ref(),
            "codex" => self.codex.as_ref(),
            "gemini-cli" => self.gemini_cli.as_ref(),
            _ => None,
        }
    }

    pub fn get_active_mut(&mut self, tool_id: &str) -> Option<&mut ActiveProfile> {
        match tool_id {
            "claude-code" => self.claude_code.as_mut(),
            "codex" => self.codex.as_mut(),
            "gemini-cli" => self.gemini_cli.as_mut(),
            _ => None,
        }
    }

    pub fn set_active(&mut self, tool_id: &str, profile_name: String) {
        let active = ActiveProfile {
            profile: profile_name,
            switched_at: Utc::now(),
            native_checksum: None,
            dirty: false,
            native_snapshot: None,
            last_synced_at: Utc::now(),
        };

        match tool_id {
            "claude-code" => self.claude_code = Some(active),
            "codex" => self.codex = Some(active),
            "gemini-cli" => self.gemini_cli = Some(active),
            _ => {}
        }

        self.metadata.last_updated = Utc::now();
    }

    pub fn clear_active(&mut self, tool_id: &str) {
        match tool_id {
            "claude-code" => self.claude_code = None,
            "codex" => self.codex = None,
            "gemini-cli" => self.gemini_cli = None,
            _ => {}
        }
        self.metadata.last_updated = Utc::now();
    }
}

impl Default for ActiveStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveProfile {
    pub profile: String,
    pub switched_at: DateTime<Utc>,
    #[serde(default)]
    pub native_checksum: Option<String>,
    #[serde(default)]
    pub dirty: bool,
    /// 原生配置文件的完整快照（用于阻止变更时恢复）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_snapshot: Option<serde_json::Value>,
    /// 最后同步时间
    #[serde(default)]
    pub last_synced_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveMetadata {
    pub last_updated: DateTime<Utc>,
}

// ==================== Profile Descriptor（前端展示用）====================

/// Profile 描述符（用于前端展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileDescriptor {
    pub tool_id: String,
    pub name: String,
    pub api_key_preview: String,
    pub base_url: String,
    pub source: ProfileSource,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub switched_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 价格模板 ID（用于成本计算）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pricing_template_id: Option<String>,
}

impl ProfileDescriptor {
    pub fn from_claude(
        name: &str,
        profile: &ClaudeProfile,
        active_profile: Option<&ActiveProfile>,
    ) -> Self {
        let is_active = active_profile.map(|ap| ap.profile == name).unwrap_or(false);
        let switched_at = if is_active {
            active_profile.map(|ap| ap.switched_at)
        } else {
            None
        };

        Self {
            tool_id: "claude-code".to_string(),
            name: name.to_string(),
            api_key_preview: mask_api_key(&profile.api_key),
            base_url: profile.base_url.clone(),
            source: profile.source.clone(),
            created_at: profile.created_at,
            updated_at: profile.updated_at,
            is_active,
            switched_at,
            provider: None,
            model: None,
            pricing_template_id: profile.pricing_template_id.clone(),
        }
    }

    pub fn from_codex(
        name: &str,
        profile: &CodexProfile,
        active_profile: Option<&ActiveProfile>,
    ) -> Self {
        let is_active = active_profile.map(|ap| ap.profile == name).unwrap_or(false);
        let switched_at = if is_active {
            active_profile.map(|ap| ap.switched_at)
        } else {
            None
        };

        Self {
            tool_id: "codex".to_string(),
            name: name.to_string(),
            api_key_preview: mask_api_key(&profile.api_key),
            base_url: profile.base_url.clone(),
            source: profile.source.clone(),
            created_at: profile.created_at,
            updated_at: profile.updated_at,
            is_active,
            switched_at,
            provider: Some(profile.wire_api.clone()), // 前端仍使用 provider 字段名
            model: None,
            pricing_template_id: profile.pricing_template_id.clone(),
        }
    }

    pub fn from_gemini(
        name: &str,
        profile: &GeminiProfile,
        active_profile: Option<&ActiveProfile>,
    ) -> Self {
        let is_active = active_profile.map(|ap| ap.profile == name).unwrap_or(false);
        let switched_at = if is_active {
            active_profile.map(|ap| ap.switched_at)
        } else {
            None
        };

        Self {
            tool_id: "gemini-cli".to_string(),
            name: name.to_string(),
            api_key_preview: mask_api_key(&profile.api_key),
            base_url: profile.base_url.clone(),
            source: profile.source.clone(),
            created_at: profile.created_at,
            updated_at: profile.updated_at,
            is_active,
            switched_at,
            provider: None,
            model: profile.model.clone(),
            pricing_template_id: profile.pricing_template_id.clone(),
        }
    }
}

// ==================== 辅助函数 ====================

fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        return "****".to_string();
    }
    let prefix = &key[..4];
    let suffix = &key[key.len() - 4..];
    format!("{}...{}", prefix, suffix)
}

// ==================== 令牌导入状态 ====================

/// 令牌导入状态（用于检测令牌是否已导入到某个工具）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenImportStatus {
    /// 工具 ID (claude-code, codex, gemini-cli)
    pub tool_id: String,
    /// 是否已导入
    pub is_imported: bool,
    /// 已导入的 Profile 名称（如果已导入）
    pub imported_profile_name: Option<String>,
}
