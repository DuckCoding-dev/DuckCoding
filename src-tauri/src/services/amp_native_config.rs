//! AMP Code 原生配置文件管理
//!
//! 配置文件位置：
//! - ~/.config/amp/settings.json - 存储 amp.url
//! - ~/.local/share/amp/secrets.json - 存储 apiKey@{url}
//!
//! Windows: %USERPROFILE%\.config\amp\... 和 %USERPROFILE%\.local\share\amp\...

use crate::data::DataManager;
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::path::PathBuf;

/// AMP Code 配置备份信息（语义备份）
#[derive(Debug, Clone)]
pub struct AmpConfigBackup {
    pub settings: Option<Value>,
    pub secrets: Option<Value>,
}

/// 获取 home 目录（跨平台）
fn home_dir() -> Result<PathBuf> {
    if let Some(p) = dirs::home_dir() {
        return Ok(p);
    }
    #[cfg(windows)]
    if let Ok(p) = std::env::var("USERPROFILE") {
        return Ok(PathBuf::from(p));
    }
    #[cfg(not(windows))]
    if let Ok(p) = std::env::var("HOME") {
        return Ok(PathBuf::from(p));
    }
    Err(anyhow!("无法获取 home 目录"))
}

/// 获取 AMP Code settings.json 路径
/// 所有平台: ~/.config/amp/settings.json
fn amp_settings_path() -> Result<PathBuf> {
    Ok(home_dir()?
        .join(".config")
        .join("amp")
        .join("settings.json"))
}

/// 获取 AMP Code secrets.json 路径
/// 所有平台: ~/.local/share/amp/secrets.json
fn amp_secrets_path() -> Result<PathBuf> {
    Ok(home_dir()?
        .join(".local")
        .join("share")
        .join("amp")
        .join("secrets.json"))
}

/// 读取当前 AMP Code 配置（语义备份）
/// 注意：文件存在但读取失败时会返回错误，避免还原时误删用户文件
pub fn backup_amp_config() -> Result<AmpConfigBackup> {
    let dm = DataManager::global();
    let jm = dm.json_uncached();

    let settings_path = amp_settings_path()?;
    let secrets_path = amp_secrets_path()?;

    let settings = if settings_path.exists() {
        Some(
            jm.read(&settings_path)
                .map_err(|e| anyhow!("读取 settings.json 失败: {}", e))?,
        )
    } else {
        None
    };

    let secrets = if secrets_path.exists() {
        Some(
            jm.read(&secrets_path)
                .map_err(|e| anyhow!("读取 secrets.json 失败: {}", e))?,
        )
    } else {
        None
    };

    Ok(AmpConfigBackup { settings, secrets })
}

/// 应用代理配置到 Amp（设置本地代理地址和密钥）
/// 注意：如果配置文件存在但格式错误，会返回错误而非静默覆盖
pub fn apply_proxy_config(proxy_url: &str, local_api_key: &str) -> Result<()> {
    let dm = DataManager::global();
    let jm = dm.json_uncached();

    let settings_path = amp_settings_path()?;
    let secrets_path = amp_secrets_path()?;

    // 读取现有配置或创建新的（文件存在但格式错误时返回错误）
    let mut settings: Value = if settings_path.exists() {
        jm.read(&settings_path)
            .map_err(|e| anyhow!("读取 settings.json 失败: {}", e))?
    } else {
        serde_json::json!({})
    };

    // 更新 settings.json（使用 object insert 因为 "amp.url" 含 .）
    let settings_obj = settings
        .as_object_mut()
        .ok_or_else(|| anyhow!("settings.json 格式错误：不是 JSON 对象"))?;
    settings_obj.insert("amp.url".to_string(), Value::String(proxy_url.to_string()));
    jm.write(&settings_path, &settings)?;

    // 读取 secrets.json（文件存在但格式错误时返回错误）
    let mut secrets: Value = if secrets_path.exists() {
        jm.read(&secrets_path)
            .map_err(|e| anyhow!("读取 secrets.json 失败: {}", e))?
    } else {
        serde_json::json!({})
    };

    // 更新 secrets.json（key 含 @ 和 url，使用 object insert）
    let secrets_obj = secrets
        .as_object_mut()
        .ok_or_else(|| anyhow!("secrets.json 格式错误：不是 JSON 对象"))?;
    let key_name = format!("apiKey@{}", proxy_url);
    secrets_obj.insert(key_name, Value::String(local_api_key.to_string()));
    jm.write(&secrets_path, &secrets)?;

    tracing::info!(
        proxy_url = %proxy_url,
        "已应用 AMP Code 代理配置"
    );

    Ok(())
}

/// 完整还原 AMP Code 配置到原始状态
pub fn restore_amp_config(backup: &AmpConfigBackup) -> Result<()> {
    let dm = DataManager::global();
    let jm = dm.json_uncached();

    let settings_path = amp_settings_path()?;
    let secrets_path = amp_secrets_path()?;

    // 还原 settings.json
    if let Some(value) = &backup.settings {
        jm.write(&settings_path, value)?;
        tracing::debug!("已还原 settings.json");
    } else if settings_path.exists() {
        jm.delete(&settings_path, None)?;
        tracing::debug!("已删除 settings.json（原本不存在）");
    }

    // 还原 secrets.json
    if let Some(value) = &backup.secrets {
        jm.write(&secrets_path, value)?;
        tracing::debug!("已还原 secrets.json");
    } else if secrets_path.exists() {
        jm.delete(&secrets_path, None)?;
        tracing::debug!("已删除 secrets.json（原本不存在）");
    }

    tracing::info!("已完整还原 AMP Code 配置");

    Ok(())
}
