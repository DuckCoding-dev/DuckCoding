use crate::commands::types::NodeEnvironment;
use ::duckcoding::utils::platform::PlatformInfo;
use std::path::PathBuf;
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// 检测 Node.js 和 npm 环境
#[tauri::command]
pub async fn check_node_environment() -> Result<NodeEnvironment, String> {
    let enhanced_path = PlatformInfo::current().build_enhanced_path();
    let run_command = |cmd: &str| -> Result<std::process::Output, std::io::Error> {
        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .env("PATH", &enhanced_path)
                .arg("/C")
                .arg(cmd)
                .creation_flags(0x08000000) // CREATE_NO_WINDOW - 隐藏终端窗口
                .output()
        }
        #[cfg(not(target_os = "windows"))]
        {
            Command::new("sh")
                .env("PATH", &enhanced_path)
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

/// 验证用户指定的工具路径是否有效
///
/// 工作流程：
/// 1. 检查文件是否存在
/// 2. 执行 --version 命令
/// 3. 解析版本号
///
/// 返回：版本号字符串
#[tauri::command]
pub async fn validate_tool_path(_tool_id: String, path: String) -> Result<String, String> {
    let path_buf = PathBuf::from(&path);

    // 检查文件是否存在
    if !path_buf.exists() {
        return Err(format!("路径不存在: {}", path));
    }

    // 检查是否是文件
    if !path_buf.is_file() {
        return Err(format!("路径不是文件: {}", path));
    }

    // 执行 --version 命令
    let version_cmd = format!("{} --version", path);

    #[cfg(target_os = "windows")]
    let output = Command::new("cmd")
        .arg("/C")
        .arg(&version_cmd)
        .output()
        .map_err(|e| format!("执行命令失败: {}", e))?;

    #[cfg(not(target_os = "windows"))]
    let output = Command::new("sh")
        .arg("-c")
        .arg(&version_cmd)
        .output()
        .map_err(|e| format!("执行命令失败: {}", e))?;

    if !output.status.success() {
        return Err(format!("命令执行失败，退出码: {}", output.status));
    }

    // 解析版本号
    let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if version_str.is_empty() {
        return Err("无法获取版本信息".to_string());
    }

    // 简单验证：版本号应该包含数字
    if !version_str.chars().any(|c| c.is_numeric()) {
        return Err(format!("无效的版本信息: {}", version_str));
    }

    Ok(version_str)
}
