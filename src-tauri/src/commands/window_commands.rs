use crate::commands::error::{AppError, AppResult};
use ::duckcoding::ui;
use tauri::{AppHandle, Manager, WebviewWindow};

/// 处理窗口关闭操作
///
/// # 参数
/// - `window`: WebviewWindow 实例
/// - `action`: 关闭操作类型 ("minimize" 或 "quit")
#[tauri::command]
pub fn handle_close_action(window: WebviewWindow, action: String) -> AppResult<()> {
    match action.as_str() {
        "minimize" => {
            // 隐藏到托盘
            ui::hide_window_to_tray(&window);
            Ok(())
        }
        "quit" => {
            window.app_handle().exit(0);
            Ok(())
        }
        other => Err(AppError::ValidationError {
            field: "action".to_string(),
            reason: format!("未知的关闭操作: {}", other),
        }),
    }
}

/// 刷新应用菜单栏（仅 macOS）
#[tauri::command]
pub fn refresh_app_menu(app: AppHandle) -> AppResult<()> {
    #[cfg(target_os = "macos")]
    {
        crate::setup::menu::refresh_app_menu(&app).map_err(|e| AppError::Internal {
            message: format!("刷新菜单失败: {}", e),
        })?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
    }
    Ok(())
}
