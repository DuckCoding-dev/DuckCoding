// 更新管理相关命令
//
// 包含应用自身的更新检查、下载、安装等功能

use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

use ::duckcoding::models::update::{PackageFormatInfo, PlatformInfo};
use ::duckcoding::services::update::{UpdateInfo, UpdateService, UpdateStatus};

/// 统一管理 UpdateService 的 Tauri State
pub struct UpdateServiceState {
    pub service: Arc<UpdateService>,
}

impl UpdateServiceState {
    pub fn new() -> Self {
        let service = Arc::new(UpdateService::new());
        let service_clone = service.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = service_clone.initialize().await {
                tracing::error!(error = ?e, "更新服务初始化失败");
            }
        });
        Self { service }
    }
}

/// 检查应用更新
#[tauri::command]
pub async fn check_for_app_updates(
    state: State<'_, UpdateServiceState>,
) -> Result<UpdateInfo, String> {
    state
        .service
        .check_for_updates()
        .await
        .map_err(|e| format!("Failed to check for updates: {e}"))
}

/// 下载应用更新
#[tauri::command]
pub async fn download_app_update(
    url: String,
    app: AppHandle,
    state: State<'_, UpdateServiceState>,
) -> Result<String, String> {
    let service = state.service.clone();
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    let window_clone = window.clone();

    service
        .download_update(&url, move |progress| {
            let _ = window_clone.emit("update-download-progress", &progress);
        })
        .await
        .map_err(|e| format!("Failed to download update: {e}"))
}

/// 安装应用更新
#[tauri::command]
pub async fn install_app_update(
    update_path: String,
    state: State<'_, UpdateServiceState>,
) -> Result<(), String> {
    state
        .service
        .install_update(&update_path)
        .await
        .map_err(|e| format!("Failed to install update: {e}"))
}

/// 获取应用更新状态
#[tauri::command]
pub async fn get_app_update_status(
    state: State<'_, UpdateServiceState>,
) -> Result<UpdateStatus, String> {
    Ok(state.service.get_status().await)
}

/// 回滚应用更新
#[tauri::command]
pub async fn rollback_app_update(state: State<'_, UpdateServiceState>) -> Result<(), String> {
    state
        .service
        .rollback_update()
        .await
        .map_err(|e| format!("Failed to rollback update: {e}"))
}

/// 获取当前应用版本
#[tauri::command]
pub async fn get_current_app_version(
    state: State<'_, UpdateServiceState>,
) -> Result<String, String> {
    Ok(state.service.get_current_version().to_string())
}

/// 重启应用以应用更新
#[tauri::command]
pub async fn restart_app_for_update(app: AppHandle) -> Result<(), String> {
    // 立即重启应用
    app.restart();
}

/// 获取平台信息
#[tauri::command]
pub async fn get_platform_info(
    state: State<'_, UpdateServiceState>,
) -> Result<PlatformInfo, String> {
    Ok(state.service.get_platform_info())
}

/// 获取推荐的包格式
#[tauri::command]
pub async fn get_recommended_package_format(
    state: State<'_, UpdateServiceState>,
) -> Result<PackageFormatInfo, String> {
    Ok(state.service.get_recommended_package_format())
}

/// 主动触发检查更新（供托盘菜单和启动时调用）
#[tauri::command]
pub async fn trigger_check_update(
    app: AppHandle,
    state: State<'_, UpdateServiceState>,
) -> Result<(), String> {
    let update_info = state
        .service
        .check_for_updates()
        .await
        .map_err(|e| format!("Failed to check for updates: {e}"))?;

    // 发送事件到前端
    if update_info.has_update {
        app.emit("update-available", &update_info)
            .map_err(|e| format!("Failed to emit update-available event: {e}"))?;
    } else {
        app.emit("update-not-found", &update_info)
            .map_err(|e| format!("Failed to emit update-not-found event: {e}"))?;
    }

    Ok(())
}
