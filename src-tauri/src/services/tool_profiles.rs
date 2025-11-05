use std::fs;
use std::path::{Path, PathBuf};

use crate::error::AppResult;

pub fn list_profiles(dir: &Path, prefix: &str, suffix: &str) -> AppResult<Vec<String>> {
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut profiles = vec![];
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = match entry.file_name().into_string() {
            Ok(name) => name,
            Err(_) => continue,
        };

        if !name.starts_with(prefix) || !name.ends_with(suffix) {
            continue;
        }

        let trimmed = name
            .strip_prefix(prefix)
            .and_then(|n| n.strip_suffix(suffix));

        if let Some(profile) = trimmed {
            profiles.push(profile.to_string());
        }
    }

    profiles.sort();
    Ok(profiles)
}

pub fn profile_file(dir: &Path, prefix: &str, profile: &str, suffix: &str) -> PathBuf {
    dir.join(format!("{}{}{}", prefix, profile, suffix))
}
