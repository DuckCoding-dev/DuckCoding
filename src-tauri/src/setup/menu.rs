//! macOS 应用菜单栏模块
//!
//! 提供 Profile 快捷切换功能，仅在 macOS 下启用

use tauri::{
    menu::{
        CheckMenuItem, Menu, MenuBuilder, MenuItem, PredefinedMenuItem, Submenu, SubmenuBuilder,
    },
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, Runtime,
};

use crate::commands::ProfileManagerState;
use duckcoding::services::profile_manager::ProfileManager;

/// Profile 菜单项 ID 前缀
const PROFILE_MENU_PREFIX: &str = "profile:";

/// 工具显示名称
fn tool_display_name(tool_id: &str) -> &'static str {
    match tool_id {
        "claude-code" => "Claude Code",
        "codex" => "Codex",
        "gemini-cli" => "Gemini CLI",
        _ => "Unknown",
    }
}

/// 解析菜单项 ID，提取工具 ID 和 Profile 名称
///
/// 格式: `profile:{tool_id}:{profile_name}`
fn parse_profile_menu_id(id: &str) -> Option<(&str, &str)> {
    if !id.starts_with(PROFILE_MENU_PREFIX) {
        return None;
    }
    let rest = &id[PROFILE_MENU_PREFIX.len()..];
    let (tool_id, profile_name) = rest.split_once(':')?;
    Some((tool_id, profile_name))
}

/// 构建单个工具的 Profile 子菜单
fn build_tool_profile_submenu<R: Runtime>(
    app: &AppHandle<R>,
    tool_id: &str,
    profiles: &[String],
    active_profile: Option<&str>,
) -> tauri::Result<Submenu<R>> {
    let display_name = tool_display_name(tool_id);
    let mut builder = SubmenuBuilder::new(app, display_name);

    if profiles.is_empty() {
        let empty_item = MenuItem::with_id(
            app,
            format!("{}{}:empty", PROFILE_MENU_PREFIX, tool_id),
            "（无配置方案）",
            false,
            None::<&str>,
        )?;
        builder = builder.item(&empty_item);
    } else {
        let max_display = 10;
        for profile_name in profiles.iter().take(max_display) {
            let is_active = active_profile == Some(profile_name.as_str());
            let menu_id = format!("{}{}:{}", PROFILE_MENU_PREFIX, tool_id, profile_name);
            let display_text = if profile_name.len() > 30 {
                format!("{}...", &profile_name[..27])
            } else {
                profile_name.to_string()
            };
            let item = CheckMenuItem::with_id(
                app,
                &menu_id,
                &display_text,
                true,
                is_active,
                None::<&str>,
            )?;
            builder = builder.item(&item);
        }
        if profiles.len() > max_display {
            builder = builder.separator();
            let more_item = MenuItem::with_id(
                app,
                format!("{}{}:more", PROFILE_MENU_PREFIX, tool_id),
                format!("更多... (共 {} 个)", profiles.len()),
                true,
                None::<&str>,
            )?;
            builder = builder.item(&more_item);
        }
    }
    builder.build()
}

/// 创建菜单栏图标菜单（工具 Profile 平铺展示）
fn create_tray_menu<R: Runtime>(
    app: &AppHandle<R>,
    profile_manager: &ProfileManager,
) -> tauri::Result<Menu<R>> {
    let tools = ["claude-code", "codex", "gemini-cli"];
    let mut builder = MenuBuilder::new(app);

    builder = builder
        .item(&MenuItem::with_id(
            app,
            "menu:show",
            "显示主窗口",
            true,
            Some("CmdOrCtrl+M"),
        )?)
        .separator();

    for (i, tool_id) in tools.iter().enumerate() {
        let profiles = profile_manager.list_profiles(tool_id).unwrap_or_default();
        let active = profile_manager
            .get_active_profile_name(tool_id)
            .ok()
            .flatten();
        let submenu = build_tool_profile_submenu(app, tool_id, &profiles, active.as_deref())?;
        builder = builder.item(&submenu);
        if i < tools.len() - 1 {
            builder = builder.separator();
        }
    }

    builder = builder
        .separator()
        .item(&MenuItem::with_id(
            app,
            "menu:settings",
            "设置...",
            true,
            Some("CmdOrCtrl+,"),
        )?)
        .separator()
        .item(&PredefinedMenuItem::quit(app, Some("退出 DuckCoding"))?);

    builder.build()
}

/// 设置应用菜单栏（仅 macOS）
pub fn setup_app_menu(app: &tauri::App) -> tauri::Result<()> {
    let state = app.state::<ProfileManagerState>();
    let manager = state.manager.blocking_read();

    // 创建菜单栏图标（显示在右上角）
    let tray_menu = create_tray_menu(app.handle(), &manager)?;
    let app_handle = app.handle().clone();

    let _tray = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&tray_menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| {
            let id = event.id.as_ref();
            tracing::debug!(menu_id = %id, "菜单栏图标菜单事件");

            if let Some((tool_id, profile_name)) = parse_profile_menu_id(id) {
                if profile_name == "empty" || profile_name == "more" {
                    if profile_name == "more" {
                        let _ = app.emit("navigate-to", "/profile");
                    }
                    return;
                }
                handle_profile_activation(&app_handle, tool_id, profile_name);
                return;
            }

            match id {
                "menu:settings" => {
                    let _ = app.emit("navigate-to", "/settings");
                }
                "menu:show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}

/// 处理 Profile 激活
fn handle_profile_activation(app: &AppHandle, tool_id: &str, profile_name: &str) {
    let state = app.state::<ProfileManagerState>();
    let manager = state.manager.blocking_read();
    match manager.activate_profile(tool_id, profile_name) {
        Ok(()) => {
            tracing::info!(tool_id = %tool_id, profile = %profile_name, "从菜单激活 Profile");
            if let Err(e) = refresh_app_menu_internal(app) {
                tracing::error!(error = ?e, "刷新菜单失败");
            }
            let _ = app.emit(
                "profile-activated-from-menu",
                serde_json::json!({
                    "tool_id": tool_id,
                    "profile_name": profile_name,
                }),
            );
        }
        Err(e) => {
            tracing::error!(error = ?e, tool_id = %tool_id, profile = %profile_name, "激活 Profile 失败");
        }
    }
}

/// 刷新应用菜单栏（内部函数）
fn refresh_app_menu_internal(app: &AppHandle) -> tauri::Result<()> {
    let state = app.state::<ProfileManagerState>();
    let manager = state.manager.blocking_read();
    let menu = create_tray_menu(app, &manager)?;

    // 获取托盘图标并更新菜单
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(menu))?;
    }
    Ok(())
}

/// 刷新应用菜单栏（公开函数）
pub fn refresh_app_menu(app: &AppHandle) -> Result<(), String> {
    refresh_app_menu_internal(app).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_profile_menu_id() {
        assert_eq!(
            parse_profile_menu_id("profile:claude-code:my-profile"),
            Some(("claude-code", "my-profile"))
        );
        assert_eq!(
            parse_profile_menu_id("profile:codex:test:with:colons"),
            Some(("codex", "test:with:colons"))
        );
        assert_eq!(parse_profile_menu_id("other:id"), None);
        assert_eq!(parse_profile_menu_id("profile:"), None);
    }

    #[test]
    fn test_tool_display_name() {
        assert_eq!(tool_display_name("claude-code"), "Claude Code");
        assert_eq!(tool_display_name("codex"), "Codex");
        assert_eq!(tool_display_name("gemini-cli"), "Gemini CLI");
        assert_eq!(tool_display_name("unknown"), "Unknown");
    }
}
