// lib.rs - 暴露服务层给 CLI 和 GUI 使用

pub mod core;           // 🆕 核心基础设施层
pub mod ui;             // 🆕 UI 管理层
pub mod http_client;
pub mod models;
pub mod services;
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

// Re-export selected utils items to avoid conflicts with update::PlatformInfo
pub use utils::command::*;
pub use utils::platform::PlatformInfo as SystemPlatformInfo;

// Re-export the correct PlatformInfo from models
pub use models::update::PlatformInfo as UpdatePlatformInfo;

// 重新导出常用类型
pub use anyhow::{Context, Result};

// 🆕 导出核心模块
pub use core::{
    AppError, AppResult, ErrorContext,
    LogConfig, LogLevel, init_logger, set_log_level,
    Timer, LogContext,
};


// 🆕 导出 UI 管理层
pub use ui::{
    // 窗口管理
    focus_main_window,
    restore_window_state,
    hide_window_to_tray,
    // 托盘管理
    create_tray_menu,
    // 事件管理
    CLOSE_CONFIRM_EVENT,
    SINGLE_INSTANCE_EVENT,
    SingleInstancePayload,
    emit_close_confirm,
    emit_single_instance,
};
