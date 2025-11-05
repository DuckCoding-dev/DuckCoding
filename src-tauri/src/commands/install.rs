use std::process::Command;

use crate::error::{AppError, AppResult};
use crate::models::{InstallResult, NodeEnvironment, ToolStatus, UpdateResult};
#[cfg(target_os = "windows")]
use crate::services::CREATE_NO_WINDOW;
use crate::services::{extended_path, CommandRunner};
use serde::Deserialize;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[tauri::command]
pub async fn check_installations() -> Result<Vec<ToolStatus>, String> {
    check_installations_impl().map_err(|e| e.to_string())
}

fn check_installations_impl() -> AppResult<Vec<ToolStatus>> {
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

    let runner = CommandRunner::new();

    check_tool_installed(&runner, "claude", "claude-code", &mut tools)?;
    check_tool_installed(&runner, "codex", "codex", &mut tools)?;
    check_tool_installed(&runner, "gemini", "gemini-cli", &mut tools)?;

    Ok(tools)
}

fn check_tool_installed(
    runner: &CommandRunner,
    command: &str,
    tool_id: &str,
    tools: &mut [ToolStatus],
) -> AppResult<()> {
    let cmd = format!("{} --version 2>&1", command);
    if let Ok(output) = runner.run(&cmd) {
        if output.status.success() {
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            let version_output = if !stdout_str.trim().is_empty() {
                stdout_str.trim().to_string()
            } else {
                stderr_str.trim().to_string()
            };
            let clean_version = extract_version(&version_output).unwrap_or(version_output);

            if let Some(tool) = tools.iter_mut().find(|t| t.id == tool_id) {
                tool.installed = true;
                tool.version = Some(clean_version);
            }
        }
    }
    Ok(())
}

fn permission_denied_error(action: &str) -> AppError {
    AppError::Other(format!(
        "{}权限不足。请以管理员身份重新执行，或参考 npm 官方文档修复权限问题：https://docs.npmjs.com/getting-started/fixing-npm-permissions",
        action
    ))
}

fn output_indicates_permission_denied(output: &std::process::Output) -> bool {
    let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
    stderr.contains("eacces") || stderr.contains("permission denied")
}

#[tauri::command]
pub async fn check_node_environment() -> Result<NodeEnvironment, String> {
    check_node_environment_impl().map_err(|e| e.to_string())
}

fn check_node_environment_impl() -> AppResult<NodeEnvironment> {
    let runner = CommandRunner::new();

    let (node_available, node_version) = match runner.run("node --version 2>&1") {
        Ok(output) if output.status.success() => {
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            let version_output = if !stdout_str.trim().is_empty() {
                stdout_str.trim().to_string()
            } else {
                stderr_str.trim().to_string()
            };
            (true, Some(version_output))
        }
        _ => (false, None),
    };

    let (npm_available, npm_version) = match runner.run("npm --version 2>&1") {
        Ok(output) if output.status.success() => {
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            let version_output = if !stdout_str.trim().is_empty() {
                stdout_str.trim().to_string()
            } else {
                stderr_str.trim().to_string()
            };
            (true, Some(version_output))
        }
        _ => (false, None),
    };

    Ok(NodeEnvironment {
        node_available,
        node_version,
        npm_available,
        npm_version,
    })
}

#[tauri::command]
pub async fn install_tool(tool: String, method: String) -> Result<InstallResult, String> {
    install_tool_impl(tool, method).map_err(|e| e.to_string())
}

fn install_tool_impl(tool: String, method: String) -> AppResult<InstallResult> {
    match (tool.as_str(), method.as_str()) {
        ("claude-code", "npm") => install_claude_via_npm(),
        ("claude-code", "mirror") => install_claude_via_mirror(),
        ("codex", "npm") => install_codex_via_npm(),
        ("codex", "mirror") => install_codex_via_mirror(),
        ("gemini-cli", "npm") => install_gemini_via_npm(),
        _ => Err(AppError::config("不支持的工具或安装方式")),
    }
}

fn install_claude_via_npm() -> AppResult<InstallResult> {
    execute_npm_install("@anthropic-ai/claude-code")
}

