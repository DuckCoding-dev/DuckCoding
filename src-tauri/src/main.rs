// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use duckcoding::utils::config::apply_proxy_if_configured;
use serde::Serialize;
use std::env;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime, WebviewWindow,
};

// 导入 commands 模块
mod commands;
use commands::*;

// 导入透明代理服务
use duckcoding::services::config::{NotifyWatcherManager, EXTERNAL_CHANGE_EVENT};
use duckcoding::ProxyManager;
use duckcoding::TransparentProxyService;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

const CLOSE_CONFIRM_EVENT: &str = "duckcoding://request-close-action";
const SINGLE_INSTANCE_EVENT: &str = "single-instance";

struct ExternalWatcherState {
    manager: Mutex<Option<NotifyWatcherManager>>,
}

#[derive(Clone, Serialize)]
struct SingleInstancePayload {
    args: Vec<String>,
    cwd: String,
}

fn create_tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
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

fn focus_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        tracing::info!("聚焦主窗口");
        restore_window_state(&window);
    } else {
        tracing::warn!("尝试聚焦时未找到主窗口");
    }
}

fn restore_window_state<R: Runtime>(window: &WebviewWindow<R>) {
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

fn hide_window_to_tray<R: Runtime>(window: &WebviewWindow<R>) {
    tracing::info!("隐藏窗口到系统托盘");
    if let Err(e) = window.hide() {
        tracing::error!(error = ?e, "隐藏窗口失败");
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
        tracing::debug!("macOS Dock 图标已隐藏");
    }
}

/// 初始化内置 Profile（用于透明代理配置切换）
fn initialize_proxy_profiles() -> Result<(), Box<dyn std::error::Error>> {
    use duckcoding::services::profile_manager::ProfileManager;
    use duckcoding::services::proxy_config_manager::ProxyConfigManager;

    let proxy_mgr = ProxyConfigManager::new()?;
    let profile_mgr = ProfileManager::new()?;

    for tool_id in &["claude-code", "codex", "gemini-cli"] {
        if let Ok(Some(config)) = proxy_mgr.get_config(tool_id) {
            // 只有在配置完整时才创建内置 Profile
            if config.enabled
                && config.local_api_key.is_some()
                && config.real_api_key.is_some()
                && config.real_base_url.is_some()
            {
                let proxy_profile_name = format!("dc_proxy_{}", tool_id.replace("-", "_"));
                let proxy_endpoint = format!("http://127.0.0.1:{}", config.port);
                let proxy_key = config.local_api_key.unwrap();

                let result = match *tool_id {
                    "claude-code" => profile_mgr.save_claude_profile_internal(
                        &proxy_profile_name,
                        proxy_key,
                        proxy_endpoint,
                    ),
                    "codex" => profile_mgr.save_codex_profile_internal(
                        &proxy_profile_name,
                        proxy_key,
                        proxy_endpoint,
                        Some("responses".to_string()),
                    ),
                    "gemini-cli" => profile_mgr.save_gemini_profile_internal(
                        &proxy_profile_name,
                        proxy_key,
                        proxy_endpoint,
                        None, // 不设置 model，保留用户原有配置
                    ),
                    _ => continue,
                };

                if let Err(e) = result {
                    tracing::warn!(
                        tool_id = tool_id,
                        error = ?e,
                        "初始化内置 Profile 失败"
                    );
                } else {
                    tracing::debug!(
                        tool_id = tool_id,
                        profile = %proxy_profile_name,
                        "已初始化内置 Profile"
                    );
                }
            }
        }
    }

    Ok(())
}

fn main() {
    // 🆕 初始化日志系统（必须在最前面）
    use duckcoding::core::init_logger;
    use duckcoding::utils::config::read_global_config;

    // 从配置文件读取日志配置，失败则使用默认配置
    let log_config = read_global_config()
        .ok()
        .flatten()
        .map(|cfg| cfg.log_config)
        .unwrap_or_default();

    if let Err(e) = init_logger(&log_config) {
        // 日志系统初始化失败时使用 eprintln!（因为 tracing 还不可用）
        eprintln!("WARNING: Failed to initialize logging system: {}", e);
        // 继续运行，但日志功能将不可用
    }

    tracing::info!("DuckCoding 应用启动");

    // 🆕 初始化内置 Profile（用于透明代理）
    if let Err(e) = initialize_proxy_profiles() {
        tracing::warn!(error = ?e, "初始化内置 Profile 失败");
    }

    // 创建透明代理服务实例（旧架构，保持兼容）
    let transparent_proxy_port = 8787; // 默认端口,实际会从配置读取
    let transparent_proxy_service = TransparentProxyService::new(transparent_proxy_port);
    let transparent_proxy_state = TransparentProxyState {
        service: Arc::new(TokioMutex::new(transparent_proxy_service)),
    };
    let watcher_state = ExternalWatcherState {
        manager: Mutex::new(None),
    };

    // 创建多工具代理管理器（新架构）
    let proxy_manager = Arc::new(ProxyManager::new());
    let proxy_manager_state = ProxyManagerState {
        manager: proxy_manager.clone(),
    };

    // 异步启动配置了自启动的透明代理
    let proxy_manager_for_auto_start = proxy_manager.clone();
    tauri::async_runtime::spawn(async move {
        duckcoding::auto_start_proxies(&proxy_manager_for_auto_start).await;
    });

    let update_service_state = UpdateServiceState::new();

    // 执行数据迁移（版本驱动）
    tracing::info!("执行数据迁移检查");
    tauri::async_runtime::block_on(async {
        let migration_manager = duckcoding::create_migration_manager();
        match migration_manager.run_all().await {
            Ok(results) => {
                if !results.is_empty() {
                    tracing::info!("迁移执行完成：{} 个迁移", results.len());
                    for result in results {
                        if result.success {
                            tracing::info!("✅ {}: {}", result.migration_id, result.message);
                        } else {
                            tracing::error!("❌ {}: {}", result.migration_id, result.message);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("迁移执行失败: {}", e);
            }
        }
    });

    // 创建工具注册表（统一工具管理系统）
    let tool_registry = tauri::async_runtime::block_on(async {
        duckcoding::ToolRegistry::new()
            .await
            .expect("无法创建工具注册表")
    });
    let tool_registry_state = ToolRegistryState {
        registry: Arc::new(TokioMutex::new(tool_registry)),
    };

    // 判断是否启用单实例模式
    // 开发环境：始终禁用（方便调试和与正式版隔离）
    // 生产环境：根据配置决定（默认启用）
    let single_instance_enabled = if cfg!(debug_assertions) {
        false // 开发环境禁用
    } else {
        // 生产环境读取配置
        read_global_config()
            .ok()
            .flatten()
            .map(|cfg| cfg.single_instance_enabled)
            .unwrap_or(true) // 默认启用
    };

    tracing::info!(
        is_debug = cfg!(debug_assertions),
        single_instance_enabled = single_instance_enabled,
        "单实例模式配置"
    );

    let builder = tauri::Builder::default()
        .manage(transparent_proxy_state)
        .manage(proxy_manager_state)
        .manage(watcher_state)
        .manage(update_service_state)
        .manage(tool_registry_state)
        .setup(|app| {
            // 尝试在应用启动时加载全局配置并应用代理设置,确保子进程继承代理 env
            apply_proxy_if_configured();

            // 设置工作目录到项目根目录(跨平台支持)
            if let Ok(resource_dir) = app.path().resource_dir() {
                tracing::debug!(resource_dir = ?resource_dir, "资源目录");

                if cfg!(debug_assertions) {
                    // 开发模式: resource_dir 是 src-tauri/target/debug
                    // 需要回到项目根目录(上三级)
                    let project_root = resource_dir
                        .parent() // target
                        .and_then(|p| p.parent()) // src-tauri
                        .and_then(|p| p.parent()) // 项目根目录
                        .unwrap_or(&resource_dir);

                    tracing::debug!(project_root = ?project_root, "开发模式，设置工作目录");
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
                    tracing::debug!(parent_dir = ?parent_dir, "生产模式，设置工作目录");
                    let _ = env::set_current_dir(parent_dir);
                }
            }

            tracing::info!(working_dir = ?env::current_dir(), "当前工作目录");

            // 启动通知式配置 watcher（若可用），增加日志方便排查
            if let Some(state) = app.try_state::<ExternalWatcherState>() {
                let enable_watch = match duckcoding::utils::config::read_global_config() {
                    Ok(Some(cfg)) => cfg.external_watch_enabled,
                    _ => true,
                };
                if !enable_watch {
                    tracing::info!("External config watcher disabled by config");
                }

                if let Ok(mut guard) = state.manager.lock() {
                    if guard.is_none() && enable_watch {
                        match NotifyWatcherManager::start_all(app.handle().clone()) {
                            Ok(manager) => {
                                tracing::debug!(
                                    "Config notify watchers started, emitting event {EXTERNAL_CHANGE_EVENT}"
                                );
                                *guard = Some(manager);
                            }
                            Err(err) => {
                                tracing::error!("Failed to start notify watchers: {err:?}");
                            }
                        }
                    } else {
                        tracing::info!(
                            already_running = guard.is_some(),
                            enable_watch,
                            "Skip starting notify watcher"
                        );
                    }
                }
            }

            // 创建系统托盘菜单
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

            // 处理窗口关闭事件 - 最小化到托盘而不是退出
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
                            hide_window_to_tray(&window_clone);
                        }
                    }
                });
            }

            // 启动后延迟检查更新
            let app_handle_for_update = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // 延迟1秒，避免影响启动速度
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                tracing::info!("启动时自动检查更新");

                // 获取 UpdateServiceState 并检查更新
                let state = app_handle_for_update.state::<UpdateServiceState>();
                match state.service.check_for_updates().await {
                    Ok(update_info) => {
                        if update_info.has_update {
                            tracing::info!(
                                version = %update_info.latest_version,
                                "发现新版本"
                            );
                            if let Err(e) =
                                app_handle_for_update.emit("update-available", &update_info)
                            {
                                tracing::error!(error = ?e, "发送更新可用事件失败");
                            }
                        } else {
                            tracing::debug!("当前已是最新版本");
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = ?e, "启动时检查更新失败");
                    }
                }
            });

            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init());

    // 条件注册单实例插件
    let builder = if single_instance_enabled {
        tracing::info!("注册单实例插件");
        builder.plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            tracing::info!(
                argv = ?argv,
                cwd = %cwd,
                "检测到第二个实例"
            );

            if let Err(err) = app.emit(
                SINGLE_INSTANCE_EVENT,
                SingleInstancePayload {
                    args: argv.clone(),
                    cwd: cwd.clone(),
                },
            ) {
                tracing::error!(error = ?err, "发送单实例事件失败");
            }

            focus_main_window(app);
        }))
    } else {
        tracing::info!("单实例插件已禁用（开发环境或用户配置）");
        builder
    };

    let builder = builder.invoke_handler(tauri::generate_handler![
        check_installations,
        refresh_tool_status,
        check_node_environment,
        install_tool,
        check_update,
        check_update_for_instance,
        refresh_all_tool_versions,
        check_all_updates,
        update_tool_instance,
        validate_tool_path,
        add_manual_tool_instance,
        scan_installer_for_tool_path,
        scan_all_tool_candidates,
        detect_single_tool,
        detect_tool_without_save,
        // 请使用新的 pm_ 系列命令
        save_global_config,
        get_global_config,
        generate_api_key_for_tool,
        get_external_changes,
        ack_external_change,
        import_native_change,
        get_usage_stats,
        get_user_quota,
        fetch_api,
        load_balance_configs,
        save_balance_config,
        update_balance_config,
        delete_balance_config,
        migrate_balance_from_localstorage,
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
        // 多工具透明代理命令（新架构）
        start_tool_proxy,
        stop_tool_proxy,
        get_all_proxy_status,
        update_proxy_from_profile,
        get_proxy_config,
        update_proxy_config,
        get_all_proxy_configs,
        // 会话管理命令
        get_session_list,
        delete_session,
        clear_all_sessions,
        update_session_config,
        update_session_note,
        // 配置监听控制
        get_watcher_status,
        start_watcher_if_needed,
        stop_watcher,
        save_watcher_settings,
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
        trigger_check_update,
        // 日志管理命令
        get_log_config,
        update_log_config,
        is_release_build,
        // 工具管理命令（工具管理系统）
        get_tool_instances,
        refresh_tool_instances,
        list_wsl_distributions,
        add_wsl_tool_instance,
        add_ssh_tool_instance,
        delete_tool_instance,
        // 引导管理命令
        get_onboarding_status,
        save_onboarding_progress,
        complete_onboarding,
        reset_onboarding,
        // 单实例模式配置命令
        get_single_instance_config,
        update_single_instance_config,
        // Profile 管理命令（v2.0）
        pm_list_all_profiles,
        pm_list_tool_profiles,
        pm_get_profile,
        pm_save_profile,
        pm_delete_profile,
        pm_activate_profile,
        pm_get_active_profile_name,
        pm_get_active_profile,
        pm_capture_from_native,
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
                    tracing::info!("macOS Reopen 事件");

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

                        tracing::debug!("从 Dock/Cmd+Tab 恢复窗口");
                    }
                }
            }
        });
}
