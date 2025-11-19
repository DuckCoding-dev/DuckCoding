// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use duckcoding::utils::config::apply_proxy_if_configured;
use serde::Serialize;
use std::env;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime, WebviewWindow,
};

// 导入 commands 模块
mod commands;
use commands::*;

// 导入透明代理服务
use duckcoding::TransparentProxyService;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

const CLOSE_CONFIRM_EVENT: &str = "duckcoding://request-close-action";
const SINGLE_INSTANCE_EVENT: &str = "single-instance";

#[derive(Clone, Serialize)]
struct SingleInstancePayload {
    args: Vec<String>,
    cwd: String,
}

fn create_tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[&show_item, &PredefinedMenuItem::separator(app)?, &quit_item],
    )?;

    Ok(menu)
}

fn focus_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        println!("Focusing existing main window");
        restore_window_state(&window);
    } else {
        println!("Main window not found when trying to focus");
    }
}

fn restore_window_state<R: Runtime>(window: &WebviewWindow<R>) {
    println!(
        "Restoring window state, is_visible={:?}, is_minimized={:?}",
        window.is_visible(),
        window.is_minimized()
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
        println!("macOS Dock icon restored");
    }

    if let Err(e) = window.show() {
        println!("Error showing window: {:?}", e);
    }
    if let Err(e) = window.unminimize() {
        println!("Error unminimizing window: {:?}", e);
    }
    if let Err(e) = window.set_focus() {
        println!("Error setting focus: {:?}", e);
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
        println!("macOS app activated");
    }
}

fn hide_window_to_tray<R: Runtime>(window: &WebviewWindow<R>) {
    println!("Hiding window to system tray");
    if let Err(e) = window.hide() {
        println!("Failed to hide window: {:?}", e);
    }

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
        println!("macOS Dock icon hidden");
    }
}

