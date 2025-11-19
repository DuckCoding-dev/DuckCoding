use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    AppHandle, Runtime,
};

/// 创建系统托盘菜单
///
/// # 参数
/// - `app`: Tauri 应用句柄
///
/// # 返回
/// - 包含 "显示窗口"、分隔符、"退出" 的菜单
///
/// # 示例
/// ```rust
/// let tray_menu = create_tray_menu(&app)?;
/// ```
pub fn create_tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    tracing::debug!("创建系统托盘菜单");

    // 创建菜单项
    let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    // 组装菜单：显示窗口 | --- | 退出
    let menu = Menu::with_items(
        app,
        &[&show_item, &PredefinedMenuItem::separator(app)?, &quit_item],
    )?;

    tracing::info!("系统托盘菜单创建成功");
    Ok(menu)
}
