use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};
use toml_edit::DocumentMut;

use crate::error::{AppError, AppResult};
use crate::models::{ActiveConfig, GlobalConfig};
use crate::services::{list_profiles as list_profiles_in_dir, profile_file, JsonStore, TomlStore};

fn home_dir() -> AppResult<PathBuf> {
    dirs::home_dir().ok_or_else(|| AppError::config("无法获取用户目录"))
}

fn ensure_dir(path: &Path) -> AppResult<()> {
    fs::create_dir_all(path).map_err(AppError::from)
}

#[tauri::command]
pub async fn configure_api(
    tool: String,
    _provider: String,
    api_key: String,
    base_url: Option<String>,
    profile_name: Option<String>,
) -> Result<(), String> {
    configure_api_impl(tool, api_key, base_url, profile_name).map_err(|e| e.to_string())
}

fn configure_api_impl(
    tool: String,
    api_key: String,
    base_url: Option<String>,
    profile_name: Option<String>,
) -> AppResult<()> {
    let home_dir = home_dir()?;
    let base_url_str = base_url.unwrap_or_else(|| "https://jp.duckcoding.com".to_string());

    match tool.as_str() {
        "claude-code" => update_claude_settings(&home_dir, &api_key, &base_url_str, profile_name),
        "codex" => update_codex_settings(&home_dir, &api_key, &base_url_str, profile_name),
        "gemini-cli" => update_gemini_settings(&home_dir, &api_key, &base_url_str, profile_name),
        _ => Err(AppError::config(format!("未知工具: {}", tool))),
    }
}

fn update_claude_settings(
    home_dir: &Path,
    api_key: &str,
    base_url: &str,
    profile_name: Option<String>,
) -> AppResult<()> {
    let config_dir = home_dir.join(".claude");
    ensure_dir(&config_dir)?;
    let settings_path = config_dir.join("settings.json");
    let store = JsonStore::new(&settings_path);

    store.update(|doc| {
        if !doc.is_object() {
            *doc = Value::Object(Map::new());
        }
        let obj = doc.as_object_mut().unwrap();
        let env_entry = obj
            .entry("env")
            .or_insert_with(|| Value::Object(Map::new()));
        let env_obj = env_entry
            .as_object_mut()
            .ok_or_else(|| AppError::config("env 字段必须是对象"))?;
        env_obj.insert(
            "ANTHROPIC_AUTH_TOKEN".into(),
            Value::String(api_key.to_string()),
        );
        env_obj.insert(
            "ANTHROPIC_BASE_URL".into(),
            Value::String(base_url.to_string()),
        );
        Ok(())
    })?;

    if let Some(profile) = profile_name.filter(|p| !p.is_empty()) {
        let backup_path = config_dir.join(format!("settings.{}.json", profile));
        let backup_data = json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": api_key,
                "ANTHROPIC_BASE_URL": base_url
            }
        });
        fs::write(&backup_path, serde_json::to_string_pretty(&backup_data)?)
            .map_err(AppError::from)?;
    }

    Ok(())
}