fn install_codex_via_npm() -> AppResult<InstallResult> {
    execute_npm_install("@openai/codex")
}

fn install_gemini_via_npm() -> AppResult<InstallResult> {
    execute_npm_install("@google/gemini-cli")
}

fn execute_npm_install(package: &str) -> AppResult<InstallResult> {
    #[cfg(target_os = "windows")]
    let output = Command::new("npm")
        .env("PATH", extended_path())
        .args(["install", "-g", package])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                permission_denied_error("npm 安装")
            } else {
                AppError::from(e)
            }
        })?;

    #[cfg(not(target_os = "windows"))]
    let output = Command::new("npm")
        .env("PATH", extended_path())
        .args(["install", "-g", package])
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                permission_denied_error("npm 安装")
            } else {
                AppError::from(e)
            }
        })?;

    if output.status.success() {
        Ok(InstallResult {
            success: true,
            message: format!("{} 安装成功", package),
            output: String::from_utf8_lossy(&output.stdout).to_string(),
        })
    } else {
        if output_indicates_permission_denied(&output) {
            Err(permission_denied_error("npm 安装"))
        } else {
            Err(AppError::command(format!(
                "npm 安装失败: {}",
                String::from_utf8_lossy(&output.stderr)
            )))
        }
    }
}

fn install_claude_via_mirror() -> AppResult<InstallResult> {
    #[cfg(target_os = "windows")]
    let command = (
        "powershell",
        vec![
            "-Command",
            "irm https://mirror.duckcoding.com/claude-code/install.ps1 | iex",
        ],
    );

    #[cfg(not(target_os = "windows"))]
    let command = (
        "sh",
        vec![
            "-c",
            "curl -fsSL https://mirror.duckcoding.com/claude-code/install.sh | bash",
        ],
    );

    execute_shell_command(command.0, &command.1)
}

#[derive(Deserialize)]
pub struct CheckUpdateArgs {
    tool: String,
    #[serde(rename = "currentVersion")]
    current_version: Option<String>,
}

#[tauri::command]
pub async fn check_update(args: CheckUpdateArgs) -> Result<UpdateResult, String> {
    check_update_impl(args.tool, args.current_version)
        .await
        .map_err(|e| e.to_string())
}

async fn check_update_impl(
    tool: String,
    provided_version: Option<String>,
) -> AppResult<UpdateResult> {
    let runner = CommandRunner::new();
    let detected_version = current_version(&runner, &tool)?;
    let current_version_opt = detected_version.or(provided_version);

    let package_name = match tool.as_str() {
        "claude-code" => "@anthropic-ai/claude-code",
        "codex" => "@openai/codex",
        "gemini-cli" => "@google/gemini-cli",
        _ => return Err(AppError::config(format!("Unknown tool: {}", tool))),
    };

    #[cfg(debug_assertions)]
    println!("[update] checking package {}", package_name);

    let latest_version_str = fetch_latest_version_from_npm(package_name).await?;

    #[cfg(debug_assertions)]
    println!(
        "[update] current={:?}, latest={}",
        current_version_opt, latest_version_str
    );

    let has_update = current_version_opt
        .as_ref()
        .map(|current| compare_versions(current, &latest_version_str))
        .unwrap_or(false);

    Ok(UpdateResult {
        success: true,
        message: "检查完成".to_string(),
        has_update,
        current_version: current_version_opt,
        latest_version: Some(latest_version_str),
    })
}

#[tauri::command]
pub async fn update_tool(tool: String) -> Result<UpdateResult, String> {
    update_tool_impl(tool).await.map_err(|e| e.to_string())
}

