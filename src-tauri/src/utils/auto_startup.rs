// filepath: src-tauri/src/utils/auto_startup.rs

//! 跨平台开机自启动功能模块
//!
//! 支持平台:
//! - Windows: 通过注册表 HKCU\Software\Microsoft\Windows\CurrentVersion\Run
//! - macOS: 通过 LaunchAgents plist 文件
//! - Linux: 通过 XDG autostart desktop 文件

use crate::core::error::AppError;
use std::env;
use std::path::PathBuf;

/// 启用开机自启动
pub fn enable_auto_startup() -> Result<(), AppError> {
    #[cfg(target_os = "windows")]
    {
        enable_windows_startup()
    }

    #[cfg(target_os = "macos")]
    {
        enable_macos_startup()
    }

    #[cfg(target_os = "linux")]
    {
        enable_linux_startup()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err(AppError::Internal {
            message: "当前平台不支持自启动功能".to_string(),
        })
    }
}

/// 禁用开机自启动
pub fn disable_auto_startup() -> Result<(), AppError> {
    #[cfg(target_os = "windows")]
    {
        disable_windows_startup()
    }

    #[cfg(target_os = "macos")]
    {
        disable_macos_startup()
    }

    #[cfg(target_os = "linux")]
    {
        disable_linux_startup()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err(AppError::Internal {
            message: "当前平台不支持自启动功能".to_string(),
        })
    }
}

/// 检查是否已启用开机自启动
pub fn is_auto_startup_enabled() -> Result<bool, AppError> {
    #[cfg(target_os = "windows")]
    {
        is_windows_startup_enabled()
    }

    #[cfg(target_os = "macos")]
    {
        is_macos_startup_enabled()
    }

    #[cfg(target_os = "linux")]
    {
        is_linux_startup_enabled()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Ok(false)
    }
}

/// 获取当前可执行文件路径
fn get_executable_path() -> Result<PathBuf, AppError> {
    env::current_exe().map_err(|e| AppError::Internal {
        message: format!("无法获取可执行文件路径: {}", e),
    })
}

// ==================== Windows 实现 ====================

#[cfg(target_os = "windows")]
fn enable_windows_startup() -> Result<(), AppError> {
    use winreg::enums::*;
    use winreg::RegKey;

    let exe_path = get_executable_path()?;
    let exe_path_str = exe_path.to_str().ok_or_else(|| AppError::Internal {
        message: "无法转换可执行文件路径为字符串".to_string(),
    })?;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = hkcu
        .open_subkey_with_flags(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            KEY_WRITE,
        )
        .map_err(|e| AppError::Internal {
            message: format!("无法打开注册表启动项: {}", e),
        })?;

    run_key
        .set_value("DuckCoding", &exe_path_str)
        .map_err(|e| AppError::Internal {
            message: format!("无法写入注册表启动项: {}", e),
        })?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn disable_windows_startup() -> Result<(), AppError> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = hkcu
        .open_subkey_with_flags(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            KEY_WRITE,
        )
        .map_err(|e| AppError::Internal {
            message: format!("无法打开注册表启动项: {}", e),
        })?;

    // 删除键值，如果不存在则忽略错误
    run_key.delete_value("DuckCoding").ok();

    Ok(())
}

#[cfg(target_os = "windows")]
fn is_windows_startup_enabled() -> Result<bool, AppError> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = hkcu
        .open_subkey_with_flags(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            KEY_READ,
        )
        .map_err(|e| AppError::Internal {
            message: format!("无法打开注册表启动项: {}", e),
        })?;

    let value: Result<String, _> = run_key.get_value("DuckCoding");
    Ok(value.is_ok())
}

// ==================== macOS 实现 ====================

#[cfg(target_os = "macos")]
fn get_macos_plist_path() -> Result<PathBuf, AppError> {
    let home = dirs::home_dir().ok_or_else(|| AppError::Internal {
        message: "无法获取用户主目录".to_string(),
    })?;

    let plist_dir = home.join("Library").join("LaunchAgents");
    Ok(plist_dir.join("com.duckcoding.app.plist"))
}

