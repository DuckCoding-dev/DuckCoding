// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};
use toml_edit::{table, value, DocumentMut};

// Windows特定：隐藏命令行窗口
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// 辅助函数：获取扩展的PATH环境变量
fn get_extended_path() -> String {
    #[cfg(target_os = "windows")]
    {
        let user_profile =
            env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".to_string());

        let system_paths = vec![
            // Claude Code 可能的安装路径
            format!("{}\\AppData\\Local\\Programs\\claude-code", user_profile),
            format!("{}\\AppData\\Roaming\\npm", user_profile),
            format!(
                "{}\\AppData\\Local\\Programs\\Python\\Python312",
                user_profile
            ),
            format!(
                "{}\\AppData\\Local\\Programs\\Python\\Python312\\Scripts",
                user_profile
            ),
            // 常见安装路径
            "C:\\Program Files\\nodejs".to_string(),
            "C:\\Program Files\\Git\\cmd".to_string(),
            // 系统路径
            "C:\\Windows\\System32".to_string(),
            "C:\\Windows".to_string(),
        ];

        let current_path = env::var("PATH").unwrap_or_default();
        format!("{};{}", system_paths.join(";"), current_path)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home_dir = env::var("HOME").unwrap_or_else(|_| "/Users/default".to_string());

        let mut system_paths = vec![
            // Claude Code 可能的安装路径
            format!("{}/.local/bin", home_dir),
            format!("{}/.claude/bin", home_dir),
            format!("{}/.claude/local", home_dir), // Claude Code local安装
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

        format!(
            "{}:{}",
            system_paths.join(":"),
            env::var("PATH").unwrap_or_default()
        )
    }
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
                .creation_flags(0x08000000) // CREATE_NO_WINDOW - 隐藏终端窗口
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
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let stderr_str = String::from_utf8_lossy(&output.stderr);

        #[cfg(debug_assertions)]
        println!(
            "Claude Code detection - status: {}, stdout: {}, stderr: {}",
            output.status.success(),
            stdout_str.trim(),
            stderr_str.trim()
        );

        // 只有命令成功执行才认为已安装
        if output.status.success() {
            if let Some(tool) = tools.iter_mut().find(|t| t.id == "claude-code") {
                tool.installed = true;
                // 尝试从stdout或stderr获取版本
                let version_output = if !stdout_str.trim().is_empty() {
                    stdout_str.trim().to_string()
                } else {
                    stderr_str.trim().to_string()
                };
                tool.version = Some(version_output);
            }
        }
    }

    // 检测 CodeX
    if let Ok(output) = run_command("codex --version 2>&1") {
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let stderr_str = String::from_utf8_lossy(&output.stderr);

        #[cfg(debug_assertions)]
        println!(
            "CodeX detection - status: {}, stdout: {}, stderr: {}",
            output.status.success(),
            stdout_str.trim(),
            stderr_str.trim()
        );

        if output.status.success() {
            if let Some(tool) = tools.iter_mut().find(|t| t.id == "codex") {
                tool.installed = true;
                let version_output = if !stdout_str.trim().is_empty() {
                    stdout_str.trim().to_string()
                } else {
                    stderr_str.trim().to_string()
                };
                tool.version = Some(version_output);
            }
        }
    }

    // 检测 Gemini CLI
    if let Ok(output) = run_command("gemini --version 2>&1") {
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let stderr_str = String::from_utf8_lossy(&output.stderr);

        #[cfg(debug_assertions)]
        println!(
            "Gemini CLI detection - status: {}, stdout: {}, stderr: {}",
            output.status.success(),
            stdout_str.trim(),
            stderr_str.trim()
        );

        if output.status.success() {
            if let Some(tool) = tools.iter_mut().find(|t| t.id == "gemini-cli") {
                tool.installed = true;
                let version_output = if !stdout_str.trim().is_empty() {
                    stdout_str.trim().to_string()
                } else {
                    stderr_str.trim().to_string()
                };
                tool.version = Some(version_output);
            }
        }
    }

    Ok(tools)
}

// 检测node环境
#[tauri::command]
async fn check_node_environment() -> Result<NodeEnvironment, String> {
    let run_command = |cmd: &str| -> Result<std::process::Output, std::io::Error> {
        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .env("PATH", get_extended_path())
                .arg("/C")
                .arg(cmd)
                .creation_flags(0x08000000) // CREATE_NO_WINDOW - 隐藏终端窗口
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

    // 检测node
    let (node_available, node_version) = if let Ok(output) = run_command("node --version 2>&1") {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            (true, Some(version))
        } else {
            (false, None)
        }
    } else {
        (false, None)
    };

    // 检测npm
    let (npm_available, npm_version) = if let Ok(output) = run_command("npm --version 2>&1") {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            (true, Some(version))
        } else {
            (false, None)
        }
    } else {
        (false, None)
    };

    Ok(NodeEnvironment {
        node_available,
        node_version,
        npm_available,
        npm_version,
    })
}