fn update_codex_settings(
    home_dir: &Path,
    api_key: &str,
    base_url: &str,
    profile_name: Option<String>,
) -> AppResult<()> {
    let config_dir = home_dir.join(".codex");
    ensure_dir(&config_dir)?;

    let config_path = config_dir.join("config.toml");
    let auth_path = config_dir.join("auth.json");

    let toml_store = TomlStore::new(&config_path);
    toml_store.update(|doc| {
        let table = doc.as_table_mut();
        table["model_provider"] = toml_edit::value("duckcoding");
        table["model"] = toml_edit::value("gpt-5-codex");
        table["model_reasoning_effort"] = toml_edit::value("high");
        table["network_access"] = toml_edit::value("enabled");
        table["disable_response_storage"] = toml_edit::value(true);

        let provider_key = if base_url.contains("duckcoding") {
            "duckcoding"
        } else {
            "custom"
        };

        let provider_base_url = if base_url.ends_with("/v1") {
            base_url.to_string()
        } else {
            format!("{}/v1", base_url)
        };

        let providers_table = table
            .entry("model_providers")
            .or_insert(toml_edit::table())
            .as_table_mut()
            .ok_or_else(|| AppError::config("model_providers 必须是 table"))?;

        let provider_entry = providers_table
            .entry(provider_key)
            .or_insert(toml_edit::table());
        let provider_table = provider_entry
            .as_table_mut()
            .ok_or_else(|| AppError::config("provider 必须是 table"))?;

        provider_table["name"] = toml_edit::value(provider_key);
        provider_table["base_url"] = toml_edit::value(provider_base_url);
        provider_table["wire_api"] = toml_edit::value("responses");
        provider_table["requires_openai_auth"] = toml_edit::value(true);

        Ok(())
    })?;

    let auth_store = JsonStore::new(&auth_path);
    auth_store.update(|doc| {
        if !doc.is_object() {
            *doc = Value::Object(Map::new());
        }
        let obj = doc.as_object_mut().unwrap();
        obj.insert("OPENAI_API_KEY".into(), Value::String(api_key.to_string()));
        Ok(())
    })?;

    if let Some(profile) = profile_name.filter(|p| !p.is_empty()) {
        let backup_config_path = config_dir.join(format!("config.{}.toml", profile));
        fs::copy(&config_path, &backup_config_path).map_err(AppError::from)?;

        let backup_auth_path = config_dir.join(format!("auth.{}.json", profile));
        fs::copy(&auth_path, &backup_auth_path).map_err(AppError::from)?;
    }

    Ok(())
}

