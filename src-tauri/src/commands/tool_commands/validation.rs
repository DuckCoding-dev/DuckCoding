use crate::commands::tool_management::ToolRegistryState;
use crate::commands::types::NodeEnvironment;
use ::duckcoding::utils::platform::PlatformInfo;
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
/// 1. 委托给 ToolRegistry.validate_tool_path
/// 2. Registry 负责检查文件存在性、执行版本命令
///
/// 返回：版本号字符串
#[tauri::command]
pub async fn validate_tool_path(
    _tool_id: String,
    path: String,
    registry_state: tauri::State<'_, ToolRegistryState>,
) -> Result<String, String> {
    let registry = registry_state.registry.lock().await;
    registry
        .validate_tool_path(&path)
        .await
        .map_err(|e| e.to_string())
}
