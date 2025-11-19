use tauri::{Manager, WebviewWindow};
use ::duckcoding::ui;

/// 处理窗口关闭操作
///
/// # 参数
/// - `window`: WebviewWindow 实例
/// - `action`: 关闭操作类型 ("minimize" 或 "quit")
#[tauri::command]
pub fn handle_close_action(window: WebviewWindow, action: String) -> Result<(), String> {
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
        other => Err(format!("未知的关闭操作: {}", other)),
    }
}
