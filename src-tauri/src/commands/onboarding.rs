// filepath: e:\DuckCoding\src-tauri\src\commands\onboarding.rs

use duckcoding::models::config::{GlobalConfig, LogConfig, OnboardingStatus};
use duckcoding::utils::config::{read_global_config, write_global_config};
use std::collections::HashMap;
use tracing::{error, info};

/// 创建最小默认配置（仅用于首次启动）
fn create_minimal_config() -> GlobalConfig {
    GlobalConfig {
        version: Some("0.0.0".to_string()),
        user_id: String::new(),
        system_token: String::new(),
        proxy_enabled: false,
        proxy_type: None,
        proxy_host: None,
        proxy_port: None,
        proxy_username: None,
        proxy_password: None,
        proxy_bypass_urls: Vec::new(),
        proxy_configs: HashMap::new(),
        session_endpoint_config_enabled: false,
        hide_transparent_proxy_tip: false,
        hide_session_config_hint: false,
        log_config: LogConfig::default(),
        onboarding_status: None,
        external_watch_enabled: true,
        external_poll_interval_ms: 5000,
        single_instance_enabled: true,
        startup_enabled: false,
    }
}

/// 获取当前引导状态
#[tauri::command]
pub async fn get_onboarding_status() -> Result<Option<OnboardingStatus>, String> {
    info!("获取引导状态");

    let config = read_global_config().map_err(|e| {
        error!("读取配置失败: {}", e);
        format!("读取配置失败: {}", e)
    })?;

    Ok(config.and_then(|c| c.onboarding_status))
}

/// 保存引导进度（用于记录跳过的步骤）
#[tauri::command]
pub async fn save_onboarding_progress(
    version: String,
    skipped_steps: Vec<String>,
) -> Result<(), String> {
    info!(
        "保存引导进度: version={}, skipped_steps={:?}",
        version, skipped_steps
    );

    // 读取或创建最小默认配置
    let mut config = read_global_config()
        .map_err(|e| {
            error!("读取配置失败: {}", e);
            format!("读取配置失败: {}", e)
        })?
        .unwrap_or_else(|| {
            info!("配置文件不存在，创建最小默认配置");
            create_minimal_config()
        });

    // 更新引导状态（未完成，只记录进度）
    config.onboarding_status = Some(OnboardingStatus {
        completed_version: version.clone(),
        skipped_steps: skipped_steps.clone(),
        completed_at: None, // 未完成
    });

    write_global_config(&config).map_err(|e| {
        error!("写入配置失败: {}", e);
        format!("写入配置失败: {}", e)
    })?;

    info!("引导进度已保存");
    Ok(())
}

/// 完成引导流程
#[tauri::command]
pub async fn complete_onboarding(version: String) -> Result<(), String> {
    info!("完成引导: version={}", version);

    // 读取或创建最小默认配置
    let mut config = read_global_config()
        .map_err(|e| {
            error!("读取配置失败: {}", e);
            format!("读取配置失败: {}", e)
        })?
        .unwrap_or_else(|| {
            info!("配置文件不存在，创建最小默认配置");
            create_minimal_config()
        });

    // 记录完成时间（ISO 8601 格式）
    let completed_at = chrono::Utc::now().to_rfc3339();

    // 更新引导状态
    config.onboarding_status = Some(OnboardingStatus {
        completed_version: version.clone(),
        skipped_steps: config
            .onboarding_status
            .as_ref()
            .map(|s| s.skipped_steps.clone())
            .unwrap_or_default(),
        completed_at: Some(completed_at),
    });

    write_global_config(&config).map_err(|e| {
        error!("写入配置失败: {}", e);
        format!("写入配置失败: {}", e)
    })?;

    info!("引导已完成并保存");
    Ok(())
}

/// 重置引导状态（用于设置页重新打开引导）
#[tauri::command]
pub async fn reset_onboarding() -> Result<(), String> {
    info!("重置引导状态");

    let mut config = read_global_config()
        .map_err(|e| {
            error!("读取配置失败: {}", e);
            format!("读取配置失败: {}", e)
        })?
        .ok_or_else(|| {
            error!("配置文件不存在");
            "配置文件不存在".to_string()
        })?;

    // 清空引导状态
    config.onboarding_status = None;

    write_global_config(&config).map_err(|e| {
        error!("写入配置失败: {}", e);
        format!("写入配置失败: {}", e)
    })?;

    info!("引导状态已重置");
    Ok(())
}