fn update_gemini_settings(
    home_dir: &Path,
    api_key: &str,
    base_url: &str,
    profile_name: Option<String>,
) -> AppResult<()> {
    let config_dir = home_dir.join(".gemini");
    ensure_dir(&config_dir)?;

    let env_path = config_dir.join(".env");
    let mut existing_env = std::collections::BTreeMap::new();

    if env_path.exists() {
        let content = fs::read_to_string(&env_path)?;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = trimmed.split_once('=') {
                existing_env.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
    }

    existing_env.insert("GOOGLE_GEMINI_BASE_URL".into(), base_url.to_string());
    existing_env.insert("GEMINI_API_KEY".into(), api_key.to_string());
    existing_env
        .entry("GEMINI_MODEL".into())
        .or_insert_with(|| "gemini-2.5-pro".to_string());

    let env_content = existing_env
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fs::write(&env_path, env_content).map_err(AppError::from)?;

    let settings_path = config_dir.join("settings.json");
    let settings_store = JsonStore::new(&settings_path);
    settings_store.update(|doc| {
        if !doc.is_object() {
            *doc = Value::Object(Map::new());
        }
        let obj = doc.as_object_mut().unwrap();
        obj.entry("ide")
            .or_insert_with(|| json!({"enabled": true }));
        if !obj.contains_key("security") {
            obj.insert(
                "security".into(),
                json!({ "auth": { "selectedType": "gemini-api-key" } }),
            );
        }
        Ok(())
    })?;

    if let Some(profile) = profile_name.filter(|p| !p.is_empty()) {
        let backup_env_path = config_dir.join(format!(".env.{}", profile));
        let backup_content = format!(
            "GOOGLE_GEMINI_BASE_URL={}\nGEMINI_API_KEY={}\nGEMINI_MODEL=gemini-2.5-pro\n",
            base_url, api_key
        );
        fs::write(&backup_env_path, backup_content).map_err(AppError::from)?;

        let backup_settings_path = config_dir.join(format!("settings.{}.json", profile));
        let backup_settings = json!({
            "ide": { "enabled": true },
            "security": { "auth": { "selectedType": "gemini-api-key" } }
        });
        fs::write(
            &backup_settings_path,
            serde_json::to_string_pretty(&backup_settings)?,
        )
        .map_err(AppError::from)?;
    }

    Ok(())
}

#[tauri::command]
pub async fn list_profiles(tool: String) -> Result<Vec<String>, String> {
    list_profiles_impl(tool).map_err(|e| e.to_string())
}

fn list_profiles_impl(tool: String) -> AppResult<Vec<String>> {
    let home_dir = home_dir()?;
    let (dir, prefix, suffix) = match tool.as_str() {
        "claude-code" => (home_dir.join(".claude"), "settings.", ".json"),
        "codex" => (home_dir.join(".codex"), "config.", ".toml"),
        "gemini-cli" => (home_dir.join(".gemini"), ".env.", ""),
        _ => return Err(AppError::config(format!("未知工具: {}", tool))),
    };

    list_profiles_in_dir(&dir, prefix, suffix)
}

#[tauri::command]
pub async fn switch_profile(tool: String, profile: String) -> Result<(), String> {
    switch_profile_impl(tool, profile).map_err(|e| e.to_string())
}

fn switch_profile_impl(tool: String, profile: String) -> AppResult<()> {
    let home_dir = home_dir()?;

    match tool.as_str() {
        "claude-code" => {
            let config_dir = home_dir.join(".claude");
            let backup_path = profile_file(&config_dir, "settings.", &profile, ".json");
            if !backup_path.exists() {
                return Err(AppError::config(format!("找不到备份: {:?}", backup_path)));
            }

            let data = fs::read_to_string(&backup_path)?;
            let backup: Value = serde_json::from_str(&data)?;
            let active_path = config_dir.join("settings.json");
            let store = JsonStore::new(&active_path);
            store.update(|doc| {
                if !doc.is_object() {
                    *doc = Value::Object(Map::new());
                }
                let obj = doc.as_object_mut().unwrap();
                let env = backup
                    .get("env")
                    .cloned()
                    .unwrap_or_else(|| Value::Object(Map::new()));
                obj.insert("env".into(), env);
                Ok(())
            })?;
        }
        "codex" => {
            let config_dir = home_dir.join(".codex");
            let backup_config_path = profile_file(&config_dir, "config.", &profile, ".toml");
            let backup_auth_path = profile_file(&config_dir, "auth.", &profile, ".json");

            if !backup_config_path.exists() {
                return Err(AppError::config(format!(
                    "找不到备份: {:?}",
                    backup_config_path
                )));
            }

            let config_doc = fs::read_to_string(&backup_config_path)?.parse::<DocumentMut>()?;
            let active_config_path = config_dir.join("config.toml");
            TomlStore::new(&active_config_path).write(&config_doc)?;

            if backup_auth_path.exists() {
                let auth_value: Value =
                    serde_json::from_str(&fs::read_to_string(&backup_auth_path)?)?;
                JsonStore::new(config_dir.join("auth.json")).write(&auth_value)?;
            }
        }
        "gemini-cli" => {
            let config_dir = home_dir.join(".gemini");
            let backup_env_path = profile_file(&config_dir, ".env.", &profile, "");
            if !backup_env_path.exists() {
                return Err(AppError::config(format!(
                    "找不到备份: {:?}",
                    backup_env_path
                )));
            }

            let env_content = fs::read_to_string(&backup_env_path)?;
            fs::write(config_dir.join(".env"), env_content).map_err(AppError::from)?;

            let backup_settings_path = profile_file(&config_dir, "settings.", &profile, ".json");
            if backup_settings_path.exists() {
                let settings_value: Value =
                    serde_json::from_str(&fs::read_to_string(&backup_settings_path)?)?;
                JsonStore::new(config_dir.join("settings.json")).write(&settings_value)?;
            }
        }
        _ => return Err(AppError::config(format!("未知工具: {}", tool))),
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_profile(tool: String, profile: String) -> Result<(), String> {
    delete_profile_impl(tool, profile).map_err(|e| e.to_string())
}

fn delete_profile_impl(tool: String, profile: String) -> AppResult<()> {
    let home_dir = home_dir()?;
    let config_dir = match tool.as_str() {
        "claude-code" => home_dir.join(".claude"),
        "codex" => home_dir.join(".codex"),
        "gemini-cli" => home_dir.join(".gemini"),
        _ => return Err(AppError::config(format!("未知工具: {}", tool))),
    };

    let candidates = match tool.as_str() {
        "claude-code" => vec![profile_file(&config_dir, "settings.", &profile, ".json")],
        "codex" => vec![
            profile_file(&config_dir, "config.", &profile, ".toml"),
            profile_file(&config_dir, "auth.", &profile, ".json"),
        ],
        "gemini-cli" => vec![
            profile_file(&config_dir, ".env.", &profile, ""),
            profile_file(&config_dir, "settings.", &profile, ".json"),
        ],
        _ => vec![],
    };

    let mut removed = false;
    for file in candidates {
        if file.exists() {
            fs::remove_file(&file).map_err(AppError::from)?;
            removed = true;
        }
    }

    if removed {
        Ok(())
    } else {
        Err(AppError::config("未找到匹配的备份文件"))
    }
}

#[tauri::command]
pub async fn get_active_config(tool: String) -> Result<ActiveConfig, String> {
    get_active_config_impl(tool).map_err(|e| e.to_string())
}

fn get_active_config_impl(tool: String) -> AppResult<ActiveConfig> {
    let home = home_dir()?;

    match tool.as_str() {
        "claude-code" => {
            let settings = JsonStore::new(home.join(".claude/settings.json")).read()?;
            let env = settings
                .get("env")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();

            let raw_key = env
                .get("ANTHROPIC_AUTH_TOKEN")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let base_url = env
                .get("ANTHROPIC_BASE_URL")
                .and_then(|v| v.as_str())
                .unwrap_or("未配置");

            let profile_name = if !raw_key.is_empty() && base_url != "未配置" {
                detect_profile_name("claude-code", raw_key, base_url, &home)
            } else {
                None
            };

            Ok(ActiveConfig {
                api_key: if raw_key.is_empty() {
                    "未配置".into()
                } else {
                    mask_api_key(raw_key)
                },
                base_url: base_url.to_string(),
                profile_name,
            })
        }
        "codex" => {
            let auth = JsonStore::new(home.join(".codex/auth.json")).read()?;
            let raw_key = auth
                .get("OPENAI_API_KEY")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let doc = TomlStore::new(home.join(".codex/config.toml")).read()?;
            let base_url = doc
                .as_table()
                .get("model_providers")
                .and_then(|item| item.as_table())
                .and_then(|providers| {
                    providers.iter().find_map(|(_, provider)| {
                        provider.as_table().and_then(|table| {
                            table
                                .get("base_url")
                                .and_then(|item| item.as_value())
                                .and_then(|value| value.as_str())
                        })
                    })
                })
                .unwrap_or("未配置")
                .to_string();

            let profile_name = if !raw_key.is_empty() && base_url != "未配置" {
                detect_profile_name("codex", raw_key, &base_url, &home)
            } else {
                None
            };

            Ok(ActiveConfig {
                api_key: if raw_key.is_empty() {
                    "未配置".into()
                } else {
                    mask_api_key(raw_key)
                },
                base_url,
                profile_name,
            })
        }
        "gemini-cli" => {
            let env_path = home.join(".gemini/.env");
            let mut api_key = String::new();
            let mut base_url = String::from("未配置");

            if env_path.exists() {
                for line in fs::read_to_string(&env_path)?.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("GEMINI_API_KEY=") {
                        api_key = trimmed
                            .strip_prefix("GEMINI_API_KEY=")
                            .unwrap_or("")
                            .to_string();
                    } else if trimmed.starts_with("GOOGLE_GEMINI_BASE_URL=") {
                        base_url = trimmed
                            .strip_prefix("GOOGLE_GEMINI_BASE_URL=")
                            .unwrap_or("未配置")
                            .to_string();
                    }
                }
            }

            let profile_name = if !api_key.is_empty() && base_url != "未配置" {
                detect_profile_name("gemini-cli", &api_key, &base_url, &home)
            } else {
                None
            };

            Ok(ActiveConfig {
                api_key: if api_key.is_empty() {
                    "未配置".into()
                } else {
                    mask_api_key(&api_key)
                },
                base_url,
                profile_name,
            })
        }
        _ => Err(AppError::config(format!("未知工具: {}", tool))),
    }
}

#[tauri::command]
pub async fn save_global_config(user_id: String, system_token: String) -> Result<(), String> {
    save_global_config_impl(user_id, system_token).map_err(|e| e.to_string())
}

fn save_global_config_impl(user_id: String, system_token: String) -> AppResult<()> {
    let path = global_config_path()?;
    let store = JsonStore::new(&path);
    store.write(&json!({
        "user_id": user_id,
        "system_token": system_token
    }))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let metadata = std::fs::metadata(&path)?;
        let mut perms = metadata.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&path, perms)?;
    }

    Ok(())
}

