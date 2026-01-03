// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use duckcoding::services::config::{NotifyWatcherManager, EXTERNAL_CHANGE_EVENT};
use duckcoding::services::proxy::config::apply_global_proxy;
use duckcoding::utils::config::read_global_config;
use serde::Serialize;
use std::env;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

// 导入 commands 模块
mod commands;
use commands::*;

// 导入 setup 模块
mod setup;

const SINGLE_INSTANCE_EVENT: &str = "single-instance";

struct ExternalWatcherState {
    manager: Mutex<Option<NotifyWatcherManager>>,
}

#[derive(Clone, Serialize)]
struct SingleInstancePayload {
    args: Vec<String>,
    cwd: String,
}

/// 判断是否启用单实例模式
///
/// 开发环境：始终禁用（方便调试和与正式版隔离）
/// 生产环境：根据配置决定（默认启用）
fn determine_single_instance_mode() -> bool {
    if cfg!(debug_assertions) {
        false // 开发环境禁用
    } else {
        // 生产环境读取配置
        read_global_config()
            .ok()
            .flatten()
            .map(|cfg| cfg.single_instance_enabled)
            .unwrap_or(true) // 默认启用
    }
}

/// 设置工作目录到项目根目录（跨平台支持）
fn setup_working_directory(app: &tauri::App) -> tauri::Result<()> {
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
    Ok(())
}

/// 启动配置文件监听（如果启用）
fn start_config_watcher(app: &tauri::App) -> tauri::Result<()> {
    if let Some(state) = app.try_state::<ExternalWatcherState>() {
        let enable_watch = match read_global_config() {
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

    Ok(())
}

/// 延迟检查应用更新
fn schedule_update_check(app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        // 延迟1秒，避免影响启动速度
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        tracing::info!("启动时自动检查更新");

        // 获取 UpdateServiceState 并检查更新
        let state = app_handle.state::<UpdateServiceState>();
        match state.service.check_for_updates().await {
            Ok(update_info) => {
                if update_info.has_update {
                    tracing::info!(
                        version = %update_info.latest_version,
                        "发现新版本"
                    );
                    if let Err(e) = app_handle.emit("update-available", &update_info) {
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
}

/// 执行应用启动钩子（setup）
fn setup_app_hooks(app: &mut tauri::App) -> tauri::Result<()> {
    // 1. 应用代理配置
    apply_global_proxy().ok();

    // 2. 设置工作目录
    setup_working_directory(app)?;

    // 3. 启动配置监听
    start_config_watcher(app)?;

    // 4. 创建系统托盘
    setup::tray::setup_system_tray(app)?;

    // 5. 处理窗口关闭事件
    setup::tray::setup_window_close_handler(app)?;

    // 6. 启动后检查更新
    schedule_update_check(app.handle().clone());

    Ok(())
}

fn main() {
    // 使用封装的初始化函数
    let init_ctx = tauri::async_runtime::block_on(async {
        setup::initialize_app().await.expect("应用初始化失败")
    });

    let watcher_state = ExternalWatcherState {
        manager: Mutex::new(None),
    };

    let proxy_manager_state = ProxyManagerState {
        manager: init_ctx.proxy_manager,
    };

    let update_service_state = UpdateServiceState::new();

    let tool_registry_state = ToolRegistryState {
        registry: init_ctx.tool_registry,
    };

    let profile_manager_state = ProfileManagerState {
        manager: init_ctx.profile_manager,
    };

    // 判断单实例模式
    let single_instance_enabled = determine_single_instance_mode();

    tracing::info!(
        is_debug = cfg!(debug_assertions),
        single_instance_enabled = single_instance_enabled,
        "单实例模式配置"
    );

    let builder = tauri::Builder::default()
        .manage(proxy_manager_state)
        .manage(watcher_state)
        .manage(update_service_state)
        .manage(tool_registry_state)
        .manage(profile_manager_state)
        .setup(|app| {
            setup_app_hooks(app)?;
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

            setup::focus_main_window(app);
        }))
    } else {
        tracing::info!("单实例插件已禁用（开发环境或用户配置）");
        builder
    };

    // 注册所有 Tauri 命令（按功能分组）
    let builder = builder.invoke_handler(tauri::generate_handler![
        // 工具检测与状态管理
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
        // 全局配置管理
        save_global_config,
        get_global_config,
        generate_api_key_for_tool,
        get_external_changes,
        ack_external_change,
        import_native_change,
        // 使用统计
        get_usage_stats,
        get_user_quota,
        // API 请求
        fetch_api,
        // 余额监控
        load_balance_configs,
        save_balance_config,
        update_balance_config,
        delete_balance_config,
        migrate_balance_from_localstorage,
        // 窗口管理
        handle_close_action,
        // 代理调试
        get_current_proxy,
        apply_proxy_now,
        test_proxy_request,
        // Claude Code 配置
        get_claude_settings,
        save_claude_settings,
        get_claude_schema,
        // Codex 配置
        get_codex_settings,
        save_codex_settings,
        get_codex_schema,
        // Gemini CLI 配置
        get_gemini_settings,
        save_gemini_settings,
        get_gemini_schema,
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
        // 开机自启动管理命令
        get_startup_config,
        update_startup_config,
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