fn main() {
    // 创建透明代理服务实例
    let transparent_proxy_port = 8787; // 默认端口,实际会从配置读取
    let transparent_proxy_service = TransparentProxyService::new(transparent_proxy_port);
    let transparent_proxy_state = TransparentProxyState {
        service: Arc::new(TokioMutex::new(transparent_proxy_service)),
    };

    let builder = tauri::Builder::default()
        .manage(transparent_proxy_state)
        .setup(|app| {
            // 尝试在应用启动时加载全局配置并应用代理设置,确保子进程继承代理 env
            apply_proxy_if_configured();

            // 设置工作目录到项目根目录(跨平台支持)
            if let Ok(resource_dir) = app.path().resource_dir() {
                println!("Resource dir: {:?}", resource_dir);

                if cfg!(debug_assertions) {
                    // 开发模式: resource_dir 是 src-tauri/target/debug
                    // 需要回到项目根目录(上三级)
                    let project_root = resource_dir
                        .parent() // target
                        .and_then(|p| p.parent()) // src-tauri
                        .and_then(|p| p.parent()) // 项目根目录
                        .unwrap_or(&resource_dir);

                    println!("Development mode, setting dir to: {:?}", project_root);
                    let _ = env::set_current_dir(project_root);
                } else {
                    // 生产模式: 跨平台支持
                    let parent_dir = if cfg!(target_os = "macos") {
                        // macOS: .app/Contents/Resources/
                        resource_dir
                            .parent()
                            .and_then(|p| p.parent())
                            .unwrap_or(&resource_dir)
                    } else if cfg!(target_os = "windows") {
                        // Windows: 通常在应用程序目录
                        resource_dir.parent().unwrap_or(&resource_dir)
                    } else {
                        // Linux: 通常在 /usr/share/appname 或类似位置
                        resource_dir.parent().unwrap_or(&resource_dir)
                    };
                    println!("Production mode, setting dir to: {:?}", parent_dir);
                    let _ = env::set_current_dir(parent_dir);
                }
            }

            println!("Working directory: {:?}", env::current_dir());

            // 创建系统托盘菜单
            let tray_menu = create_tray_menu(app.handle())?;
            let app_handle2 = app.handle().clone();

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| {
                    println!("Tray menu event: {:?}", event.id);
                    match event.id.as_ref() {
                        "show" => {
                            println!("Show window requested from tray menu");
                            focus_main_window(app);
                        }
                        "quit" => {
                            println!("Quit requested from tray menu");
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(move |_tray, event| {
                    println!("Tray icon event received: {:?}", event);
                    match event {
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } => {
                            println!("Tray icon LEFT click detected");
                            focus_main_window(&app_handle2);
                        }
                        _ => {
                            // 不打印太多日志
                        }
                    }
                })
                .build(app)?;

            // 处理窗口关闭事件 - 最小化到托盘而不是退出
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();

                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        println!("Window close requested - prompting for action");
                        // 阻止默认关闭行为
                        api.prevent_close();
                        if let Err(err) = window_clone.emit(CLOSE_CONFIRM_EVENT, ()) {
                            println!(
                                "Failed to emit close confirmation event, fallback to hiding: {:?}",
                                err
                            );
                            hide_window_to_tray(&window_clone);
                        }
                    }
                });
            }

            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            println!(
                "Secondary instance detected, args: {:?}, cwd: {}",
                argv, cwd
            );

            if let Err(err) = app.emit(
                SINGLE_INSTANCE_EVENT,
                SingleInstancePayload {
                    args: argv.clone(),
                    cwd: cwd.clone(),
                },
            ) {
                println!("Failed to emit single-instance event: {:?}", err);
            }

            focus_main_window(app);
        }))
        .invoke_handler(tauri::generate_handler![
            check_installations,
            check_node_environment,
            install_tool,
            check_update,
            check_all_updates,
            update_tool,
            configure_api,
            list_profiles,
            switch_profile,
            delete_profile,
            get_active_config,
            save_global_config,
            get_global_config,
            generate_api_key_for_tool,
            get_usage_stats,
            get_user_quota,
            handle_close_action,
            // expose current proxy for debugging/testing
            get_current_proxy,
            apply_proxy_now,
            test_proxy_request,
            get_claude_settings,
            save_claude_settings,
            get_claude_schema,
            get_codex_settings,
            save_codex_settings,
            get_codex_schema,
            get_gemini_settings,
            save_gemini_settings,
            get_gemini_schema,
            // 透明代理相关命令
            start_transparent_proxy,
            stop_transparent_proxy,
            get_transparent_proxy_status,
            update_transparent_proxy_config,
            // 更新管理相关命令
            check_for_app_updates,
            download_app_update,
            install_app_update,
            get_app_update_status,
            rollback_app_update,
            get_current_app_version,
            restart_app_for_update,
            get_platform_info,
            get_recommended_package_format,
        ]);

    // 使用自定义事件循环处理 macOS Reopen 事件
    builder
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            #[cfg(not(target_os = "macos"))]
            {
                let _ = app_handle;
                let _ = event;
            }
            #[cfg(target_os = "macos")]
            #[allow(deprecated)]
            {
                use cocoa::appkit::NSApplication;
                use cocoa::base::nil;
                use cocoa::foundation::NSAutoreleasePool;
                use objc::runtime::YES;

                if let tauri::RunEvent::Reopen { .. } = event {
                    println!("macOS Reopen event detected");

                    if let Some(window) = app_handle.get_webview_window("main") {
                        unsafe {
                            let _pool = NSAutoreleasePool::new(nil);
                            let app_macos = NSApplication::sharedApplication(nil);
                            app_macos.setActivationPolicy_(cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular);
                        }

                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();

                        unsafe {
                            let ns_app = NSApplication::sharedApplication(nil);
                            ns_app.activateIgnoringOtherApps_(YES);
                        }

                        println!("Window restored from Dock/Cmd+Tab");
                    }
                }
            }
        });
}