#[tauri::command]
async fn install_tool(tool: String, method: String) -> Result<InstallResult, String> {
    #[cfg(debug_assertions)]
    println!(
        "Installing {} via {} (pure Rust implementation)",
        tool, method
    );

    match tool.as_str() {
        "claude-code" => {
            if method == "npm" {
                // npm 安装
                #[cfg(target_os = "windows")]
                let output = Command::new("npm")
                    .env("PATH", get_extended_path())
                    .args(&["install", "-g", "@anthropic-ai/claude-code"])
                    .creation_flags(0x08000000)  // CREATE_NO_WINDOW - 隐藏终端窗口
                    .output()
                    .map_err(|e| {
                        if e.kind() == std::io::ErrorKind::NotFound {
                            "npm 未安装或未找到\n\n请先安装 Node.js (包含 npm):\n1. 访问 https://nodejs.org 下载安装\n2. 或使用官方安装方式（无需 npm）".to_string()
                        } else {
                            format!("执行 npm 失败: {}", e)
                        }
                    })?;

                #[cfg(not(target_os = "windows"))]
                let output = Command::new("npm")
                    .env("PATH", get_extended_path())
                    .args(&["install", "-g", "@anthropic-ai/claude-code"])
                    .output()
                    .map_err(|e| {
                        if e.kind() == std::io::ErrorKind::NotFound {
                            "npm 未安装或未找到\n\n请先安装 Node.js (包含 npm):\n1. 访问 https://nodejs.org 下载安装\n2. 或使用官方安装方式（无需 npm）".to_string()
                        } else {
                            format!("执行 npm 失败: {}", e)
                        }
                    })?;

                if output.status.success() {
                    Ok(InstallResult {
                        success: true,
                        message: "Claude Code installed successfully via npm".to_string(),
                        output: String::from_utf8_lossy(&output.stdout).to_string(),
                    })
                } else {
                    Err(format!(
                        "npm installation failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ))
                }
            } else {
                // official: 使用DuckCoding镜像安装脚本
                #[cfg(target_os = "windows")]
                {
                    // Windows: irm https://mirror.duckcoding.com/claude-code/install.ps1 | iex
                    let output = Command::new("powershell")
                        .env("PATH", get_extended_path())
                        .args(&[
                            "-Command",
                            "irm https://mirror.duckcoding.com/claude-code/install.ps1 | iex",
                        ])
                        .creation_flags(0x08000000) // CREATE_NO_WINDOW - 隐藏终端窗口
                        .output()
                        .map_err(|e| format!("Failed to execute installation: {}", e))?;

                    if output.status.success() {
                        Ok(InstallResult {
                            success: true,
                            message: "Claude Code installed successfully".to_string(),
                            output: String::from_utf8_lossy(&output.stdout).to_string(),
                        })
                    } else {
                        Err(format!(
                            "Installation failed: {}",
                            String::from_utf8_lossy(&output.stderr)
                        ))
                    }
                }

                #[cfg(not(target_os = "windows"))]
                {
                    // macOS/Linux: curl -fsSL https://mirror.duckcoding.com/claude-code/install.sh | bash
                    let output = Command::new("sh")
                        .env("PATH", get_extended_path())
                        .args(&[
                            "-c",
                            "curl -fsSL https://mirror.duckcoding.com/claude-code/install.sh | bash"
                        ])
                        .output()
                        .map_err(|e| format!("Failed to execute installation: {}", e))?;

                    if output.status.success() {
                        Ok(InstallResult {
                            success: true,
                            message: "Claude Code installed successfully".to_string(),
                            output: String::from_utf8_lossy(&output.stdout).to_string(),
                        })
                    } else {
                        Err(format!(
                            "Installation failed: {}",
                            String::from_utf8_lossy(&output.stderr)
                        ))
                    }
                }
            }
        }
        "codex" => {
            // CodeX 安装
            if method == "brew" {
                #[cfg(target_os = "macos")]
                {
                    let output = Command::new("brew")
                        .env("PATH", get_extended_path())
                        .args(&["install", "--cask", "codex"])
                        .output()
                        .map_err(|e| format!("Failed to execute brew: {}", e))?;

                    if output.status.success() {
                        Ok(InstallResult {
                            success: true,
                            message: "CodeX installed successfully via Homebrew".to_string(),
                            output: String::from_utf8_lossy(&output.stdout).to_string(),
                        })
                    } else {
                        Err(format!(
                            "Homebrew installation failed: {}",
                            String::from_utf8_lossy(&output.stderr)
                        ))
                    }
                }
                #[cfg(not(target_os = "macos"))]
                {
                    Err("Homebrew is only available on macOS".to_string())
                }
            } else {
                // npm 安装（跨平台）
                #[cfg(target_os = "windows")]
                let output = Command::new("npm")
                    .env("PATH", get_extended_path())
                    .args(&["install", "-g", "@openai/codex"])
                    .creation_flags(0x08000000)  // CREATE_NO_WINDOW - 隐藏终端窗口
                    .output()
                    .map_err(|e| {
                        if e.kind() == std::io::ErrorKind::NotFound {
                            "npm 未安装或未找到\n\n请先安装 Node.js (包含 npm):\n访问 https://nodejs.org 下载安装".to_string()
                        } else {
                            format!("执行 npm 失败: {}", e)
                        }
                    })?;

                #[cfg(not(target_os = "windows"))]
                let output = Command::new("npm")
                    .env("PATH", get_extended_path())
                    .args(&["install", "-g", "@openai/codex"])
                    .output()
                    .map_err(|e| {
                        if e.kind() == std::io::ErrorKind::NotFound {
                            "npm 未安装或未找到\n\n请先安装 Node.js (包含 npm):\n访问 https://nodejs.org 下载安装".to_string()
                        } else {
                            format!("执行 npm 失败: {}", e)
                        }
                    })?;

                if output.status.success() {
                    Ok(InstallResult {
                        success: true,
                        message: "CodeX installed successfully via npm".to_string(),
                        output: String::from_utf8_lossy(&output.stdout).to_string(),
                    })
                } else {
                    Err(format!(
                        "npm installation failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ))
                }
            }
        }
        "gemini-cli" => {
            // Gemini CLI 使用 npm 安装
            #[cfg(target_os = "windows")]
            let output = Command::new("npm")
                .env("PATH", get_extended_path())
                .args(&["install", "-g", "@google/gemini-cli"])
                .creation_flags(0x08000000)  // CREATE_NO_WINDOW - 隐藏终端窗口
                .output()
                .map_err(|e| {
                    if e.kind() == std::io::ErrorKind::NotFound {
                        "npm 未安装或未找到\n\n请先安装 Node.js (包含 npm):\n访问 https://nodejs.org 下载安装".to_string()
                    } else {
                        format!("执行 npm 失败: {}", e)
                    }
                })?;

            #[cfg(not(target_os = "windows"))]
            let output = Command::new("npm")
                .env("PATH", get_extended_path())
                .args(&["install", "-g", "@google/gemini-cli"])
                .output()
                .map_err(|e| {
                    if e.kind() == std::io::ErrorKind::NotFound {
                        "npm 未安装或未找到\n\n请先安装 Node.js (包含 npm):\n访问 https://nodejs.org 下载安装".to_string()
                    } else {
                        format!("执行 npm 失败: {}", e)
                    }
                })?;

            if output.status.success() {
                Ok(InstallResult {
                    success: true,
                    message: "Gemini CLI installed successfully via npm".to_string(),
                    output: String::from_utf8_lossy(&output.stdout).to_string(),
                })
            } else {
                Err(format!(
                    "npm installation failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ))
            }
        }
        _ => Err(format!("Unknown tool: {}", tool)),
    }
}

// npm Registry API 响应结构
#[derive(Deserialize, Debug)]
struct NpmPackageInfo {
    #[serde(rename = "dist-tags")]
    dist_tags: NpmDistTags,
}

#[derive(Deserialize, Debug)]
struct NpmDistTags {
    latest: String,
}

// 从npm镜像源获取最新版本
async fn fetch_latest_version_from_npm(package_name: &str) -> Result<String, String> {
    let client = reqwest::Client::new();

    // 优先使用国内镜像（淘宝npm镜像）
    let mirrors = vec![
        format!("https://registry.npmmirror.com/{}", package_name),
        format!("https://registry.npmjs.org/{}", package_name),
    ];

    for mirror_url in mirrors {
        println!("Trying to fetch version from: {}", mirror_url);

        match client
            .get(&mirror_url)
            .header("User-Agent", "DuckCoding-Desktop-App")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<NpmPackageInfo>().await {
                        Ok(package_info) => {
                            println!(
                                "Successfully fetched version: {}",
                                package_info.dist_tags.latest
                            );
                            return Ok(package_info.dist_tags.latest);
                        }
                        Err(e) => {
                            println!("Failed to parse response from {}: {}", mirror_url, e);
                            continue;
                        }
                    }
                } else {
                    println!(
                        "Failed to fetch from {}: status {}",
                        mirror_url,
                        response.status()
                    );
                    continue;
                }
            }
            Err(e) => {
                println!("Request to {} failed: {}", mirror_url, e);
                continue;
            }
        }
    }

    Err("所有npm镜像源均无法访问".to_string())
}

// 只检查更新，不执行
#[tauri::command]
async fn check_update(tool: String) -> Result<UpdateResult, String> {
    #[cfg(debug_assertions)]
    println!("Checking updates for {} (pure Rust + npm mirror)", tool);

    // 跨平台命令执行辅助函数
    let run_command = |cmd: &str| -> Result<std::process::Output, std::io::Error> {
        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .env("PATH", get_extended_path())
                .arg("/C")
                .arg(cmd)
                .creation_flags(0x08000000) // CREATE_NO_WINDOW - 隐藏终端窗口
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

    // 获取当前安装的版本
    let current_version = match tool.as_str() {
        "claude-code" => {
            if let Ok(output) = run_command("claude --version 2>&1") {
                if output.status.success() {
                    let stdout_str = String::from_utf8_lossy(&output.stdout);
                    let stderr_str = String::from_utf8_lossy(&output.stderr);
                    let version_output = if !stdout_str.trim().is_empty() {
                        stdout_str.trim().to_string()
                    } else {
                        stderr_str.trim().to_string()
                    };
                    extract_version(&version_output)
                } else {
                    None
                }
            } else {
                None
            }
        }
        "codex" => {
            if let Ok(output) = run_command("codex --version 2>&1") {
                if output.status.success() {
                    let stdout_str = String::from_utf8_lossy(&output.stdout);
                    let stderr_str = String::from_utf8_lossy(&output.stderr);
                    let version_output = if !stdout_str.trim().is_empty() {
                        stdout_str.trim().to_string()
                    } else {
                        stderr_str.trim().to_string()
                    };
                    extract_version(&version_output)
                } else {
                    None
                }
            } else {
                None
            }
        }
        "gemini-cli" => {
            if let Ok(output) = run_command("gemini --version 2>&1") {
                if output.status.success() {
                    let stdout_str = String::from_utf8_lossy(&output.stdout);
                    let stderr_str = String::from_utf8_lossy(&output.stderr);
                    let version_output = if !stdout_str.trim().is_empty() {
                        stdout_str.trim().to_string()
                    } else {
                        stderr_str.trim().to_string()
                    };
                    extract_version(&version_output)
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    };

    #[cfg(debug_assertions)]
    println!("Current version: {:?}", current_version);

    // 根据工具类型获取npm包名
    let package_name = match tool.as_str() {
        "claude-code" => "@anthropic-ai/claude-code",
        "codex" => "@openai/codex",
        "gemini-cli" => "@google/gemini-cli",
        _ => {
            return Err(format!("Unknown tool: {}", tool));
        }
    };

    // 从npm镜像源获取最新版本
    let latest_version_result = fetch_latest_version_from_npm(package_name).await;

    match latest_version_result {
        Ok(latest_version_str) => {
            #[cfg(debug_assertions)]
            println!("Latest version from npm: {}", latest_version_str);

            // 比较版本
            let has_update = if let Some(ref current) = current_version {
                compare_versions(current, &latest_version_str)
            } else {
                false
            };

            Ok(UpdateResult {
                success: true,
                message: "检查完成".to_string(),
                has_update,
                current_version,
                latest_version: Some(latest_version_str),
            })
        }
        Err(e) => {
            println!("Failed to fetch latest version: {}", e);
            // 降级：如果npm镜像源失败，返回无法检查但不报错
            Ok(UpdateResult {
                success: true,
                message: format!("无法检查更新: {}", e),
                has_update: false,
                current_version,
                latest_version: None,
            })
        }
    }
}

#[tauri::command]
async fn update_tool(tool: String) -> Result<UpdateResult, String> {
    #[cfg(debug_assertions)]
    println!("Updating {} (pure Rust implementation)", tool);

    // 根据工具类型获取更新命令
    let (update_command, update_args, description) = match tool.as_str() {
        "claude-code" => {
            // Claude Code 检测安装方式
            // 首先检查是否通过 npm 安装
            #[cfg(target_os = "windows")]
            let check_npm = Command::new("npm")
                .env("PATH", get_extended_path())
                .args(&["list", "-g", "@anthropic-ai/claude-code", "--depth=0"])
                .creation_flags(0x08000000) // CREATE_NO_WINDOW - 隐藏终端窗口
                .output();

            #[cfg(not(target_os = "windows"))]
            let check_npm = Command::new("npm")
                .env("PATH", get_extended_path())
                .args(&["list", "-g", "@anthropic-ai/claude-code", "--depth=0"])
                .output();

            if let Ok(output) = check_npm {
                let stdout_str = String::from_utf8_lossy(&output.stdout);
                // 如果 npm list 输出包含包名，说明是 npm 安装的
                if output.status.success() && stdout_str.contains("@anthropic-ai/claude-code") {
                    #[cfg(debug_assertions)]
                    println!("Claude Code detected as npm installation, using npm update");
                    (
                        "npm",
                        vec!["update", "-g", "@anthropic-ai/claude-code"],
                        "npm更新",
                    )
                } else {
                    // 使用官方安装脚本更新（更稳定）
                    #[cfg(debug_assertions)]
                    println!(
                        "Claude Code detected as native installation, using official installer"
                    );
                    #[cfg(target_os = "windows")]
                    {
                        (
                            "powershell",
                            vec!["-Command", "irm https://claude.ai/install.ps1 | iex"],
                            "官方安装脚本更新",
                        )
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        (
                            "sh",
                            vec!["-c", "curl -fsSL https://claude.ai/install.sh | bash"],
                            "官方安装脚本更新",
                        )
                    }
                }
            } else {
                // npm 命令失败，默认使用官方安装脚本
                #[cfg(debug_assertions)]
                println!("npm check failed, defaulting to official installer");
                #[cfg(target_os = "windows")]
                {
                    (
                        "powershell",
                        vec!["-Command", "irm https://claude.ai/install.ps1 | iex"],
                        "官方安装脚本更新",
                    )
                }
                #[cfg(not(target_os = "windows"))]
                {
                    (
                        "sh",
                        vec!["-c", "curl -fsSL https://claude.ai/install.sh | bash"],
                        "官方安装脚本更新",
                    )
                }
            }
        }
        "codex" => {
            // CodeX 更新策略：根据平台选择最佳方式
            #[cfg(target_os = "macos")]
            {
                // macOS: 优先使用 Homebrew Cask（无需 sudo，版本 0.53.0）
                // 如果用户想要最新的 npm 版本（0.54.0），需要手动切换

                // 检查是否已通过 Homebrew 安装
                let check_brew_cask = Command::new("brew")
                    .env("PATH", get_extended_path())
                    .args(&["list", "--cask", "codex"])
                    .output();

                let is_brew_installed = if let Ok(output) = check_brew_cask {
                    output.status.success()
                } else {
                    false
                };

                if is_brew_installed {
                    // 已经是 Homebrew 安装，直接升级
                    (
                        "brew",
                        vec!["upgrade", "--cask", "codex"],
                        "Homebrew Cask更新",
                    )
                } else {
                    // 不是 Homebrew，切换到 Homebrew（推荐）
                    ("sh", vec!["-c", "rm -f /opt/homebrew/bin/codex /usr/local/bin/codex ~/.local/bin/codex ~/.codex/bin/codex && brew install --cask codex"], "切换到 Homebrew Cask")
                }
            }
            #[cfg(target_os = "windows")]
            {
                // Windows: 使用 npm（主要方式）
                ("npm", vec!["update", "-g", "@openai/codex"], "npm更新")
            }
            #[cfg(target_os = "linux")]
            {
                // Linux: 使用 npm（主要方式）
                ("npm", vec!["update", "-g", "@openai/codex"], "npm更新")
            }
        }
        "gemini-cli" => {
            // Gemini CLI 使用 npm 更新（跨平台）
            ("npm", vec!["update", "-g", "@google/gemini-cli"], "npm更新")
        }
        _ => {
            return Err(format!("Unknown tool: {}", tool));
        }
    };

    println!(
        "使用{}方式更新: {} {:?}",
        description, update_command, update_args
    );

    // 执行更新，使用tokio::time::timeout添加超时（120秒）
    use tokio::time::{timeout, Duration};

    let update_task = tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            Command::new(update_command)
                .env("PATH", get_extended_path())
                .args(&update_args)
                .creation_flags(0x08000000) // CREATE_NO_WINDOW - 隐藏终端窗口
                .output()
        }
        #[cfg(not(target_os = "windows"))]
        {
            Command::new(update_command)
                .env("PATH", get_extended_path())
                .args(&update_args)
                .output()
        }
    });

    let output = match timeout(Duration::from_secs(120), update_task).await {
        Ok(Ok(Ok(output))) => output,
        Ok(Ok(Err(e))) => {
            return Err(format!("更新失败: {}", e));
        }
        Ok(Err(e)) => {
            return Err(format!("更新任务错误: {}", e));
        }
        Err(_) => {
            let timeout_msg = if description.contains("DuckCoding镜像") {
                "更新超时（120秒）。\n\n可能的原因：\n1. 镜像服务器响应慢\n2. 网络连接不稳定\n\n建议：\n1. 检查网络连接\n2. 重试更新\n3. 或使用 npm 方式：先卸载后重装\n   npm uninstall -g @anthropic-ai/claude-code\n   npm install -g @anthropic-ai/claude-code"
            } else {
                "更新超时（120秒）。\n\n请检查网络连接或稍后重试。"
            };
            return Err(timeout_msg.to_string());
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    #[cfg(debug_assertions)]
    {
        println!("Update stdout: {}", stdout);
        println!("Update stderr: {}", stderr);
    }

    // 检查是否是 npm 缓存权限错误
    if !output.status.success() && stderr.contains("EACCES") && stderr.contains(".npm") {
        return Err(format!(
            "npm 权限问题\n\n这是因为之前使用 sudo npm 安装导致的。\n\n解决方案（任选其一）：\n\n方案1 - 修复 npm 权限（推荐）：\n在终端运行：\nsudo chown -R $(id -u):$(id -g) \"$HOME/.npm\"\n\n方案2 - 配置 npm 使用用户目录：\nnpm config set prefix ~/.npm-global\nexport PATH=~/.npm-global/bin:$PATH\n\n方案3 - macOS 用户切换到 Homebrew（无需 sudo）：\nbrew uninstall --cask codex\nbrew install --cask codex\n\n然后重试更新。"
        ));
    }

    if output.status.success() {
        // 获取更新后的版本
        let run_command = |cmd: &str| -> Result<std::process::Output, std::io::Error> {
            #[cfg(target_os = "windows")]
            {
                Command::new("cmd")
                    .env("PATH", get_extended_path())
                    .arg("/C")
                    .arg(cmd)
                    .creation_flags(0x08000000) // CREATE_NO_WINDOW - 隐藏终端窗口
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

        let new_version = match tool.as_str() {
            "claude-code" => {
                if let Ok(output) = run_command("claude --version 2>&1") {
                    let stdout_str = String::from_utf8_lossy(&output.stdout);
                    let stderr_str = String::from_utf8_lossy(&output.stderr);
                    let version_output = if !stdout_str.trim().is_empty() {
                        stdout_str.trim().to_string()
                    } else {
                        stderr_str.trim().to_string()
                    };
                    extract_version(&version_output)
                } else {
                    None
                }
            }
            "codex" => {
                if let Ok(output) = run_command("codex --version 2>&1") {
                    let stdout_str = String::from_utf8_lossy(&output.stdout);
                    let stderr_str = String::from_utf8_lossy(&output.stderr);
                    let version_output = if !stdout_str.trim().is_empty() {
                        stdout_str.trim().to_string()
                    } else {
                        stderr_str.trim().to_string()
                    };
                    extract_version(&version_output)
                } else {
                    None
                }
            }
            "gemini-cli" => {
                if let Ok(output) = run_command("gemini --version 2>&1") {
                    let stdout_str = String::from_utf8_lossy(&output.stdout);
                    let stderr_str = String::from_utf8_lossy(&output.stderr);
                    let version_output = if !stdout_str.trim().is_empty() {
                        stdout_str.trim().to_string()
                    } else {
                        stderr_str.trim().to_string()
                    };
                    extract_version(&version_output)
                } else {
                    None
                }
            }
            _ => None,
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
async fn configure_api(
    tool: String,
    _provider: String,
    api_key: String,
    base_url: Option<String>,
    profile_name: Option<String>,
) -> Result<(), String> {
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;
    let base_url_str = base_url.unwrap_or_else(|| "https://jp.duckcoding.com".to_string());

    match tool.as_str() {
        "claude-code" => {
            let config_dir = home_dir.join(".claude");
            let config_path = config_dir.join("settings.json");

            // 确保目录存在
            fs::create_dir_all(&config_dir)
                .map_err(|e| format!("Failed to create directory: {}", e))?;

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
            env_obj.insert(
                "ANTHROPIC_AUTH_TOKEN".to_string(),
                Value::String(api_key.clone()),
            );
            env_obj.insert(
                "ANTHROPIC_BASE_URL".to_string(),
                Value::String(base_url_str.clone()),
            );

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
                    fs::write(
                        &backup_path,
                        serde_json::to_string_pretty(&backup_config).unwrap(),
                    )
                    .map_err(|e| format!("Failed to write backup: {}", e))?;
                }
            }
        }
        "codex" => {
            println!("Configuring CodeX directly in Rust (no cli.js)...");
            let config_dir = home_dir.join(".codex");
            let config_path = config_dir.join("config.toml");
            let auth_path = config_dir.join("auth.json");

            fs::create_dir_all(&config_dir)
                .map_err(|e| format!("Failed to create directory: {}", e))?;

            let mut config_doc = if config_path.exists() {
                let content = fs::read_to_string(&config_path)
                    .map_err(|e| format!("Failed to read config.toml: {}", e))?;
                content
                    .parse::<DocumentMut>()
                    .unwrap_or_else(|_| DocumentMut::new())
            } else {
                DocumentMut::new()
            };

            let config_table = config_doc.as_table_mut();
            config_table["model_provider"] = value("duckcoding");
            config_table["model"] = value("gpt-5-codex");
            config_table["model_reasoning_effort"] = value("high");
            config_table["network_access"] = value("enabled");
            config_table["disable_response_storage"] = value(true);

            let provider_key = if base_url_str.contains("duckcoding") {
                "duckcoding"
            } else {
                "custom"
            };
            let provider_base_url = if base_url_str.ends_with("/v1") {
                base_url_str.clone()
            } else {
                format!("{}/v1", base_url_str)
            };

            let providers_table = config_table
                .entry("model_providers")
                .or_insert(table())
                .as_table_mut()
                .expect("model_providers should be a table");

            let provider_entry = providers_table.entry(provider_key).or_insert(table());
            let provider_table = provider_entry
                .as_table_mut()
                .expect("provider entry should be a table");

            provider_table["name"] = value(provider_key);
            provider_table["base_url"] = value(provider_base_url);
            provider_table["wire_api"] = value("responses");
            provider_table["requires_openai_auth"] = value(true);

            let serialized_config = config_doc.to_string();
            fs::write(&config_path, &serialized_config)
                .map_err(|e| format!("Failed to write config.toml: {}", e))?;
            println!("CodeX config.toml written successfully");

            let auth_data = serde_json::json!({
                "OPENAI_API_KEY": api_key
            });
            fs::write(
                &auth_path,
                serde_json::to_string_pretty(&auth_data).unwrap(),
            )
            .map_err(|e| format!("Failed to write auth.json: {}", e))?;
            println!("CodeX auth.json written successfully");

            if let Some(profile) = &profile_name {
                if !profile.is_empty() {
                    println!("Saving CodeX backup for profile: {}", profile);
                    let backup_config_path = config_dir.join(format!("config.{}.toml", profile));
                    fs::write(&backup_config_path, &serialized_config)
                        .map_err(|e| format!("Failed to write backup config: {}", e))?;

                    let backup_auth_path = config_dir.join(format!("auth.{}.json", profile));
                    fs::write(
                        &backup_auth_path,
                        serde_json::to_string_pretty(&auth_data).unwrap(),
                    )
                    .map_err(|e| format!("Failed to write backup auth: {}", e))?;

                    println!(
                        "CodeX backup saved: config.{}.toml, auth.{}.json",
                        profile, profile
                    );
                }
            }
        }
        "gemini-cli" => {
            let config_dir = home_dir.join(".gemini");
            let env_path = config_dir.join(".env");

            // 确保目录存在
            fs::create_dir_all(&config_dir)
                .map_err(|e| format!("Failed to create directory: {}", e))?;

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
            let env_content: String = env_vars
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";

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
        }
        _ => return Err(format!("Unknown tool: {}", tool)),
    }

    Ok(())
}

#[tauri::command]
async fn list_profiles(tool: String) -> Result<Vec<String>, String> {
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
        }
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
        }
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
        }
        _ => return Err(format!("Unknown tool: {}", tool)),
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
            fs::write(
                &active_path,
                serde_json::to_string_pretty(&active_config).unwrap(),
            )
            .map_err(|e| format!("Failed to write active config: {}", e))?;
        }
        "codex" => {
            let config_dir = home_dir.join(".codex");
            let backup_config_path = config_dir.join(format!("config.{}.toml", profile));
            let active_config_path = config_dir.join("config.toml");
            let backup_auth_path = config_dir.join(format!("auth.{}.json", profile));
            let active_auth_path = config_dir.join("auth.json");

            if !backup_config_path.exists() {
                return Err(format!("Backup config not found: {:?}", backup_config_path));
            }

            let backup_config_content = fs::read_to_string(&backup_config_path)
                .map_err(|e| format!("Failed to read backup config: {}", e))?;
            let backup_doc = backup_config_content
                .parse::<DocumentMut>()
                .map_err(|e| format!("Failed to parse backup TOML: {}", e))?;

            let mut active_doc = if active_config_path.exists() {
                let content = fs::read_to_string(&active_config_path)
                    .map_err(|e| format!("Failed to read active config: {}", e))?;
                content
                    .parse::<DocumentMut>()
                    .unwrap_or_else(|_| DocumentMut::new())
            } else {
                DocumentMut::new()
            };

            {
                let backup_table = backup_doc.as_table();
                let active_table = active_doc.as_table_mut();

                for key in [
                    "model_provider",
                    "model",
                    "model_reasoning_effort",
                    "network_access",
                    "disable_response_storage",
                ] {
                    if let Some(item) = backup_table.get(key) {
                        active_table.insert(key, item.clone());
                    }
                }

                if let Some(backup_providers) = backup_table
                    .get("model_providers")
                    .and_then(|item| item.as_table())
                {
                    let providers_table = active_table
                        .entry("model_providers")
                        .or_insert(table())
                        .as_table_mut()
                        .expect("model_providers should be a table");

                    for (key, value) in backup_providers.iter() {
                        providers_table.insert(key, value.clone());
                    }
                }
            }

            let toml_string = active_doc.to_string();
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

                fs::write(
                    &active_auth_path,
                    serde_json::to_string_pretty(&active_auth).unwrap(),
                )
                .map_err(|e| format!("Failed to write active auth: {}", e))?;
            }
        }
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
            let env_content: String = active_env
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";

            fs::write(&active_env_path, env_content)
                .map_err(|e| format!("Failed to write active .env: {}", e))?;
        }
        _ => return Err(format!("Unknown tool: {}", tool)),
    }

    Ok(())
}

#[tauri::command]
async fn delete_profile(tool: String, profile: String) -> Result<(), String> {
    println!("Deleting profile: tool={}, profile={}", tool, profile);
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;

    match tool.as_str() {
        "claude-code" => {
            let config_dir = home_dir.join(".claude");
            let backup_path = config_dir.join(format!("settings.{}.json", profile));
            println!("Claude Code backup path: {:?}", backup_path);

            if !backup_path.exists() {
                let err = format!("配置文件不存在: {}", profile);
                println!("Error: {}", err);
                return Err(err);
            }

            fs::remove_file(&backup_path).map_err(|e| {
                let err = format!("删除配置失败: {}", e);
                println!("Error: {}", err);
                err
            })?;
            println!("Successfully deleted Claude Code profile: {}", profile);
        }
        "codex" => {
            let config_dir = home_dir.join(".codex");
            let backup_config_path = config_dir.join(format!("config.{}.toml", profile));
            let backup_auth_path = config_dir.join(format!("auth.{}.json", profile));
            println!("CodeX config path: {:?}", backup_config_path);
            println!("CodeX auth path: {:?}", backup_auth_path);

            if !backup_config_path.exists() {
                let err = format!("配置文件不存在: {}", profile);
                println!("Error: {}", err);
                return Err(err);
            }

            fs::remove_file(&backup_config_path).map_err(|e| {
                let err = format!("删除配置失败: {}", e);
                println!("Error: {}", err);
                err
            })?;
            println!("Deleted config.toml for profile: {}", profile);

            if backup_auth_path.exists() {
                fs::remove_file(&backup_auth_path).map_err(|e| {
                    let err = format!("删除认证文件失败: {}", e);
                    println!("Error: {}", err);
                    err
                })?;
                println!("Deleted auth.json for profile: {}", profile);
            }
            println!("Successfully deleted CodeX profile: {}", profile);
        }
        "gemini-cli" => {
            let config_dir = home_dir.join(".gemini");
            let backup_env_path = config_dir.join(format!(".env.{}", profile));
            let backup_settings_path = config_dir.join(format!("settings.{}.json", profile));

            if !backup_env_path.exists() {
                return Err(format!("配置文件不存在: {}", profile));
            }

            fs::remove_file(&backup_env_path).map_err(|e| format!("删除配置失败: {}", e))?;

            if backup_settings_path.exists() {
                fs::remove_file(&backup_settings_path)
                    .map_err(|e| format!("删除设置文件失败: {}", e))?;
            }
        }
        _ => return Err(format!("Unknown tool: {}", tool)),
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
struct NodeEnvironment {
    node_available: bool,
    node_version: Option<String>,
    npm_available: bool,
    npm_version: Option<String>,
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
    profile_name: Option<String>, // 当前配置的名称
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
    println!("save_global_config called with user_id: {}", user_id);

    let config = GlobalConfig {
        user_id,
        system_token,
    };
    let config_path = get_global_config_path()?;

    println!("Config path: {:?}", config_path);

    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&config_path, json).map_err(|e| format!("Failed to write config: {}", e))?;

    println!("Config saved successfully");

    // 设置文件权限为仅所有者可读写（Unix系统）
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(&config_path)
            .map_err(|e| format!("Failed to get file metadata: {}", e))?;
        let mut perms = metadata.permissions();
        perms.set_mode(0o600); // -rw-------
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

    let content =
        fs::read_to_string(&config_path).map_err(|e| format!("Failed to read config: {}", e))?;

    let config: GlobalConfig =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse config: {}", e))?;

    Ok(Some(config))
}

// 生成API Key的主函数
#[tauri::command]
async fn generate_api_key_for_tool(tool: String) -> Result<GenerateApiKeyResult, String> {
    // 读取全局配置
    let global_config = get_global_config()
        .await?
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
        .header(
            "Authorization",
            format!("Bearer {}", global_config.system_token),
        )
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
    let search_url = format!(
        "https://duckcoding.com/api/token/search?keyword={}",
        urlencoding::encode(name)
    );

    let search_response = client
        .get(&search_url)
        .header(
            "Authorization",
            format!("Bearer {}", global_config.system_token),
        )
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
    let global_config = get_global_config()
        .await?
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
        .header(
            "Authorization",
            format!("Bearer {}", global_config.system_token),
        )
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
    let global_config = get_global_config()
        .await?
        .ok_or("请先配置用户ID和系统访问令牌")?;

    // 调用API
    let client = reqwest::Client::new();
    let url = "https://duckcoding.com/api/user/self";

    let response = client
        .get(url)
        .header(
            "Authorization",
            format!("Bearer {}", global_config.system_token),
        )
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
        println!(
            "Raw remaining: {}, converted: {}",
            user_info.quota, remaining_quota
        );
        println!(
            "Raw used: {}, converted: {}",
            user_info.used_quota, used_quota
        );
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

// 辅助函数：检测当前配置匹配哪个profile
fn detect_profile_name(
    tool: &str,
    active_api_key: &str,
    active_base_url: &str,
    home_dir: &std::path::Path,
) -> Option<String> {
    let config_dir = match tool {
        "claude-code" => home_dir.join(".claude"),
        "codex" => home_dir.join(".codex"),
        "gemini-cli" => home_dir.join(".gemini"),
        _ => return None,
    };

    if !config_dir.exists() {
        return None;
    }

    // 遍历配置目录，查找匹配的备份文件
    if let Ok(entries) = fs::read_dir(&config_dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            // 根据工具类型匹配不同的备份文件格式
            let profile_name = match tool {
                "claude-code" => {
                    // 匹配 settings.{profile}.json
                    if file_name_str.starts_with("settings.")
                        && file_name_str.ends_with(".json")
                        && file_name_str != "settings.json"
                    {
                        file_name_str
                            .strip_prefix("settings.")
                            .and_then(|s| s.strip_suffix(".json"))
                    } else {
                        None
                    }
                }
                "codex" => {
                    // 匹配 config.{profile}.toml
                    if file_name_str.starts_with("config.")
                        && file_name_str.ends_with(".toml")
                        && file_name_str != "config.toml"
                    {
                        file_name_str
                            .strip_prefix("config.")
                            .and_then(|s| s.strip_suffix(".toml"))
                    } else {
                        None
                    }
                }
                "gemini-cli" => {
                    // 匹配 .env.{profile}
                    if file_name_str.starts_with(".env.") && file_name_str != ".env" {
                        file_name_str.strip_prefix(".env.")
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(profile) = profile_name {
                // 读取备份文件并比较内容
                let is_match = match tool {
                    "claude-code" => {
                        if let Ok(content) = fs::read_to_string(entry.path()) {
                            if let Ok(config) = serde_json::from_str::<Value>(&content) {
                                let backup_api_key = config
                                    .get("env")
                                    .and_then(|env| env.get("ANTHROPIC_AUTH_TOKEN"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                let backup_base_url = config
                                    .get("env")
                                    .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");

                                backup_api_key == active_api_key
                                    && backup_base_url == active_base_url
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    }
                    "codex" => {
                        // 需要同时检查 config.toml 和 auth.json
                        let auth_backup = config_dir.join(format!("auth.{}.json", profile));

                        let mut api_key_matches = false;
                        if let Ok(auth_content) = fs::read_to_string(&auth_backup) {
                            if let Ok(auth) = serde_json::from_str::<Value>(&auth_content) {
                                let backup_api_key = auth
                                    .get("OPENAI_API_KEY")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");

                                api_key_matches = backup_api_key == active_api_key;
                            }
                        }

                        if !api_key_matches {
                            false
                        } else {
                            // API Key 匹配，继续检查 base_url
                            if let Ok(config_content) = fs::read_to_string(entry.path()) {
                                if let Ok(doc) = config_content.parse::<DocumentMut>() {
                                    if let Some(providers) = doc
                                        .as_table()
                                        .get("model_providers")
                                        .and_then(|item| item.as_table())
                                    {
                                        let mut url_matches = false;
                                        for (_, provider) in providers.iter() {
                                            if let Some(provider_table) = provider.as_table() {
                                                if let Some(url) = provider_table
                                                    .get("base_url")
                                                    .and_then(|item| item.as_value())
                                                    .and_then(|value| value.as_str())
                                                {
                                                    if url == active_base_url {
                                                        url_matches = true;
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                        url_matches
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        }
                    }
                    "gemini-cli" => {
                        if let Ok(content) = fs::read_to_string(entry.path()) {
                            let mut backup_api_key = "";
                            let mut backup_base_url = "";

                            for line in content.lines() {
                                let line = line.trim();
                                if line.is_empty() || line.starts_with('#') {
                                    continue;
                                }

                                if let Some((key, value)) = line.split_once('=') {
                                    match key.trim() {
                                        "GEMINI_API_KEY" => backup_api_key = value.trim(),
                                        "GOOGLE_GEMINI_BASE_URL" => backup_base_url = value.trim(),
                                        _ => {}
                                    }
                                }
                            }

                            backup_api_key == active_api_key && backup_base_url == active_base_url
                        } else {
                            false
                        }
                    }
                    _ => false,
                };

                if is_match {
                    return Some(profile.to_string());
                }
            }
        }
    }

    None
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
                    profile_name: None,
                });
            }

            let content = fs::read_to_string(&config_path)
                .map_err(|e| format!("Failed to read config: {}", e))?;
            let config: Value = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse config: {}", e))?;

            let raw_api_key = config
                .get("env")
                .and_then(|env| env.get("ANTHROPIC_AUTH_TOKEN"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let api_key = if raw_api_key.is_empty() {
                "未配置".to_string()
            } else {
                mask_api_key(raw_api_key)
            };

            let base_url = config
                .get("env")
                .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
                .and_then(|v| v.as_str())
                .unwrap_or("未配置");

            // 检测配置名称
            let profile_name = if !raw_api_key.is_empty() && base_url != "未配置" {
                detect_profile_name("claude-code", raw_api_key, base_url, &home_dir)
            } else {
                None
            };

            Ok(ActiveConfig {
                api_key,
                base_url: base_url.to_string(),
                profile_name,
            })
        }
        "codex" => {
            let auth_path = home_dir.join(".codex").join("auth.json");
            let config_path = home_dir.join(".codex").join("config.toml");

            let mut raw_api_key = String::new();
            let mut api_key = "未配置".to_string();
            let mut base_url = "未配置".to_string();

            // 读取 auth.json
            if auth_path.exists() {
                let content = fs::read_to_string(&auth_path)
                    .map_err(|e| format!("Failed to read auth: {}", e))?;
                let auth: Value = serde_json::from_str(&content)
                    .map_err(|e| format!("Failed to parse auth: {}", e))?;

                if let Some(key) = auth.get("OPENAI_API_KEY").and_then(|v| v.as_str()) {
                    raw_api_key = key.to_string();
                    api_key = mask_api_key(key);
                }
            }

            // 读取 config.toml
            if config_path.exists() {
                let content = fs::read_to_string(&config_path)
                    .map_err(|e| format!("Failed to read config: {}", e))?;
                let config = content
                    .parse::<DocumentMut>()
                    .map_err(|e| format!("Failed to parse TOML: {}", e))?;

                if let Some(providers) = config
                    .as_table()
                    .get("model_providers")
                    .and_then(|item| item.as_table())
                {
                    for (_, provider) in providers.iter() {
                        if let Some(provider_table) = provider.as_table() {
                            if let Some(url) = provider_table
                                .get("base_url")
                                .and_then(|item| item.as_value())
                                .and_then(|value| value.as_str())
                            {
                                base_url = url.to_string();
                                break;
                            }
                        }
                    }
                }
            }

            // 检测配置名称
            let profile_name = if !raw_api_key.is_empty() && base_url != "未配置" {
                detect_profile_name("codex", &raw_api_key, &base_url, &home_dir)
            } else {
                None
            };

            Ok(ActiveConfig {
                api_key,
                base_url,
                profile_name,
            })
        }
        "gemini-cli" => {
            let env_path = home_dir.join(".gemini").join(".env");
            if !env_path.exists() {
                return Ok(ActiveConfig {
                    api_key: "未配置".to_string(),
                    base_url: "未配置".to_string(),
                    profile_name: None,
                });
            }

            let content =
                fs::read_to_string(&env_path).map_err(|e| format!("Failed to read .env: {}", e))?;

            let mut raw_api_key = String::new();
            let mut api_key = "未配置".to_string();
            let mut base_url = "未配置".to_string();

            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                if let Some((key, value)) = line.split_once('=') {
                    match key.trim() {
                        "GEMINI_API_KEY" => {
                            raw_api_key = value.trim().to_string();
                            api_key = mask_api_key(value.trim());
                        }
                        "GOOGLE_GEMINI_BASE_URL" => base_url = value.trim().to_string(),
                        _ => {}
                    }
                }
            }

            // 检测配置名称
            let profile_name = if !raw_api_key.is_empty() && base_url != "未配置" {
                detect_profile_name("gemini-cli", &raw_api_key, &base_url, &home_dir)
            } else {
                None
            };

            Ok(ActiveConfig {
                api_key,
                base_url,
                profile_name,
            })
        }
        _ => Err(format!("Unknown tool: {}", tool)),
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
        &[&show_item, &PredefinedMenuItem::separator(app)?, &quit_item],
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
                        .parent() // target
                        .and_then(|p| p.parent()) // src-tauri
                        .and_then(|p| p.parent()) // 项目根目录
                        .unwrap_or(&resource_dir);

                    println!("Development mode, setting dir to: {:?}", project_root);
                    let _ = env::set_current_dir(project_root);
                } else {
                    // 生产模式：跨平台支持
                    let parent_dir = if cfg!(target_os = "macos") {
                        // macOS: .app/Contents/Resources/
                        resource_dir
                            .parent()
                            .and_then(|p| p.parent())
                            .unwrap_or(&resource_dir)
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

            // 创建系统托盘菜单
            let tray_menu = create_tray_menu(app.handle())?;
            let app_handle2 = app.handle().clone();

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| {
                    println!("Tray menu event: {:?}", event.id);
                    match event.id.as_ref() {
                        "show" => {
                            println!("Show window requested from tray menu");
                            if let Some(window) = app.get_webview_window("main") {
                                println!("Window is_visible: {:?}", window.is_visible());
                                println!("Window is_minimized: {:?}", window.is_minimized());

                                // 显示并激活窗口
                                if let Err(e) = window.show() {
                                    println!("Error showing window: {:?}", e);
                                }
                                if let Err(e) = window.unminimize() {
                                    println!("Error unminimizing window: {:?}", e);
                                }
                                if let Err(e) = window.set_focus() {
                                    println!("Error setting focus: {:?}", e);
                                }

                                // macOS: 强制激活应用到前台
                                #[cfg(target_os = "macos")]
                                {
                                    use cocoa::appkit::NSApplication;
                                    use cocoa::base::nil;
                                    use objc::runtime::YES;

                                    unsafe {
                                        let ns_app = NSApplication::sharedApplication(nil);
                                        ns_app.activateIgnoringOtherApps_(YES);
                                    }
                                    println!("macOS app activated");
                                }

                                println!("After show - is_visible: {:?}", window.is_visible());
                            } else {
                                println!("Window not found!");
                            }
                        }
                        "quit" => {
                            println!("Quit requested from tray menu");
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(move |_tray, event| {
                    println!("Tray icon event received: {:?}", event);
                    match event {
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } => {
                            println!("Tray icon LEFT click detected");
                            // 单击左键显示主窗口
                            if let Some(window) = app_handle2.get_webview_window("main") {
                                println!("Window found, is_visible: {:?}", window.is_visible());
                                println!("Window is_minimized: {:?}", window.is_minimized());

                                // 显示并激活窗口
                                if let Err(e) = window.show() {
                                    println!("Error showing window: {:?}", e);
                                }
                                if let Err(e) = window.unminimize() {
                                    println!("Error unminimizing window: {:?}", e);
                                }
                                if let Err(e) = window.set_focus() {
                                    println!("Error setting focus: {:?}", e);
                                }

                                // macOS: 强制激活应用到前台
                                #[cfg(target_os = "macos")]
                                {
                                    use cocoa::appkit::NSApplication;
                                    use cocoa::base::nil;
                                    use objc::runtime::YES;

                                    unsafe {
                                        let ns_app = NSApplication::sharedApplication(nil);
                                        ns_app.activateIgnoringOtherApps_(YES);
                                    }
                                    println!("macOS app activated");
                                }

                                println!("After show - is_visible: {:?}", window.is_visible());
                            } else {
                                println!("Window not found from tray click!");
                            }
                        }
                        _ => {
                            // 不打印太多日志
                        }
                    }
                })
                .build(app)?;

            // 处理窗口关闭事件 - 最小化到托盘而不是退出
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        println!("Window close requested - hiding to tray");
                        // 阻止默认关闭行为
                        api.prevent_close();
                        // 隐藏窗口到托盘
                        let _ = window_clone.hide();
                        println!("Window hidden");
                    }
                });
            }

            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            check_installations,
            check_node_environment,
            install_tool,
            check_update,
            update_tool,
            configure_api,
            list_profiles,
            switch_profile,
            delete_profile,
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