#[tauri::command]
pub async fn get_global_config() -> Result<Option<GlobalConfig>, String> {
    load_global_config().map_err(|e| e.to_string())
}

fn global_config_path() -> AppResult<PathBuf> {
    let home_dir = home_dir()?;
    let config_dir = home_dir.join(".duckcoding");
    ensure_dir(&config_dir)?;
    Ok(config_dir.join("config.json"))
}

pub fn load_global_config() -> AppResult<Option<GlobalConfig>> {
    let path = global_config_path()?;
    let value = JsonStore::new(path).read()?;
    if value.is_null() || value.as_object().map(|o| o.is_empty()).unwrap_or(false) {
        Ok(None)
    } else {
        Ok(Some(serde_json::from_value(value)?))
    }
}

fn detect_profile_name(
    tool: &str,
    api_key: &str,
    base_url: &str,
    home_dir: &Path,
) -> Option<String> {
    let profiles = list_profiles_impl(tool.to_string()).ok()?;
    for profile in profiles {
        match tool {
            "claude-code" => {
                let path = home_dir
                    .join(".claude")
                    .join(format!("settings.{}.json", profile));
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(value) = serde_json::from_str::<Value>(&content) {
                        if let Some(env) = value.get("env").and_then(|v| v.as_object()) {
                            let backup_key = env
                                .get("ANTHROPIC_AUTH_TOKEN")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let backup_base = env
                                .get("ANTHROPIC_BASE_URL")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            if backup_key == api_key && backup_base == base_url {
                                return Some(profile);
                            }
                        }
                    }
                }
            }
            "codex" => {
                let backup_config = home_dir
                    .join(".codex")
                    .join(format!("config.{}.toml", profile));
                let backup_auth = home_dir
                    .join(".codex")
                    .join(format!("auth.{}.json", profile));

                if let (Ok(config_content), Ok(auth_content)) = (
                    fs::read_to_string(&backup_config),
                    fs::read_to_string(&backup_auth),
                ) {
                    if let (Ok(doc), Ok(auth)) = (
                        config_content.parse::<DocumentMut>(),
                        serde_json::from_str::<Value>(&auth_content),
                    ) {
                        let backup_key = auth
                            .get("OPENAI_API_KEY")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        let backup_base = doc
                            .as_table()
                            .get("model_providers")
                            .and_then(|item| item.as_table())
                            .and_then(|providers| {
                                providers.iter().find_map(|(_, provider)| {
                                    provider.as_table().and_then(|table| {
                                        table
                                            .get("base_url")
                                            .and_then(|item| item.as_value())
                                            .and_then(|value| value.as_str())
                                    })
                                })
                            })
                            .unwrap_or("");

                        if backup_key == api_key && backup_base == base_url {
                            return Some(profile);
                        }
                    }
                }
            }
            "gemini-cli" => {
                let backup_env = home_dir.join(".gemini").join(format!(".env.{}", profile));
                if let Ok(content) = fs::read_to_string(&backup_env) {
                    let mut backup_key = "";
                    let mut backup_base = "";
                    for line in content.lines() {
                        if let Some(val) = line.strip_prefix("GEMINI_API_KEY=") {
                            backup_key = val;
                        } else if let Some(val) = line.strip_prefix("GOOGLE_GEMINI_BASE_URL=") {
                            backup_base = val;
                        }
                    }
                    if backup_key == api_key && backup_base == base_url {
                        return Some(profile);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        "****".into()
    } else {
        format!("{}...{}", &key[..4], &key[key.len() - 4..])
    }
}
