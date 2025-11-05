use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::AppResult;

pub fn backup_json(path: &Path) -> AppResult<Option<PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }

    create_backup_with_validator(path, |backup| {
        let data = fs::read(backup)?;
        let _: serde_json::Value = serde_json::from_slice(&data)?;
        Ok(())
    })
    .map(Some)
}

pub fn backup_toml(path: &Path) -> AppResult<Option<PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }

    create_backup_with_validator(path, |backup| {
        let content = fs::read_to_string(backup)?;
        let _: toml_edit::DocumentMut = content.parse()?;
        Ok(())
    })
    .map(Some)
}

fn create_backup_with_validator<F>(source: &Path, validator: F) -> AppResult<PathBuf>
where
    F: Fn(&Path) -> AppResult<()>,
{
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let file_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config");

    let backup_name = format!("{}.{}.bak", file_name, timestamp);
    let backup_path = source.with_file_name(backup_name);

    if let Some(parent) = backup_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(source, &backup_path)?;
    validator(&backup_path)?;

    Ok(backup_path)
}
