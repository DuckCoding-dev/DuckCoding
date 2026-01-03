// filepath: src-tauri/src/commands/startup_commands.rs

//! 开机自启动管理命令
//!
//! 提供前端调用的开机自启动配置管理接口

use duckcoding::utils::auto_startup::{
    disable_auto_startup, enable_auto_startup, is_auto_startup_enabled,
};
use duckcoding::utils::config::{read_global_config, write_global_config};

/// 获取开机自启动配置
///
/// 返回当前配置状态，并自动同步系统实际状态
#[tauri::command]
pub async fn get_startup_config() -> Result<bool, String> {
    // 读取配置文件中的状态
    let config_opt = read_global_config().map_err(|e| e.to_string())?;

    // 检查系统实际状态
    let system_enabled = is_auto_startup_enabled().map_err(|e| e.to_string())?;

    // 如果配置不存在，返回系统状态
    let Some(mut config) = config_opt else {
        return Ok(system_enabled);
    };

    // 如果配置与系统状态不一致，以系统状态为准并更新配置
    if config.startup_enabled != system_enabled {
        config.startup_enabled = system_enabled;
        write_global_config(&config).map_err(|e| e.to_string())?;
    }

    Ok(config.startup_enabled)
}

/// 更新开机自启动配置
///
/// # 参数
/// - `enabled`: true 表示启用自启动，false 表示禁用
///
/// # 返回
/// - 成功返回 Ok(())
/// - 失败返回 Err(错误信息)
#[tauri::command]
pub async fn update_startup_config(enabled: bool) -> Result<(), String> {
    // 根据参数调用系统API
    if enabled {
        enable_auto_startup().map_err(|e| e.to_string())?;
    } else {
        disable_auto_startup().map_err(|e| e.to_string())?;
    }

    // 更新配置文件
    let config_opt = read_global_config().map_err(|e| e.to_string())?;

    // 如果配置不存在，只更新系统设置不保存到配置文件
    let Some(mut config) = config_opt else {
        return Ok(());
    };

    config.startup_enabled = enabled;
    write_global_config(&config).map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_startup_config() {
        // 测试读取配置（不进行实际系统操作）
        let result = get_startup_config().await;
        // 应该能正常读取，无论启用与否
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // 需要手动测试，避免污染系统
    async fn test_update_startup_config() {
        // 测试启用
        let result = update_startup_config(true).await;
        assert!(result.is_ok());

        // 检查状态
        let enabled = get_startup_config().await.unwrap();
        assert!(enabled);

        // 测试禁用
        let result = update_startup_config(false).await;
        assert!(result.is_ok());

        // 再次检查状态
        let enabled = get_startup_config().await.unwrap();
        assert!(!enabled);
    }
}
