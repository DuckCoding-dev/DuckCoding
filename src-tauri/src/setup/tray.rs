use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindow};

#[cfg(not(target_os = "macos"))]
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

/// 创建系统托盘菜单
#[cfg(not(target_os = "macos"))]
pub fn create_tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let check_update_item = MenuItem::with_id(app, "check_update", "检查更新", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &show_item,
            &PredefinedMenuItem::separator(app)?,
            &check_update_item,
            &PredefinedMenuItem::separator(app)?,
            &quit_item,
        ],
    )?;

    Ok(menu)
}

/// 聚焦主窗口
pub fn focus_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        tracing::info!("聚焦主窗口");
        restore_window_state(&window);
    } else {
        tracing::warn!("尝试聚焦时未找到主窗口");
    }
}

/// 恢复窗口状态（跨平台支持）
pub fn restore_window_state<R: Runtime>(window: &WebviewWindow<R>) {
    tracing::debug!(
        is_visible = ?window.is_visible(),
        is_minimized = ?window.is_minimized(),
        "恢复窗口状态"
    );

    #[cfg(target_os = "macos")]
    #[allow(deprecated)]
    {
        use cocoa::appkit::NSApplication;
        use cocoa::base::nil;
        use cocoa::foundation::NSAutoreleasePool;

        unsafe {
            let _pool = NSAutoreleasePool::new(nil);
            let app_macos = NSApplication::sharedApplication(nil);
            app_macos.setActivationPolicy_(
                cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular,
            );
        }
        tracing::debug!("macOS Dock 图标已恢复");
    }

    if let Err(e) = window.show() {
        tracing::error!(error = ?e, "显示窗口失败");
    }
    if let Err(e) = window.unminimize() {
        tracing::error!(error = ?e, "取消最小化窗口失败");
    }
    if let Err(e) = window.set_focus() {
        tracing::error!(error = ?e, "设置窗口焦点失败");
    }

    #[cfg(target_os = "macos")]
    #[allow(deprecated)]
    {
        use cocoa::appkit::NSApplication;
        use cocoa::base::nil;
        use objc::runtime::YES;

        unsafe {
            let ns_app = NSApplication::sharedApplication(nil);
            ns_app.activateIgnoringOtherApps_(YES);
        }
        tracing::debug!("macOS 应用已激活");
    }
}

/// 设置系统托盘（包含事件处理）
#[cfg(not(target_os = "macos"))]
pub fn setup_system_tray<R: Runtime>(app: &tauri::App<R>) -> tauri::Result<()> {
    let tray_menu = create_tray_menu(app.handle())?;
    let app_handle2 = app.handle().clone();

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            tracing::debug!(event_id = ?event.id, "托盘菜单事件");
            match event.id.as_ref() {
                "show" => {
                    tracing::info!("从托盘显示窗口");
                    focus_main_window(app);
                }
                "check_update" => {
                    tracing::info!("从托盘请求检查更新");
                    // 发送检查更新事件到前端
                    if let Err(e) = app.emit("request-check-update", ()) {
                        tracing::error!(error = ?e, "发送更新检查事件失败");
                    }
                }
                "quit" => {
                    tracing::info!("从托盘退出应用");
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(move |_tray, event| {
            tracing::trace!(event = ?event, "托盘图标事件");
            match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    tracing::info!("托盘图标左键点击");
                    focus_main_window(&app_handle2);
                }
                _ => {
                    // 不打印太多日志
                }
            }
        })
        .build(app)?;

    Ok(())
}

const CLOSE_CONFIRM_EVENT: &str = "duckcoding://request-close-action";

/// 设置窗口关闭处理（最小化到托盘而不是退出，跨平台）
pub fn setup_window_close_handler<R: Runtime>(app: &tauri::App<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        let window_clone = window.clone();

        window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                tracing::info!("窗口关闭请求 - 提示用户选择操作");
                // 阻止默认关闭行为
                api.prevent_close();
                if let Err(err) = window_clone.emit(CLOSE_CONFIRM_EVENT, ()) {
                    tracing::error!(
                        error = ?err,
                        "发送关闭确认事件失败，降级为隐藏窗口"
                    );
                    ::duckcoding::ui::hide_window_to_tray(&window_clone);
                }
            }
        });
    }

    Ok(())
}
