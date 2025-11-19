use tauri::{AppHandle, Manager, Runtime, WebviewWindow};

/// 聚焦并显示主窗口
///
/// # 参数
/// - `app`: Tauri 应用句柄
///
/// # 功能
/// - 获取名为 "main" 的主窗口
/// - 调用 restore_window_state 恢复窗口状态
pub fn focus_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        tracing::info!("聚焦主窗口");
        restore_window_state(&window);
    } else {
        tracing::warn!("尝试聚焦时未找到主窗口");
    }
}

/// 恢复窗口状态（从托盘恢复）
///
/// # 参数
/// - `window`: 要恢复的窗口实例
///
/// # 功能
/// - macOS: 恢复 Dock 图标
/// - 显示窗口
/// - 取消最小化
/// - 设置焦点
/// - macOS: 激活应用到前台
pub fn restore_window_state<R: Runtime>(window: &WebviewWindow<R>) {
    tracing::info!(
        visible = ?window.is_visible(),
        minimized = ?window.is_minimized(),
        "恢复窗口状态"
    );

    // macOS: 恢复 Dock 图标
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

    // 显示窗口
    if let Err(e) = window.show() {
        tracing::error!(error = ?e, "显示窗口失败");
    }

    // 取消最小化
    if let Err(e) = window.unminimize() {
        tracing::error!(error = ?e, "取消最小化失败");
    }

    // 设置焦点
    if let Err(e) = window.set_focus() {
        tracing::error!(error = ?e, "设置焦点失败");
    }

    // macOS: 激活应用到前台
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
        tracing::debug!("macOS 应用已激活到前台");
    }
}

/// 隐藏窗口到系统托盘
///
/// # 参数
/// - `window`: 要隐藏的窗口实例
///
/// # 功能
/// - 隐藏窗口
/// - macOS: 隐藏 Dock 图标（应用仅在托盘显示）
pub fn hide_window_to_tray<R: Runtime>(window: &WebviewWindow<R>) {
    tracing::info!("隐藏窗口到系统托盘");

    // 隐藏窗口
    if let Err(e) = window.hide() {
        tracing::error!(error = ?e, "隐藏窗口失败");
    }

    // macOS: 隐藏 Dock 图标
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
                cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory,
            );
        }
        tracing::debug!("macOS Dock 图标已隐藏");
    }
}