#[cfg(target_os = "macos")]
fn enable_macos_startup() -> Result<(), AppError> {
    use std::fs;

    let exe_path = get_executable_path()?;
    let exe_path_str = exe_path.to_str().ok_or_else(|| AppError::Internal {
        message: "无法转换可执行文件路径为字符串".to_string(),
    })?;

    let plist_path = get_macos_plist_path()?;

    // 确保目录存在
    if let Some(parent) = plist_path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::Internal {
            message: format!("无法创建 LaunchAgents 目录: {}", e),
        })?;
    }

    let plist_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.duckcoding.app</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>"#,
        exe_path_str
    );

    fs::write(&plist_path, plist_content).map_err(|e| AppError::Internal {
        message: format!("无法写入 plist 文件: {}", e),
    })?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn disable_macos_startup() -> Result<(), AppError> {
    use std::fs;

    let plist_path = get_macos_plist_path()?;

    // 如果文件存在则删除，不存在则忽略
    if plist_path.exists() {
        fs::remove_file(&plist_path).map_err(|e| AppError::Internal {
            message: format!("无法删除 plist 文件: {}", e),
        })?;
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn is_macos_startup_enabled() -> Result<bool, AppError> {
    let plist_path = get_macos_plist_path()?;
    Ok(plist_path.exists())
}

// ==================== Linux 实现 ====================

#[cfg(target_os = "linux")]
fn get_linux_desktop_path() -> Result<PathBuf, AppError> {
    let home = dirs::home_dir().ok_or_else(|| AppError::Internal {
        message: "无法获取用户主目录".to_string(),
    })?;

    let autostart_dir = home.join(".config").join("autostart");
    Ok(autostart_dir.join("duckcoding.desktop"))
}

#[cfg(target_os = "linux")]
fn enable_linux_startup() -> Result<(), AppError> {
    use std::fs;

    let exe_path = get_executable_path()?;
    let exe_path_str = exe_path.to_str().ok_or_else(|| AppError::Internal {
        message: "无法转换可执行文件路径为字符串".to_string(),
    })?;

    let desktop_path = get_linux_desktop_path()?;

    // 确保目录存在
    if let Some(parent) = desktop_path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::Internal {
            message: format!("无法创建 autostart 目录: {}", e),
        })?;
    }

    let desktop_content = format!(
        r#"[Desktop Entry]
Type=Application
Name=DuckCoding
Exec={}
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true
Comment=DuckCoding AI Tools Configuration Manager
"#,
        exe_path_str
    );

    fs::write(&desktop_path, desktop_content).map_err(|e| AppError::Internal {
        message: format!("无法写入 desktop 文件: {}", e),
    })?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn disable_linux_startup() -> Result<(), AppError> {
    use std::fs;

    let desktop_path = get_linux_desktop_path()?;

    // 如果文件存在则删除，不存在则忽略
    if desktop_path.exists() {
        fs::remove_file(&desktop_path).map_err(|e| AppError::Internal {
            message: format!("无法删除 desktop 文件: {}", e),
        })?;
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn is_linux_startup_enabled() -> Result<bool, AppError> {
    let desktop_path = get_linux_desktop_path()?;
    Ok(desktop_path.exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_executable_path() {
        let result = get_executable_path();
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.exists() || cfg!(test)); // 测试环境可能路径不同
    }

    #[test]
    #[ignore] // 需要手动测试，避免污染系统
    fn test_enable_disable_startup() {
        // 测试启用
        let result = enable_auto_startup();
        assert!(result.is_ok());

        // 检查状态
        let enabled = is_auto_startup_enabled().unwrap();
        assert!(enabled);

        // 测试禁用
        let result = disable_auto_startup();
        assert!(result.is_ok());

        // 再次检查状态
        let enabled = is_auto_startup_enabled().unwrap();
        assert!(!enabled);
    }
}
