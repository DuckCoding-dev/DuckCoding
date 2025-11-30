// lib.rs - 暴露服务层给 CLI 和 GUI 使用

pub mod core; // 🆕 核心基础设施层
pub mod http_client;
pub mod models;
pub mod services;
pub mod ui; // 🆕 UI 管理层
pub mod utils;

pub use models::*;
// Explicitly re-export only selected service types to avoid ambiguous glob re-exports
pub use models::InstallMethod; // InstallMethod is defined in models (tool.rs) — re-export from models
pub use services::config::ConfigService;
pub use services::downloader::FileDownloader;
pub use services::installer::InstallerService;
pub use services::proxy::ProxyService;
pub use services::transparent_proxy::{ProxyConfig, TransparentProxyService};
pub use services::transparent_proxy_config::TransparentProxyConfigService;
pub use services::update::UpdateService;
pub use services::version::VersionService;
// Re-export tool status cache and tool registry
pub use services::tool::{ToolRegistry, ToolStatusCache};
// Re-export new proxy architecture types
pub use models::ToolProxyConfig;
pub use services::proxy::{ProxyInstance, ProxyManager, RequestProcessor};
// Re-export session management types
pub use services::session::{ProxySession, SessionEvent, SessionListResponse, SESSION_MANAGER};

// Re-export selected utils items to avoid conflicts with update::PlatformInfo
pub use utils::command::*;
pub use utils::platform::PlatformInfo as SystemPlatformInfo;

// Re-export the correct PlatformInfo from models
pub use models::update::PlatformInfo as UpdatePlatformInfo;

// 重新导出常用类型
pub use anyhow::{Context, Result};

// 🆕 导出核心模块
#[allow(deprecated)]
pub use core::{
    init_logger, set_log_level, update_log_level, AppError, AppResult, ErrorContext, LogConfig,
    LogContext, LogFormat, LogLevel, LogOutput, Timer,
};

// 🆕 导出 UI 管理层
pub use ui::{
    // 托盘管理
    create_tray_menu,
    emit_close_confirm,
    emit_single_instance,
    // 窗口管理
    focus_main_window,
    hide_window_to_tray,
    restore_window_state,
    SingleInstancePayload,
    // 事件管理
    CLOSE_CONFIRM_EVENT,
    SINGLE_INSTANCE_EVENT,
};

/// 应用启动时自动启动符合条件的透明代理
///
/// 条件：`enabled: true` 且 `auto_start: true`
pub async fn auto_start_proxies(manager: &ProxyManager) {
    use utils::config::read_global_config;

    tracing::info!("检查透明代理自启动配置");

    let config = match read_global_config() {
        Ok(Some(cfg)) => cfg,
        Ok(None) => {
            tracing::debug!("未找到全局配置，跳过自启动");
            return;
        }
        Err(e) => {
            tracing::error!(error = ?e, "读取配置失败");
            return;
        }
    };

    let mut started_count = 0;
    let mut failed_count = 0;

    for (tool_id, tool_config) in &config.proxy_configs {
        // 检查是否满足自启动条件
        if !tool_config.enabled || !tool_config.auto_start {
            continue;
        }

        // 检查是否有保护密钥
        if tool_config.local_api_key.is_none() {
            tracing::warn!(tool_id = %tool_id, "未配置保护密钥，跳过自启动");
            continue;
        }

        tracing::info!(
            tool_id = %tool_id,
            port = tool_config.port,
            "自动启动代理"
        );

        match manager.start_proxy(tool_id, tool_config.clone()).await {
            Ok(_) => {
                started_count += 1;
                tracing::info!(tool_id = %tool_id, "代理启动成功");
            }
            Err(e) => {
                failed_count += 1;
                tracing::error!(
                    tool_id = %tool_id,
                    error = ?e,
                    "代理启动失败"
                );
            }
        }
    }

    if started_count > 0 || failed_count > 0 {
        tracing::info!(
            started = started_count,
            failed = failed_count,
            "自启动代理完成"
        );
    } else {
        tracing::debug!("没有配置自启动的代理");
    }
}
