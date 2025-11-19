// 更新管理相关命令
//
// 包含应用自身的更新检查、下载、安装等功能

use tauri::{AppHandle, Emitter, Manager};

use ::duckcoding::models::update::{PackageFormatInfo, PlatformInfo};
use ::duckcoding::services::update::{UpdateInfo, UpdateService, UpdateStatus};

// 全局更新服务实例
static UPDATE_SERVICE: std::sync::OnceLock<std::sync::Arc<UpdateService>> =
    std::sync::OnceLock::new();

fn get_update_service() -> std::sync::Arc<UpdateService> {
    UPDATE_SERVICE
        .get_or_init(|| {
            let service = std::sync::Arc::new(UpdateService::new());
            // 初始化更新服务
            let service_clone = service.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = service_clone.initialize().await {
                    eprintln!("Failed to initialize update service: {}", e);
                }
            });
            service
        })
        .clone()
}

/// 检查应用更新
#[tauri::command]
pub async fn check_for_app_updates() -> Result<UpdateInfo, String> {
    let service = get_update_service();
    service
        .check_for_updates()
        .await
        .map_err(|e| format!("Failed to check for updates: {}", e))
}

/// 下载应用更新
#[tauri::command]
pub async fn download_app_update(url: String, app: AppHandle) -> Result<String, String> {
    let service = get_update_service();
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    let _service_clone = service.clone();
    let window_clone = window.clone();

    service
        .download_update(&url, move |progress| {
            let _ = window_clone.emit("update-download-progress", &progress);
        })
        .await
        .map_err(|e| format!("Failed to download update: {}", e))
}

/// 安装应用更新
#[tauri::command]
pub async fn install_app_update(update_path: String) -> Result<(), String> {
    let service = get_update_service();
    service
        .install_update(&update_path)
        .await
        .map_err(|e| format!("Failed to install update: {}", e))
}

/// 获取应用更新状态
#[tauri::command]
pub async fn get_app_update_status() -> Result<UpdateStatus, String> {
    let service = get_update_service();
    Ok(service.get_status().await)
}

/// 回滚应用更新
#[tauri::command]
pub async fn rollback_app_update() -> Result<(), String> {
    let service = get_update_service();
    service
        .rollback_update()
        .await
        .map_err(|e| format!("Failed to rollback update: {}", e))
}

/// 获取当前应用版本
#[tauri::command]
pub async fn get_current_app_version() -> Result<String, String> {
    let service = get_update_service();
    Ok(service.get_current_version().to_string())
}

/// 重启应用以应用更新
#[tauri::command]
pub async fn restart_app_for_update(app: AppHandle) -> Result<(), String> {
    // 立即重启应用
    app.restart();
}

/// 获取平台信息
#[tauri::command]
pub async fn get_platform_info() -> Result<PlatformInfo, String> {
    let service = get_update_service();
    Ok(service.get_platform_info())
}

/// 获取推荐的包格式
#[tauri::command]
pub async fn get_recommended_package_format() -> Result<PackageFormatInfo, String> {
    let service = get_update_service();
    Ok(service.get_recommended_package_format())
}
