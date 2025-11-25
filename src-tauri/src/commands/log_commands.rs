// 日志配置管理命令
// 提供前端查询和更新日志配置的接口

use duckcoding::models::config::LogConfig;
use duckcoding::utils::config::{read_global_config, write_global_config};
use tauri::command;

/// 检测当前是否为 Release 构建
#[command]
pub fn is_release_build() -> bool {
    !cfg!(debug_assertions)
}

/// 获取当前日志配置
///
/// 从全局配置中读取日志系统配置并返回给前端
#[command]
pub async fn get_log_config() -> Result<LogConfig, String> {
    let global_config = read_global_config().map_err(|e| e.to_string())?;

    match global_config {
        Some(config) => Ok(config.log_config.clone()),
        None => {
            // 配置不存在，返回默认配置
            Ok(LogConfig::default())
        }
    }
}

/// 更新日志配置
///
/// 将新的日志配置保存到全局配置，并判断是否需要重启应用。
/// 仅日志级别变更可以热重载，其他配置项需要重启应用生效。
///
/// # 返回值
/// - 成功：返回提示消息
///   - 可热重载：`"日志配置已更新并生效"`
///   - 需要重启：`"日志配置已保存，需要重启应用后生效"`
/// - 失败：返回错误信息
#[command]
pub async fn update_log_config(new_config: LogConfig) -> Result<String, String> {
    tracing::info!(
        level = new_config.level.as_str(),
        format = ?new_config.format,
        output = ?new_config.output,
        "更新日志配置"
    );

    // 1. 读取当前全局配置
    let mut global_config = read_global_config()
        .map_err(|e| format!("读取配置失败: {}", e))?
        .ok_or_else(|| "配置文件不存在".to_string())?;

    // 2. 检查是否可以热重载
    let old_config = &global_config.log_config;
    let can_hot_reload = old_config.can_hot_reload(&new_config);

    // 3. 保存新配置到文件
    global_config.log_config = new_config.clone();
    write_global_config(&global_config).map_err(|e| format!("保存配置失败: {}", e))?;

    // 4. 如果只是日志级别变更，执行热重载
    if can_hot_reload {
        duckcoding::update_log_level(new_config.level).map_err(|e| format!("热重载失败: {}", e))?;

        tracing::info!("日志配置已热重载");
        Ok("日志配置已更新并生效".to_string())
    } else {
        tracing::warn!("日志配置已保存，但需要重启应用生效");
        Ok("日志配置已保存，需要重启应用后生效".to_string())
    }
}
