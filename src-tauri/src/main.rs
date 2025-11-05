mod commands;
mod error;
mod models;
mod services;

use std::env;

use commands::{
    check_installations, check_node_environment, check_update, configure_api, delete_profile,
    generate_api_key_for_tool, get_active_config, get_global_config, get_usage_stats,
    get_user_quota, install_tool, list_profiles, save_global_config, switch_profile, update_tool,
};
use error::AppResult;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            setup_working_directory(app)?;
            setup_tray(app)?;
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            check_installations,
            check_node_environment,
            install_tool,
            check_update,
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
            get_user_quota
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn setup_working_directory(app: &tauri::App) -> AppResult<()> {
    if let Ok(resource_dir) = app.path().resource_dir() {
        if cfg!(debug_assertions) {
            if let Some(project_root) = resource_dir
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
            {
                let _ = env::set_current_dir(project_root);
            }
        } else {
            let parent_dir = if cfg!(target_os = "macos") {
                resource_dir
                    .parent()
                    .and_then(|p| p.parent())
                    .unwrap_or(&resource_dir)
            } else {
                resource_dir.parent().unwrap_or(&resource_dir)
            };
            let _ = env::set_current_dir(parent_dir);
        }
    }

    Ok(())
}

fn setup_tray(app: &tauri::App) -> AppResult<()> {
    let tray_menu = create_tray_menu(app.handle())?;
    let app_handle = app.handle().clone();

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(move |_tray, event| match event {
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } => show_main_window(&app_handle),
            _ => {}
        })
        .build(app)?;

    if let Some(window) = app.get_webview_window("main") {
        let window_clone = window.clone();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window_clone.hide();
            }
        });
    }

    Ok(())
}

fn create_tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[&show_item, &PredefinedMenuItem::separator(app)?, &quit_item],
    )
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();

        #[cfg(target_os = "macos")]
        {
            use cocoa::appkit::NSApplication;
            use cocoa::base::nil;
            use objc::runtime::YES;

            unsafe {
                let ns_app = NSApplication::sharedApplication(nil);
                ns_app.activateIgnoringOtherApps_(YES);
            }
        }
    }
}
