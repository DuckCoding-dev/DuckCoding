// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, Runtime, AppHandle, State,
};
use std::process::Command;
use std::env;
use std::fs;
use serde_json::{Value, Map};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

// 全局状态：CLI目录路径
struct CliDir(Mutex<PathBuf>);

// 辅助函数：获取扩展的PATH环境变量
fn get_extended_path() -> String {
    let home_dir = env::var("HOME").unwrap_or_else(|_| "/Users/wufeifan".to_string());

    let mut system_paths = vec![
        // Claude Code 可能的安装路径
        format!("{}/.local/bin", home_dir),
        format!("{}/.claude/bin", home_dir),
        format!("{}/.claude/local", home_dir),  // Claude Code local安装

        // Homebrew
        "/opt/homebrew/bin".to_string(),
        "/usr/local/bin".to_string(),

        // 系统路径
        "/usr/bin".to_string(),
        "/bin".to_string(),
        "/usr/sbin".to_string(),
        "/sbin".to_string(),
    ];

    // 添加nvm路径（如果存在）
    let nvm_dir = format!("{}/.nvm/versions/node", home_dir);
    if let Ok(entries) = fs::read_dir(&nvm_dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let bin_path = entry.path().join("bin");
                    if bin_path.exists() {
                        system_paths.push(bin_path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    format!("{}:{}", system_paths.join(":"), env::var("PATH").unwrap_or_default())
}

// 辅助函数：获取CLI工作目录
fn get_cli_dir(app_handle: &AppHandle) -> Result<PathBuf, String> {
    // 尝试获取资源目录（打包后）
    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        println!("Resource dir: {:?}", resource_dir);

        // Tauri 2.0在macOS上会把资源放在_up_子目录
        let up_dir = resource_dir.join("_up_");
        if up_dir.join("cli.js").exists() {
            println!("Found cli.js in _up_ dir: {:?}", up_dir);
            return Ok(up_dir);
        }

        // 检查cli.js是否直接在资源目录
        let cli_path = resource_dir.join("cli.js");
        if cli_path.exists() {
            println!("Found cli.js in resource dir: {:?}", cli_path);
            return Ok(resource_dir);
        }
    }

    // 开发环境：使用当前目录
    let current = env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    println!("Using current dir: {:?}", current);
    Ok(current)
}

//定义 Tauri Commands
#[tauri::command]
async fn check_installations() -> Result<Vec<ToolStatus>, String> {
    let mut tools = vec![
        ToolStatus {
            id: "claude-code".to_string(),
            name: "Claude Code".to_string(),
            installed: false,
            version: None,
        },
        ToolStatus {
            id: "codex".to_string(),
            name: "CodeX".to_string(),
            installed: false,
            version: None,
        },
        ToolStatus {
            id: "gemini-cli".to_string(),
            name: "Gemini CLI".to_string(),
            installed: false,
            version: None,
        },
    ];

    // 跨平台命令执行辅助函数
    let run_command = |cmd: &str| -> Result<std::process::Output, std::io::Error> {
        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .env("PATH", get_extended_path())
                .arg("/C")
                .arg(cmd)
                .output()
        }

        #[cfg(not(target_os = "windows"))]
        {
            Command::new("sh")
                .env("PATH", get_extended_path())
                .arg("-c")
                .arg(cmd)
                .output()
        }
    };

    // 检测 Claude Code
    if let Ok(output) = run_command("claude --version 2>&1") {
        if output.status.success() {
            if let Some(tool) = tools.iter_mut().find(|t| t.id == "claude-code") {
                tool.installed = true;
                tool.version = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
        }
    }

    // 检测 CodeX
    if let Ok(output) = run_command("codex --version 2>&1") {
        if output.status.success() {
            if let Some(tool) = tools.iter_mut().find(|t| t.id == "codex") {
                tool.installed = true;
                tool.version = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
        }
    }

    // 检测 Gemini CLI
    if let Ok(output) = run_command("gemini --version 2>&1") {
        if output.status.success() {
            if let Some(tool) = tools.iter_mut().find(|t| t.id == "gemini-cli") {
                tool.installed = true;
                tool.version = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
        }
    }

    Ok(tools)
}

#[tauri::command]
async fn install_tool(tool: String, method: String, cli_dir: State<'_, CliDir>) -> Result<InstallResult, String> {
    let cli_path = cli_dir.0.lock().unwrap().clone();

    println!("Installing {} via {} in {:?}", tool, method, cli_path);

    // 调用 Node.js CLI 安装工具
    let output = Command::new("node")
        .env("PATH", get_extended_path())
        .current_dir(&cli_path)
        .arg("cli.js")
        .arg("install")
        .arg(&tool)
        .arg("--method")
        .arg(&method)
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    println!("Install output: {}", String::from_utf8_lossy(&output.stdout));
    println!("Install stderr: {}", String::from_utf8_lossy(&output.stderr));

    if output.status.success() {
        Ok(InstallResult {
            success: true,
            message: format!("{} installed successfully via {}", tool, method),
            output: String::from_utf8_lossy(&output.stdout).to_string(),
        })
    } else {
        Err(format!(
            "Installation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

// 只检查更新，不执行
#[tauri::command]
async fn check_update(tool: String) -> Result<UpdateResult, String> {
    println!("Checking updates for {}", tool);

    // 根据工具类型获取 npm 包名
    let package_name = match tool.as_str() {
        "claude-code" => "@anthropic-ai/claude-code",
        "codex" => "@openai/codex",
        "gemini-cli" => "@google/gemini-cli",
        _ => {
            return Err(format!("Unknown tool: {}", tool));
        }
    };

    // 获取当前安装的版本
    let check_output = Command::new("node")
        .env("PATH", get_extended_path())
        .arg("cli.js")
        .arg("check")
        .output();

    let current_version = if let Ok(output) = check_output {
        let version_str = String::from_utf8_lossy(&output.stdout);
        extract_version_for_tool(&version_str, &tool)
    } else {
        None
    };

    // 检查最新版本
    let npm_output = Command::new("npm")
        .env("PATH", get_extended_path())
        .arg("show")
        .arg(package_name)
        .arg("version")
        .output()
        .map_err(|e| format!("Failed to check npm version: {}", e))?;

    if npm_output.status.success() {
        let latest_version_str = String::from_utf8_lossy(&npm_output.stdout).trim().to_string();

        // 比较版本
        let has_update = if let Some(ref current) = current_version {
            compare_versions(current, &latest_version_str)
        } else {
            false
        };

        return Ok(UpdateResult {
            success: true,
            message: "检查完成".to_string(),
            has_update,
            current_version,
            latest_version: Some(latest_version_str),
        });
    }

    Ok(UpdateResult {
        success: true,
        message: "无法检查更新".to_string(),
        has_update: false,
        current_version,
        latest_version: None,
    })
}

#[tauri::command]
async fn update_tool(tool: String, cli_dir: State<'_, CliDir>) -> Result<UpdateResult, String> {
    println!("Updating {}", tool);

    // 根据工具类型获取更新命令
    let (update_command, update_args) = match tool.as_str() {
        "claude-code" => {
            // Claude Code 使用 npm 更新（跨平台）
            ("npm", vec!["install", "-g", "@anthropic-ai/claude-code@latest"])
        },
        "codex" => {
            // CodeX 检测安装方式（仅 macOS 检查 brew）
            #[cfg(target_os = "macos")]
            {
                // macOS: 检查是否是 brew 安装的
                let check_which = Command::new("which")
                    .env("PATH", get_extended_path())
                    .arg("codex")
                    .output();

                if let Ok(output) = check_which {
                    let path = String::from_utf8_lossy(&output.stdout);
                    if path.contains("/opt/homebrew/") || path.contains("/usr/local/") {
                        // brew 安装的，使用 brew 更新
                        ("brew", vec!["upgrade", "codex"])
                    } else {
                        // npm 安装的
                        ("npm", vec!["install", "-g", "@openai/codex@latest"])
                    }
                } else {
                    // 默认用 npm
                    ("npm", vec!["install", "-g", "@openai/codex@latest"])
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                // Windows 和 Linux 统一用 npm
                ("npm", vec!["install", "-g", "@openai/codex@latest"])
            }
        },
        "gemini-cli" => {
            // Gemini CLI 使用 npm 更新（跨平台）
            ("npm", vec!["install", "-g", "@google/gemini-cli@latest"])
        },
        _ => {
            return Err(format!("Unknown tool: {}", tool));
        }
    };

    // 执行更新
    println!("Executing update: {} {:?}", update_command, update_args);
    let output = Command::new(update_command)
        .env("PATH", get_extended_path())
        .args(&update_args)
        .output()
        .map_err(|e| format!("Failed to execute update: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("Update stdout: {}", stdout);
    println!("Update stderr: {}", stderr);

    if output.status.success() {
        // 获取更新后的版本
        let check_output = Command::new("node")
            .env("PATH", get_extended_path())
            .arg("cli.js")
            .arg("check")
            .output();

        let new_version = if let Ok(check) = check_output {
            let version_str = String::from_utf8_lossy(&check.stdout);
            extract_version_for_tool(&version_str, &tool)
        } else {
            None
        };

        Ok(UpdateResult {
            success: true,
            message: "更新成功".to_string(),
            has_update: false,
            current_version: new_version.clone(),
            latest_version: new_version,
        })
    } else {
        Err(format!("更新失败: {}", stderr))
    }
}

// 从 check 命令输出中提取特定工具的版本号
fn extract_version_for_tool(text: &str, tool: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();

    // 找到工具对应的行
    for (i, line) in lines.iter().enumerate() {
        if (tool == "claude-code" && line.contains("Claude Code")) ||
           (tool == "codex" && line.contains("CodeX")) ||
           (tool == "gemini-cli" && line.contains("Gemini CLI")) {
            // 查找下一行的版本号
            if let Some(next_line) = lines.get(i + 1) {
                if next_line.contains("版本:") {
                    return extract_version(next_line);
                }
            }
        }
    }
    None
}

// 从字符串中提取版本号
fn extract_version(text: &str) -> Option<String> {
    // 匹配类似 "1.2.3" 的版本号
    let re = regex::Regex::new(r"(\d+\.\d+\.\d+)").ok()?;
    re.captures(text)?.get(1).map(|m| m.as_str().to_string())
}

// 比较版本号 (简单比较，返回 true 如果 latest > current)
fn compare_versions(current: &str, latest: &str) -> bool {
    let current_parts: Vec<u32> = current.split('.').filter_map(|s| s.parse().ok()).collect();
    let latest_parts: Vec<u32> = latest.split('.').filter_map(|s| s.parse().ok()).collect();

    for i in 0..3 {
        let c = current_parts.get(i).copied().unwrap_or(0);
        let l = latest_parts.get(i).copied().unwrap_or(0);

        if l > c {
            return true;
        } else if l < c {
            return false;
        }
    }

    false
}

#[tauri::command]
async fn configure_api(tool: String, provider: String, api_key: String, base_url: Option<String>, profile_name: Option<String>, cli_dir: State<'_, CliDir>) -> Result<(), String> {
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;
    let base_url_str = base_url.unwrap_or_else(|| "https://jp.duckcoding.com".to_string());
    let cli_path = cli_dir.0.lock().unwrap().clone();

    match tool.as_str() {
        "claude-code" => {
            let config_dir = home_dir.join(".claude");
            let config_path = config_dir.join("settings.json");

            // 确保目录存在
            fs::create_dir_all(&config_dir).map_err(|e| format!("Failed to create directory: {}", e))?;

            // 读取现有配置
            let mut config: Value = if config_path.exists() {
                let content = fs::read_to_string(&config_path)
                    .map_err(|e| format!("Failed to read config: {}", e))?;
                serde_json::from_str(&content).unwrap_or(Value::Object(Map::new()))
            } else {
                Value::Object(Map::new())
            };

            // 确保有 env 对象
            if !config.is_object() {
                config = Value::Object(Map::new());
            }
            let config_obj = config.as_object_mut().unwrap();
            if !config_obj.contains_key("env") {
                config_obj.insert("env".to_string(), Value::Object(Map::new()));
            }

            // 更新 API 配置
            let env_obj = config_obj.get_mut("env").unwrap().as_object_mut().unwrap();
            env_obj.insert("ANTHROPIC_AUTH_TOKEN".to_string(), Value::String(api_key.clone()));
            env_obj.insert("ANTHROPIC_BASE_URL".to_string(), Value::String(base_url_str.clone()));

            // 写入配置
            fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap())
                .map_err(|e| format!("Failed to write config: {}", e))?;

            // 如果有 profile_name，保存备份
            if let Some(profile) = profile_name {
                if !profile.is_empty() {
                    let backup_path = config_dir.join(format!("settings.{}.json", profile));
                    let backup_config = serde_json::json!({
                        "env": {
                            "ANTHROPIC_AUTH_TOKEN": api_key,
                            "ANTHROPIC_BASE_URL": base_url_str
                        }
                    });
                    fs::write(&backup_path, serde_json::to_string_pretty(&backup_config).unwrap())
                        .map_err(|e| format!("Failed to write backup: {}", e))?;
                }
            }
        },
        "codex" => {
            // 使用 CLI 处理 CodeX 配置（因为涉及 TOML）

            let mut cmd = Command::new("node");
            cmd.env("PATH", get_extended_path());
            cmd.current_dir(&cli_path)
                .arg("cli.js")
                .arg("config")
                .arg(&tool)
                .arg("-k")
                .arg(&api_key)
                .arg("-p")
                .arg(&provider)
                .arg("-u")
                .arg(&base_url_str);

            if let Some(profile) = profile_name {
                if !profile.is_empty() {
                    cmd.arg("-n").arg(&profile);
                }
            }

            let output = cmd.output()
                .map_err(|e| format!("Failed to execute command: {}", e))?;

            if !output.status.success() {
                return Err(format!(
                    "Configuration failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        },
        "gemini-cli" => {
            let config_dir = home_dir.join(".gemini");
            let env_path = config_dir.join(".env");

            // 确保目录存在
            fs::create_dir_all(&config_dir).map_err(|e| format!("Failed to create directory: {}", e))?;

            // 读取现有 .env 文件
            let mut env_vars = std::collections::HashMap::new();
            if env_path.exists() {
                let content = fs::read_to_string(&env_path)
                    .map_err(|e| format!("Failed to read .env: {}", e))?;
                for line in content.lines() {
                    let line = line.trim();
                    if !line.is_empty() && !line.starts_with('#') {
                        if let Some((key, value)) = line.split_once('=') {
                            env_vars.insert(key.trim().to_string(), value.trim().to_string());
                        }
                    }
                }
            }

            // 更新 API 相关的环境变量
            env_vars.insert("GOOGLE_GEMINI_BASE_URL".to_string(), base_url_str.clone());
            env_vars.insert("GEMINI_API_KEY".to_string(), api_key.clone());
            if !env_vars.contains_key("GEMINI_MODEL") {
                env_vars.insert("GEMINI_MODEL".to_string(), "gemini-2.5-pro".to_string());
            }

            // 写入 .env 文件
            let env_content: String = env_vars.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("\n") + "\n";

            fs::write(&env_path, env_content)
                .map_err(|e| format!("Failed to write .env: {}", e))?;

            // 如果有 profile_name，保存备份
            if let Some(profile) = profile_name {
                if !profile.is_empty() {
                    let backup_env_path = config_dir.join(format!(".env.{}", profile));
                    let backup_content = format!(
                        "GOOGLE_GEMINI_BASE_URL={}\nGEMINI_API_KEY={}\nGEMINI_MODEL=gemini-2.5-pro\n",
                        base_url_str, api_key
                    );
                    fs::write(&backup_env_path, backup_content)
                        .map_err(|e| format!("Failed to write backup .env: {}", e))?;
                }
            }
        },
        _ => return Err(format!("Unknown tool: {}", tool))
    }

    Ok(())
}

#[tauri::command]
async fn list_profiles(tool: String, cli_dir: State<'_, CliDir>) -> Result<Vec<String>, String> {
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;
    let mut profiles = Vec::new();

    match tool.as_str() {
        "claude-code" => {
            let config_dir = home_dir.join(".claude");
            if !config_dir.exists() {
                return Ok(profiles);
            }

            // 查找 settings.{profile}.json 文件
            if let Ok(entries) = fs::read_dir(&config_dir) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name();
                    let file_name_str = file_name.to_string_lossy();

                    // 匹配 settings.{profile}.json 格式
                    if file_name_str.starts_with("settings.") && file_name_str.ends_with(".json") {
                        let profile_name = file_name_str
                            .strip_prefix("settings.")
                            .and_then(|s| s.strip_suffix(".json"));
                        if let Some(name) = profile_name {
                            profiles.push(name.to_string());
                        }
                    }
                }
            }
        },
        "codex" => {
            let config_dir = home_dir.join(".codex");
            if !config_dir.exists() {
                return Ok(profiles);
            }

            // 查找 config.{profile}.toml 文件
            if let Ok(entries) = fs::read_dir(&config_dir) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name();
                    let file_name_str = file_name.to_string_lossy();

                    // 匹配 config.{profile}.toml 格式
                    if file_name_str.starts_with("config.") && file_name_str.ends_with(".toml") {
                        let profile_name = file_name_str
                            .strip_prefix("config.")
                            .and_then(|s| s.strip_suffix(".toml"));
                        if let Some(name) = profile_name {
                            profiles.push(name.to_string());
                        }
                    }
                }
            }
        },
        "gemini-cli" => {
            let config_dir = home_dir.join(".gemini");
            if !config_dir.exists() {
                return Ok(profiles);
            }

            // 查找 .env.{profile} 文件
            if let Ok(entries) = fs::read_dir(&config_dir) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name();
                    let file_name_str = file_name.to_string_lossy();

                    // 匹配 .env.{profile} 格式
                    if file_name_str.starts_with(".env.") {
                        let profile_name = file_name_str.strip_prefix(".env.");
                        if let Some(name) = profile_name {
                            profiles.push(name.to_string());
                        }
                    }
                }
            }
        },
        _ => return Err(format!("Unknown tool: {}", tool))
    }

    Ok(profiles)
}

#[tauri::command]
async fn switch_profile(tool: String, profile: String) -> Result<(), String> {
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;

    match tool.as_str() {
        "claude-code" => {
            let config_dir = home_dir.join(".claude");
            let backup_path = config_dir.join(format!("settings.{}.json", profile));
            let active_path = config_dir.join("settings.json");

            if !backup_path.exists() {
                return Err(format!("Backup config not found: {:?}", backup_path));
            }

            // 读取备份配置
            let backup_content = fs::read_to_string(&backup_path)
                .map_err(|e| format!("Failed to read backup config: {}", e))?;
            let backup_config: Value = serde_json::from_str(&backup_content)
                .map_err(|e| format!("Failed to parse backup config: {}", e))?;

            // 读取当前配置
            let mut active_config: Value = if active_path.exists() {
                let content = fs::read_to_string(&active_path)
                    .map_err(|e| format!("Failed to read active config: {}", e))?;
                serde_json::from_str(&content).unwrap_or(Value::Object(Map::new()))
            } else {
                Value::Object(Map::new())
            };

            // 合并配置：只更新 API 相关字段
            if let Some(backup_env) = backup_config.get("env") {
                if !active_config.is_object() {
                    active_config = Value::Object(Map::new());
                }
                let active_obj = active_config.as_object_mut().unwrap();
                if !active_obj.contains_key("env") {
                    active_obj.insert("env".to_string(), Value::Object(Map::new()));
                }

                let active_env = active_obj.get_mut("env").unwrap().as_object_mut().unwrap();
                if let Some(backup_env_obj) = backup_env.as_object() {
                    if let Some(token) = backup_env_obj.get("ANTHROPIC_AUTH_TOKEN") {
                        active_env.insert("ANTHROPIC_AUTH_TOKEN".to_string(), token.clone());
                    }
                    if let Some(base_url) = backup_env_obj.get("ANTHROPIC_BASE_URL") {
                        active_env.insert("ANTHROPIC_BASE_URL".to_string(), base_url.clone());
                    }
                }
            }

            // 写入配置
            fs::write(&active_path, serde_json::to_string_pretty(&active_config).unwrap())
                .map_err(|e| format!("Failed to write active config: {}", e))?;
        },
        "codex" => {
            let config_dir = home_dir.join(".codex");
            let backup_config_path = config_dir.join(format!("config.{}.toml", profile));
            let active_config_path = config_dir.join("config.toml");
            let backup_auth_path = config_dir.join(format!("auth.{}.json", profile));
            let active_auth_path = config_dir.join("auth.json");

            if !backup_config_path.exists() {
                return Err(format!("Backup config not found: {:?}", backup_config_path));
            }

            // 读取备份的 config.toml
            let backup_config_content = fs::read_to_string(&backup_config_path)
                .map_err(|e| format!("Failed to read backup config: {}", e))?;
            let backup_config: toml::Value = toml::from_str(&backup_config_content)
                .map_err(|e| format!("Failed to parse backup TOML: {}", e))?;

            // 读取当前的 config.toml
            let mut active_config: toml::Value = if active_config_path.exists() {
                let content = fs::read_to_string(&active_config_path)
                    .map_err(|e| format!("Failed to read active config: {}", e))?;
                toml::from_str(&content).unwrap_or(toml::Value::Table(toml::map::Map::new()))
            } else {
                toml::Value::Table(toml::map::Map::new())
            };

            // 合并配置：只更新必要字段
            if let toml::Value::Table(ref backup_table) = backup_config {
                if let toml::Value::Table(ref mut active_table) = active_config {
                    // 更新顶层字段
                    if let Some(provider) = backup_table.get("model_provider") {
                        active_table.insert("model_provider".to_string(), provider.clone());
                    }
                    if let Some(model) = backup_table.get("model") {
                        active_table.insert("model".to_string(), model.clone());
                    }
                    if let Some(effort) = backup_table.get("model_reasoning_effort") {
                        active_table.insert("model_reasoning_effort".to_string(), effort.clone());
                    }
                    if let Some(network) = backup_table.get("network_access") {
                        active_table.insert("network_access".to_string(), network.clone());
                    }
                    if let Some(storage) = backup_table.get("disable_response_storage") {
                        active_table.insert("disable_response_storage".to_string(), storage.clone());
                    }

                    // 更新 model_providers
                    if let Some(backup_providers) = backup_table.get("model_providers") {
                        if !active_table.contains_key("model_providers") {
                            active_table.insert("model_providers".to_string(), toml::Value::Table(toml::map::Map::new()));
                        }
                        if let Some(toml::Value::Table(active_providers)) = active_table.get_mut("model_providers") {
                            if let toml::Value::Table(bp) = backup_providers {
                                for (key, value) in bp {
                                    active_providers.insert(key.clone(), value.clone());
                                }
                            }
                        }
                    }
                }
            }

            // 写入 config.toml
            let toml_string = toml::to_string_pretty(&active_config)
                .map_err(|e| format!("Failed to serialize TOML: {}", e))?;
            fs::write(&active_config_path, toml_string)
                .map_err(|e| format!("Failed to write active config: {}", e))?;

            // 更新 auth.json
            if backup_auth_path.exists() {
                let backup_auth_content = fs::read_to_string(&backup_auth_path)
                    .map_err(|e| format!("Failed to read backup auth: {}", e))?;
                let backup_auth: Value = serde_json::from_str(&backup_auth_content)
                    .map_err(|e| format!("Failed to parse backup auth: {}", e))?;

                let mut active_auth: Value = if active_auth_path.exists() {
                    let content = fs::read_to_string(&active_auth_path)
                        .map_err(|e| format!("Failed to read active auth: {}", e))?;
                    serde_json::from_str(&content).unwrap_or(Value::Object(Map::new()))
                } else {
                    Value::Object(Map::new())
                };

                if let Some(backup_key) = backup_auth.get("OPENAI_API_KEY") {
                    if let Value::Object(ref mut active_obj) = active_auth {
                        active_obj.insert("OPENAI_API_KEY".to_string(), backup_key.clone());
                    }
                }

                fs::write(&active_auth_path, serde_json::to_string_pretty(&active_auth).unwrap())
                    .map_err(|e| format!("Failed to write active auth: {}", e))?;
            }
        },
        "gemini-cli" => {
            let config_dir = home_dir.join(".gemini");
            let backup_env_path = config_dir.join(format!(".env.{}", profile));
            let active_env_path = config_dir.join(".env");

            if !backup_env_path.exists() {
                return Err(format!("Backup .env not found: {:?}", backup_env_path));
            }

            // 读取备份的环境变量
            let backup_content = fs::read_to_string(&backup_env_path)
                .map_err(|e| format!("Failed to read backup .env: {}", e))?;
            let mut backup_env = std::collections::HashMap::new();
            for line in backup_content.lines() {
                let line = line.trim();
                if !line.is_empty() && !line.starts_with('#') {
                    if let Some((key, value)) = line.split_once('=') {
                        backup_env.insert(key.trim().to_string(), value.trim().to_string());
                    }
                }
            }

            // 读取当前的环境变量
            let mut active_env = std::collections::HashMap::new();
            if active_env_path.exists() {
                let content = fs::read_to_string(&active_env_path)
                    .map_err(|e| format!("Failed to read active .env: {}", e))?;
                for line in content.lines() {
                    let line = line.trim();
                    if !line.is_empty() && !line.starts_with('#') {
                        if let Some((key, value)) = line.split_once('=') {
                            active_env.insert(key.trim().to_string(), value.trim().to_string());
                        }
                    }
                }
            }

            // 合并：只更新 API 相关字段
            if let Some(base_url) = backup_env.get("GOOGLE_GEMINI_BASE_URL") {
                active_env.insert("GOOGLE_GEMINI_BASE_URL".to_string(), base_url.clone());
            }
            if let Some(api_key) = backup_env.get("GEMINI_API_KEY") {
                active_env.insert("GEMINI_API_KEY".to_string(), api_key.clone());
            }
            if let Some(model) = backup_env.get("GEMINI_MODEL") {
                active_env.insert("GEMINI_MODEL".to_string(), model.clone());
            }

            // 写回 .env
            let env_content: String = active_env.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("\n") + "\n";

            fs::write(&active_env_path, env_content)
                .map_err(|e| format!("Failed to write active .env: {}", e))?;
        },
        _ => return Err(format!("Unknown tool: {}", tool))
    }

    Ok(())
}

// 数据结构定义
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct ToolStatus {
    id: String,
    name: String,
    installed: bool,
    version: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct InstallResult {
    success: bool,
    message: String,
    output: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct UpdateResult {
    success: bool,
    message: String,
    has_update: bool,
    current_version: Option<String>,
    latest_version: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ActiveConfig {
    api_key: String,
    base_url: String,
}

// 全局配置结构
#[derive(Serialize, Deserialize, Clone)]
struct GlobalConfig {
    user_id: String,
    system_token: String,
}

// DuckCoding API 响应结构
#[derive(Deserialize, Debug)]
struct TokenData {
    id: i64,
    key: String,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    group: String,
}

#[derive(Deserialize, Debug)]
struct ApiResponse {
    success: bool,
    message: String,
    data: Option<Vec<TokenData>>,
}

#[derive(Serialize)]
struct GenerateApiKeyResult {
    success: bool,
    message: String,
    api_key: Option<String>,
}

// 用量统计数据结构
#[derive(Deserialize, Serialize, Debug, Clone)]
struct UsageData {
    id: i64,
    user_id: i64,
    username: String,
    model_name: String,
    created_at: i64,
    token_used: i64,
    count: i64,
    quota: i64,
}

#[derive(Deserialize, Debug)]
struct UsageApiResponse {
    success: bool,
    message: String,
    data: Option<Vec<UsageData>>,
}

#[derive(Serialize)]
struct UsageStatsResult {
    success: bool,
    message: String,
    data: Vec<UsageData>,
}

// 用户信息数据结构
#[derive(Deserialize, Serialize, Debug)]
struct UserInfo {
    id: i64,
    username: String,
    quota: i64,
    used_quota: i64,
    request_count: i64,
}

#[derive(Deserialize, Debug)]
struct UserApiResponse {
    success: bool,
    message: String,
    data: Option<UserInfo>,
}

#[derive(Serialize)]
struct UserQuotaResult {
    success: bool,
    message: String,
    total_quota: f64,
    used_quota: f64,
    remaining_quota: f64,
    request_count: i64,
}

// 全局配置辅助函数
fn get_global_config_path() -> Result<PathBuf, String> {
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;
    let config_dir = home_dir.join(".duckcoding");
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    Ok(config_dir.join("config.json"))
}

// Tauri命令：保存全局配置
#[tauri::command]
async fn save_global_config(user_id: String, system_token: String) -> Result<(), String> {
    let config = GlobalConfig { user_id, system_token };
    let config_path = get_global_config_path()?;

    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&config_path, json)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    // 设置文件权限为仅所有者可读写（Unix系统）
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(&config_path)
            .map_err(|e| format!("Failed to get file metadata: {}", e))?;
        let mut perms = metadata.permissions();
        perms.set_mode(0o600);  // -rw-------
        fs::set_permissions(&config_path, perms)
            .map_err(|e| format!("Failed to set file permissions: {}", e))?;
    }

    Ok(())
}

// Tauri命令：读取全局配置
#[tauri::command]
async fn get_global_config() -> Result<Option<GlobalConfig>, String> {
    let config_path = get_global_config_path()?;

    if !config_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;

    let config: GlobalConfig = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    Ok(Some(config))
}

// 生成API Key的主函数
#[tauri::command]
async fn generate_api_key_for_tool(tool: String) -> Result<GenerateApiKeyResult, String> {
    // 读取全局配置
    let global_config = get_global_config().await?
        .ok_or("请先配置用户ID和系统访问令牌")?;

    // 根据工具名称获取配置
    let (name, group) = match tool.as_str() {
        "claude-code" => ("Claude Code一键创建", "Claude Code专用"),
        "codex" => ("CodeX一键创建", "CodeX专用"),
        "gemini-cli" => ("Gemini CLI一键创建", "Gemini CLI专用"),
        _ => return Err(format!("Unknown tool: {}", tool)),
    };

    // 创建token
    let client = reqwest::Client::new();
    let create_url = "https://duckcoding.com/api/token";

    let create_body = serde_json::json!({
        "remain_quota": 500000,
        "expired_time": -1,
        "unlimited_quota": true,
        "model_limits_enabled": false,
        "model_limits": "",
        "name": name,
        "group": group,
        "allow_ips": ""
    });

    let create_response = client
        .post(create_url)
        .header("Authorization", format!("Bearer {}", global_config.system_token))
        .header("New-Api-User", &global_config.user_id)
        .header("Content-Type", "application/json")
        .json(&create_body)
        .send()
        .await
        .map_err(|e| format!("创建token失败: {}", e))?;

    if !create_response.status().is_success() {
        let status = create_response.status();
        let error_text = create_response.text().await.unwrap_or_default();
        return Ok(GenerateApiKeyResult {
            success: false,
            message: format!("创建token失败 ({}): {}", status, error_text),
            api_key: None,
        });
    }

    // 等待一小段时间让服务器处理
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // 搜索刚创建的token
    let search_url = format!("https://duckcoding.com/api/token/search?keyword={}",
        urlencoding::encode(name));

    let search_response = client
        .get(&search_url)
        .header("Authorization", format!("Bearer {}", global_config.system_token))
        .header("New-Api-User", &global_config.user_id)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("搜索token失败: {}", e))?;

    if !search_response.status().is_success() {
        return Ok(GenerateApiKeyResult {
            success: false,
            message: "创建成功但获取API Key失败，请稍后在DuckCoding控制台查看".to_string(),
            api_key: None,
        });
    }

    let api_response: ApiResponse = search_response
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    if !api_response.success {
        return Ok(GenerateApiKeyResult {
            success: false,
            message: format!("API返回错误: {}", api_response.message),
            api_key: None,
        });
    }

    // 获取id最大的token（最新创建的）
    if let Some(mut data) = api_response.data {
        if !data.is_empty() {
            // 按id降序排序，取第一个（id最大的）
            data.sort_by(|a, b| b.id.cmp(&a.id));
            let token = &data[0];
            let api_key = format!("sk-{}", token.key);
            return Ok(GenerateApiKeyResult {
                success: true,
                message: "API Key生成成功".to_string(),
                api_key: Some(api_key),
            });
        }
    }

    Ok(GenerateApiKeyResult {
        success: false,
        message: "未找到生成的token".to_string(),
        api_key: None,
    })
}

// 获取用户用量统计（近30天）
#[tauri::command]
async fn get_usage_stats() -> Result<UsageStatsResult, String> {
    // 读取全局配置
    let global_config = get_global_config().await?
        .ok_or("请先配置用户ID和系统访问令牌")?;

    // 计算时间戳（北京时间）
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // 今天的24:00:00（加上8小时时区偏移，然后取第二天的0点）
    let beijing_offset = 8 * 3600;
    let today_end = (now + beijing_offset) / 86400 * 86400 + 86400 - beijing_offset;

    // 30天前的00:00:00
    let start_timestamp = today_end - 30 * 86400;
    let end_timestamp = today_end;

    // 调用API
    let client = reqwest::Client::new();
    let url = format!(
        "https://duckcoding.com/api/data/self?start_timestamp={}&end_timestamp={}",
        start_timestamp, end_timestamp
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", global_config.system_token))
        .header("New-Api-User", &global_config.user_id)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("获取用量统计失败: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Ok(UsageStatsResult {
            success: false,
            message: format!("获取用量统计失败 ({}): {}", status, error_text),
            data: vec![],
        });
    }

    let api_response: UsageApiResponse = response
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    if !api_response.success {
        return Ok(UsageStatsResult {
            success: false,
            message: format!("API返回错误: {}", api_response.message),
            data: vec![],
        });
    }

    Ok(UsageStatsResult {
        success: true,
        message: "获取成功".to_string(),
        data: api_response.data.unwrap_or_default(),
    })
}

// 获取用户额度信息
#[tauri::command]
async fn get_user_quota() -> Result<UserQuotaResult, String> {
    // 读取全局配置
    let global_config = get_global_config().await?
        .ok_or("请先配置用户ID和系统访问令牌")?;

    // 调用API
    let client = reqwest::Client::new();
    let url = "https://duckcoding.com/api/user/self";

    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {}", global_config.system_token))
        .header("New-Api-User", &global_config.user_id)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("获取用户信息失败: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("获取用户信息失败 ({}): {}", status, error_text));
    }

    let api_response: UserApiResponse = response
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    if !api_response.success {
        return Err(format!("API返回错误: {}", api_response.message));
    }

    let user_info = api_response.data.ok_or("未获取到用户信息")?;

    // 修正：API返回的quota是剩余额度，不是总额度
    // 正确计算：总额度 = 剩余额度 + 已用额度
    let remaining_quota = user_info.quota as f64 / 500000.0;
    let used_quota = user_info.used_quota as f64 / 500000.0;
    let total_quota = remaining_quota + used_quota;

    #[cfg(debug_assertions)]
    {
        println!("Raw remaining: {}, converted: {}", user_info.quota, remaining_quota);
        println!("Raw used: {}, converted: {}", user_info.used_quota, used_quota);
        println!("Total quota: {}", total_quota);
    }

    Ok(UserQuotaResult {
        success: true,
        message: "获取成功".to_string(),
        total_quota,
        used_quota,
        remaining_quota,
        request_count: user_info.request_count,
    })
}

#[tauri::command]
async fn get_active_config(tool: String) -> Result<ActiveConfig, String> {
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;

    match tool.as_str() {
        "claude-code" => {
            let config_path = home_dir.join(".claude").join("settings.json");
            if !config_path.exists() {
                return Ok(ActiveConfig {
                    api_key: "未配置".to_string(),
                    base_url: "未配置".to_string(),
                });
            }

            let content = fs::read_to_string(&config_path)
                .map_err(|e| format!("Failed to read config: {}", e))?;
            let config: Value = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse config: {}", e))?;

            let api_key = config.get("env")
                .and_then(|env| env.get("ANTHROPIC_AUTH_TOKEN"))
                .and_then(|v| v.as_str())
                .map(|s| mask_api_key(s))
                .unwrap_or_else(|| "未配置".to_string());

            let base_url = config.get("env")
                .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
                .and_then(|v| v.as_str())
                .unwrap_or("未配置");

            Ok(ActiveConfig {
                api_key,
                base_url: base_url.to_string(),
            })
        },
        "codex" => {
            let auth_path = home_dir.join(".codex").join("auth.json");
            let config_path = home_dir.join(".codex").join("config.toml");

            let mut api_key = "未配置".to_string();
            let mut base_url = "未配置".to_string();

            // 读取 auth.json
            if auth_path.exists() {
                let content = fs::read_to_string(&auth_path)
                    .map_err(|e| format!("Failed to read auth: {}", e))?;
                let auth: Value = serde_json::from_str(&content)
                    .map_err(|e| format!("Failed to parse auth: {}", e))?;

                if let Some(key) = auth.get("OPENAI_API_KEY").and_then(|v| v.as_str()) {
                    api_key = mask_api_key(key);
                }
            }

            // 读取 config.toml
            if config_path.exists() {
                let content = fs::read_to_string(&config_path)
                    .map_err(|e| format!("Failed to read config: {}", e))?;
                let config: toml::Value = toml::from_str(&content)
                    .map_err(|e| format!("Failed to parse TOML: {}", e))?;

                if let toml::Value::Table(table) = config {
                    if let Some(toml::Value::Table(providers)) = table.get("model_providers") {
                        // 尝试获取 duckcoding 或 custom provider 的 base_url
                        for (_, provider) in providers {
                            if let toml::Value::Table(p) = provider {
                                if let Some(toml::Value::String(url)) = p.get("base_url") {
                                    base_url = url.clone();
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            Ok(ActiveConfig { api_key, base_url })
        },
        "gemini-cli" => {
            let env_path = home_dir.join(".gemini").join(".env");
            if !env_path.exists() {
                return Ok(ActiveConfig {
                    api_key: "未配置".to_string(),
                    base_url: "未配置".to_string(),
                });
            }

            let content = fs::read_to_string(&env_path)
                .map_err(|e| format!("Failed to read .env: {}", e))?;

            let mut api_key = "未配置".to_string();
            let mut base_url = "未配置".to_string();

            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                if let Some((key, value)) = line.split_once('=') {
                    match key.trim() {
                        "GEMINI_API_KEY" => api_key = mask_api_key(value.trim()),
                        "GOOGLE_GEMINI_BASE_URL" => base_url = value.trim().to_string(),
                        _ => {}
                    }
                }
            }

            Ok(ActiveConfig { api_key, base_url })
        },
        _ => Err(format!("Unknown tool: {}", tool))
    }
}

fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        return "****".to_string();
    }
    let prefix = &key[..4];
    let suffix = &key[key.len() - 4..];
    format!("{}...{}", prefix, suffix)
}

fn create_tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &show_item,
            &PredefinedMenuItem::separator(app)?,
            &quit_item,
        ],
    )?;

    Ok(menu)
}


fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // 设置工作目录到项目根目录（跨平台支持）
            if let Ok(resource_dir) = app.path().resource_dir() {
                println!("Resource dir: {:?}", resource_dir);

                if cfg!(debug_assertions) {
                    // 开发模式：resource_dir 是 src-tauri/target/debug
                    // 需要回到项目根目录（上三级）
                    let project_root = resource_dir
                        .parent()  // target
                        .and_then(|p| p.parent())  // src-tauri
                        .and_then(|p| p.parent())  // 项目根目录
                        .unwrap_or(&resource_dir);

                    println!("Development mode, setting dir to: {:?}", project_root);
                    let _ = env::set_current_dir(project_root);
                } else {
                    // 生产模式：跨平台支持
                    let parent_dir = if cfg!(target_os = "macos") {
                        // macOS: .app/Contents/Resources/
                        resource_dir.parent().and_then(|p| p.parent()).unwrap_or(&resource_dir)
                    } else if cfg!(target_os = "windows") {
                        // Windows: 通常在应用程序目录
                        resource_dir.parent().unwrap_or(&resource_dir)
                    } else {
                        // Linux: 通常在 /usr/share/appname 或类似位置
                        resource_dir.parent().unwrap_or(&resource_dir)
                    };
                    println!("Production mode, setting dir to: {:?}", parent_dir);
                    let _ = env::set_current_dir(parent_dir);
                }
            }

            println!("Working directory: {:?}", env::current_dir());

            // 初始化CLI目录state
            let cli_path = get_cli_dir(app.handle())?;
            app.manage(CliDir(Mutex::new(cli_path)));

            // 创建系统托盘菜单
            let tray_menu = create_tray_menu(app.handle())?;
            let app_handle = app.handle().clone();

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(move |_tray, event| {
                    match event {
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } => {
                            // 单击左键显示主窗口
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            check_installations,
            install_tool,
            check_update,
            update_tool,
            configure_api,
            list_profiles,
            switch_profile,
            get_active_config,
            save_global_config,
            get_global_config,
            generate_api_key_for_tool,
            get_usage_stats,
            get_user_quota
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
