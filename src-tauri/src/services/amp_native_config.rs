//! AMP Code 原生配置文件管理
//!
//! 配置文件位置：
//! - ~/.config/amp/settings.json - 存储 amp.url
//! - ~/.local/share/amp/secrets.json - 存储 apiKey@{url}

use anyhow::{anyhow, Result};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

/// Amp 配置备份信息（完整文件内容）
#[derive(Debug, Clone)]
pub struct AmpConfigBackup {
    pub settings_json: Option<String>,
    pub secrets_json: Option<String>,
}

/// 获取 Amp settings.json 路径
/// macOS/Linux: ~/.config/amp/settings.json
fn amp_settings_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("无法获取 home 目录"))?;
    Ok(home.join(".config").join("amp").join("settings.json"))
}

/// 获取 Amp secrets.json 路径
/// macOS/Linux: ~/.local/share/amp/secrets.json
fn amp_secrets_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("无法获取 home 目录"))?;
    Ok(home
        .join(".local")
        .join("share")
        .join("amp")
        .join("secrets.json"))
}

/// 读取文件内容，不存在则返回 None
fn read_file_content(path: &PathBuf) -> Option<String> {
    if path.exists() {
        fs::read_to_string(path).ok()
    } else {
        None
    }
}

/// 写入文件，自动创建目录
fn write_file_content(path: &PathBuf, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

/// 删除文件（如果存在）
fn delete_file_if_exists(path: &PathBuf) -> Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// 读取当前 Amp 配置（完整备份）
pub fn backup_amp_config() -> Result<AmpConfigBackup> {
    let settings_path = amp_settings_path()?;
    let secrets_path = amp_secrets_path()?;

    Ok(AmpConfigBackup {
        settings_json: read_file_content(&settings_path),
        secrets_json: read_file_content(&secrets_path),
    })
}

/// 应用代理配置到 Amp（设置本地代理地址和密钥）
pub fn apply_proxy_config(proxy_url: &str, local_api_key: &str) -> Result<()> {
    let settings_path = amp_settings_path()?;
    let secrets_path = amp_secrets_path()?;

    // 读取现有配置或创建新的
    let settings_content = read_file_content(&settings_path);
    let secrets_content = read_file_content(&secrets_path);

    // 更新 settings.json
    let mut settings: Value = settings_content
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    let settings_obj = settings
        .as_object_mut()
        .ok_or_else(|| anyhow!("settings.json 格式错误"))?;
    settings_obj.insert("amp.url".to_string(), Value::String(proxy_url.to_string()));

    let new_settings = serde_json::to_string_pretty(&settings)?;
    write_file_content(&settings_path, &new_settings)?;

    // 更新 secrets.json
    let mut secrets: Value = secrets_content
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    let secrets_obj = secrets
        .as_object_mut()
        .ok_or_else(|| anyhow!("secrets.json 格式错误"))?;
    let key_name = format!("apiKey@{}", proxy_url);
    secrets_obj.insert(key_name, Value::String(local_api_key.to_string()));

    let new_secrets = serde_json::to_string_pretty(&secrets)?;
    write_file_content(&secrets_path, &new_secrets)?;

    tracing::info!(
        proxy_url = %proxy_url,
        "已应用 Amp 代理配置"
    );

    Ok(())
}

/// 完整还原 Amp 配置到原始状态
pub fn restore_amp_config(backup: &AmpConfigBackup) -> Result<()> {
    let settings_path = amp_settings_path()?;
    let secrets_path = amp_secrets_path()?;

    // 还原 settings.json
    if let Some(content) = &backup.settings_json {
        write_file_content(&settings_path, content)?;
        tracing::debug!("已还原 settings.json");
    } else {
        delete_file_if_exists(&settings_path)?;
        tracing::debug!("已删除 settings.json（原本不存在）");
    }

    // 还原 secrets.json
    if let Some(content) = &backup.secrets_json {
        write_file_content(&secrets_path, content)?;
        tracing::debug!("已还原 secrets.json");
    } else {
        delete_file_if_exists(&secrets_path)?;
        tracing::debug!("已删除 secrets.json（原本不存在）");
    }

    tracing::info!("已完整还原 Amp 配置");

    Ok(())
}