async fn update_tool_impl(tool: String) -> AppResult<UpdateResult> {
    let runner = CommandRunner::new();
    let current_version_opt = current_version(&runner, &tool)?;

    let (cmd, args, description) = match tool.as_str() {
        "claude-code" => detect_claude_update_command()?,
        "codex" => detect_codex_update_command()?,
        "gemini-cli" => (
            "npm",
            vec!["update", "-g", "@google/gemini-cli"],
            "npm更新".to_string(),
        ),
        _ => return Err(AppError::config(format!("Unknown tool: {}", tool))),
    };

    #[cfg(debug_assertions)]
    println!("[update] executing {} {:?}", cmd, args);

    let output = run_update_command(cmd, &args).await?;

    if !output.status.success() {
        #[cfg(debug_assertions)]
        println!(
            "[update] command failed stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if output_indicates_permission_denied(&output) {
            return Err(permission_denied_error("更新操作"));
        }
        return Err(AppError::command(format!(
            "{} 失败: {}",
            description,
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let new_runner = CommandRunner::new();
    let new_version = current_version(&new_runner, tool.as_str())?;

    Ok(UpdateResult {
        success: true,
        message: format!("{} 已完成", description),
        has_update: false,
        current_version: new_version.clone().or(current_version_opt),
        latest_version: new_version,
    })
}

fn detect_claude_update_command() -> AppResult<(&'static str, Vec<&'static str>, String)> {
    #[cfg(target_os = "windows")]
    let check_npm = Command::new("npm")
        .env("PATH", extended_path())
        .args(["list", "-g", "@anthropic-ai/claude-code", "--depth=0"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    #[cfg(not(target_os = "windows"))]
    let check_npm = Command::new("npm")
        .env("PATH", extended_path())
        .args(["list", "-g", "@anthropic-ai/claude-code", "--depth=0"])
        .output();

    if let Ok(output) = check_npm {
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        if output.status.success() && stdout_str.contains("@anthropic-ai/claude-code") {
            return Ok((
                "npm",
                vec!["update", "-g", "@anthropic-ai/claude-code"],
                "npm更新".to_string(),
            ));
        }
    }

    #[cfg(target_os = "windows")]
    {
        Ok((
            "powershell",
            vec!["-Command", "irm https://claude.ai/install.ps1 | iex"],
            "官方安装脚本更新".to_string(),
        ))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok((
            "sh",
            vec!["-c", "curl -fsSL https://claude.ai/install.sh | bash"],
            "官方安装脚本更新".to_string(),
        ))
    }
}

fn detect_codex_update_command() -> AppResult<(&'static str, Vec<&'static str>, String)> {
    #[cfg(target_os = "windows")]
    let check_npm = Command::new("npm")
        .env("PATH", extended_path())
        .args(["list", "-g", "@openai/codex", "--depth=0"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    #[cfg(not(target_os = "windows"))]
    let check_npm = Command::new("npm")
        .env("PATH", extended_path())
        .args(["list", "-g", "@openai/codex", "--depth=0"])
        .output();

    if let Ok(output) = check_npm {
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        if output.status.success() && stdout_str.contains("@openai/codex") {
            return Ok((
                "npm",
                vec!["update", "-g", "@openai/codex"],
                "npm更新".to_string(),
            ));
        }
    }

    #[cfg(target_os = "windows")]
    {
        Ok((
            "powershell",
            vec!["-Command", "irm https://codex.openai.com/install.ps1 | iex"],
            "官方安装脚本更新".to_string(),
        ))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok((
            "sh",
            vec![
                "-c",
                "curl -fsSL https://codex.openai.com/install.sh | bash",
            ],
            "官方安装脚本更新".to_string(),
        ))
    }
}

fn current_version(runner: &CommandRunner, tool: &str) -> AppResult<Option<String>> {
    let cmd = match tool {
        "claude-code" => "claude --version 2>&1",
        "codex" => "codex --version 2>&1",
        "gemini-cli" => "gemini --version 2>&1",
        _ => return Ok(None),
    };

    match runner.run(cmd) {
        Ok(output) => {
            #[cfg(debug_assertions)]
            println!(
                "[update] {} command status {:?} stdout={} stderr={}",
                tool,
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );

            if output.status.success() {
                let stdout_str = String::from_utf8_lossy(&output.stdout);
                let stderr_str = String::from_utf8_lossy(&output.stderr);
                let version_output = if !stdout_str.trim().is_empty() {
                    stdout_str.trim().to_string()
                } else {
                    stderr_str.trim().to_string()
                };
                return Ok(extract_version(&version_output));
            }
        }
        Err(err) => {
            #[cfg(debug_assertions)]
            println!("[update] {} command error: {}", tool, err);
        }
    }

    Ok(None)
}

async fn fetch_latest_version_from_npm(package_name: &str) -> AppResult<String> {
    #[cfg(target_os = "windows")]
    let npm_view_output = {
        let mut command = Command::new("npm");
        command.env("PATH", extended_path());
        command.args(["view", package_name, "version"]);
        command.creation_flags(CREATE_NO_WINDOW);
        command.output()
    };

    #[cfg(not(target_os = "windows"))]
    let npm_view_output = {
        let mut command = Command::new("npm");
        command.env("PATH", extended_path());
        command.args(["view", package_name, "version"]);
        command.output()
    };

    if let Ok(output) = npm_view_output {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !version.is_empty() {
                #[cfg(debug_assertions)]
                println!("[update] npm view {} -> {}", package_name, version);
                return Ok(version);
            }
        }
        #[cfg(debug_assertions)]
        println!(
            "[update] npm view failed: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if output_indicates_permission_denied(&output) {
            return Err(permission_denied_error("npm 查询"));
        }
    }

    let client = reqwest::Client::new();
    let mirrors = vec![
        format!("https://registry.npmmirror.com/{}", package_name),
        format!("https://registry.npmjs.org/{}", package_name),
    ];

    for url in mirrors {
        #[cfg(debug_assertions)]
        println!("[update] requesting {}", url);

        let response = client
            .get(&url)
            .header("User-Agent", "DuckCoding-Desktop-App")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        if response.status().is_success() {
            let info = response.json::<crate::models::NpmPackageInfo>().await?;
            #[cfg(debug_assertions)]
            println!("[update] mirror {} -> {}", url, info.dist_tags.latest);
            return Ok(info.dist_tags.latest);
        }
        #[cfg(debug_assertions)]
        println!("[update] mirror {} status {:?}", url, response.status());
    }

    Err(AppError::Other("所有npm镜像源均无法访问".to_string()))
}

fn extract_version(text: &str) -> Option<String> {
    let re = regex::Regex::new(r"(\d+\.\d+\.\d+)").ok()?;
    re.captures(text)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}

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

async fn run_update_command(cmd: &str, args: &[&str]) -> AppResult<std::process::Output> {
    use tokio::process::Command as AsyncCommand;
    use tokio::time::{timeout, Duration};

    let mut command = AsyncCommand::new(cmd);
    command.env("PATH", extended_path());
    command.args(args);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = timeout(Duration::from_secs(120), command.output())
        .await
        .map_err(|_| AppError::Other("更新操作超时，请稍后重试".to_string()))?
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                permission_denied_error("更新操作")
            } else {
                AppError::from(e)
            }
        })?;

    Ok(output)
}

fn install_codex_via_mirror() -> AppResult<InstallResult> {
    #[cfg(target_os = "windows")]
    let command = (
        "powershell",
        vec![
            "-Command",
            "irm https://mirror.duckcoding.com/codex/install.ps1 | iex",
        ],
    );

    #[cfg(not(target_os = "windows"))]
    let command = (
        "sh",
        vec![
            "-c",
            "curl -fsSL https://mirror.duckcoding.com/codex/install.sh | bash",
        ],
    );

    execute_shell_command(command.0, &command.1)
}

fn execute_shell_command(cmd: &str, args: &[&str]) -> AppResult<InstallResult> {
    let mut command = Command::new(cmd);
    command.env("PATH", extended_path());
    command.args(args);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = command.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            permission_denied_error("命令执行")
        } else {
            AppError::from(e)
        }
    })?;

    if output.status.success() {
        Ok(InstallResult {
            success: true,
            message: format!("{} 执行成功", cmd),
            output: String::from_utf8_lossy(&output.stdout).to_string(),
        })
    } else {
        if output_indicates_permission_denied(&output) {
            Err(permission_denied_error("命令执行"))
        } else {
            Err(AppError::command(format!(
                "命令执行失败: {}",
                String::from_utf8_lossy(&output.stderr)
            )))
        }
    }
}
