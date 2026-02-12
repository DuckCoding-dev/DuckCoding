//! macOS 应用菜单栏模块
//!
//! 提供 Profile 快捷切换、透明代理控制和更新检查功能，仅在 macOS 下启用

use std::collections::HashMap;
use tauri::{
    menu::{
        CheckMenuItem, Menu, MenuBuilder, MenuItem, PredefinedMenuItem, Submenu, SubmenuBuilder,
    },
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, Runtime,
};

use crate::commands::profile_commands::ProfileManagerState;
use crate::commands::proxy_commands::{
    start_tool_proxy_internal, stop_tool_proxy_internal, update_proxy_from_profile_internal,
    ProxyManagerState,
};
use crate::commands::update_commands::{trigger_check_update_internal, UpdateServiceState};
use duckcoding::models::proxy_config::ToolProxyConfig;
use duckcoding::services::config::watcher::suppress_external_detection_for_tool;
use duckcoding::services::profile_manager::ProfileManager;
use duckcoding::services::proxy_config_manager::ProxyConfigManager;

/// Profile 菜单项 ID 前缀
const PROFILE_MENU_PREFIX: &str = "profile:";
/// 透明代理菜单项 ID 前缀
const PROXY_MENU_PREFIX: &str = "proxy:";
/// 菜单中最多展示的 Profile 数
const MAX_MENU_PROFILE_COUNT: usize = 10;
/// 支持在菜单栏展示的工具
const SUPPORTED_MENU_TOOLS: [&str; 3] = ["claude-code", "codex", "gemini-cli"];

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProxyMenuAction<'a> {
    Start(&'a str),
    Stop(&'a str),
    Open(&'a str),
    Config(&'a str, &'a str),
}

/// 工具显示名称
fn tool_display_name(tool_id: &str) -> &'static str {
    match tool_id {
        "claude-code" => "Claude Code",
        "codex" => "Codex",
        "gemini-cli" => "Gemini CLI",
        _ => "Unknown",
    }
}

fn is_supported_proxy_tool(tool_id: &str) -> bool {
    SUPPORTED_MENU_TOOLS.contains(&tool_id)
}

fn proxy_page_path(tool_id: &str) -> String {
    format!("/transparent-proxy/{tool_id}")
}

fn proxy_tool_menu_label(tool_id: &str, is_running: bool) -> String {
    if is_running {
        format!("{} · 运行中", tool_display_name(tool_id))
    } else {
        format!("{} · 已停止", tool_display_name(tool_id))
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

/// 解析透明代理菜单项 ID
///
/// 支持格式:
/// - `proxy:start:{tool_id}`
/// - `proxy:stop:{tool_id}`
/// - `proxy:open:{tool_id}`
/// - `proxy:config:{tool_id}:{profile_name}`
fn parse_proxy_menu_id(id: &str) -> Option<ProxyMenuAction<'_>> {
    if !id.starts_with(PROXY_MENU_PREFIX) {
        return None;
    }

    let rest = &id[PROXY_MENU_PREFIX.len()..];
    let (action, payload) = rest.split_once(':')?;

    match action {
        "start" => {
            if is_supported_proxy_tool(payload) {
                Some(ProxyMenuAction::Start(payload))
            } else {
                None
            }
        }
        "stop" => {
            if is_supported_proxy_tool(payload) {
                Some(ProxyMenuAction::Stop(payload))
            } else {
                None
            }
        }
        "open" => {
            if is_supported_proxy_tool(payload) {
                Some(ProxyMenuAction::Open(payload))
            } else {
                None
            }
        }
        "config" => {
            let (tool_id, profile_name) = payload.split_once(':')?;
            if is_supported_proxy_tool(tool_id) {
                Some(ProxyMenuAction::Config(tool_id, profile_name))
            } else {
                None
            }
        }
        _ => None,
    }
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
        for profile_name in profiles.iter().take(MAX_MENU_PROFILE_COUNT) {
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
        if profiles.len() > MAX_MENU_PROFILE_COUNT {
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

/// 构建单个工具的透明代理子菜单
fn build_proxy_tool_submenu<R: Runtime>(
    app: &AppHandle<R>,
    tool_id: &str,
    profiles: &[String],
    selected_profile: Option<&str>,
    is_running: bool,
) -> tauri::Result<Submenu<R>> {
    let mut builder = SubmenuBuilder::new(app, proxy_tool_menu_label(tool_id, is_running));

    let start_item = MenuItem::with_id(
        app,
        format!("{}start:{}", PROXY_MENU_PREFIX, tool_id),
        "启动代理",
        !is_running,
        None::<&str>,
    )?;
    let stop_item = MenuItem::with_id(
        app,
        format!("{}stop:{}", PROXY_MENU_PREFIX, tool_id),
        "停止代理",
        is_running,
        None::<&str>,
    )?;
    let open_item = MenuItem::with_id(
        app,
        format!("{}open:{}", PROXY_MENU_PREFIX, tool_id),
        "打开透明代理页面",
        true,
        None::<&str>,
    )?;
    builder = builder
        .item(&start_item)
        .item(&stop_item)
        .item(&open_item)
        .separator();

    let profile_header = MenuItem::with_id(
        app,
        format!("{}profile_title:{}", PROXY_MENU_PREFIX, tool_id),
        "选择代理配置",
        false,
        None::<&str>,
    )?;
    builder = builder.item(&profile_header);

    if profiles.is_empty() {
        let empty_item = MenuItem::with_id(
            app,
            format!("{}config:{}:empty", PROXY_MENU_PREFIX, tool_id),
            "（无可选配置）",
            false,
            None::<&str>,
        )?;
        builder = builder.item(&empty_item);
    } else {
        for profile_name in profiles.iter().take(MAX_MENU_PROFILE_COUNT) {
            let is_selected = selected_profile == Some(profile_name.as_str());
            let menu_id = format!("{}config:{}:{}", PROXY_MENU_PREFIX, tool_id, profile_name);
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
                is_selected,
                None::<&str>,
            )?;
            builder = builder.item(&item);
        }

        if profiles.len() > MAX_MENU_PROFILE_COUNT {
            builder = builder.separator();
            let more_item = MenuItem::with_id(
                app,
                format!("{}config:{}:more", PROXY_MENU_PREFIX, tool_id),
                format!("更多... (共 {} 个)", profiles.len()),
                true,
                None::<&str>,
            )?;
            builder = builder.item(&more_item);
        }
    }

    builder.build()
}

/// 创建菜单栏图标菜单
fn create_tray_menu<R: Runtime>(
    app: &AppHandle<R>,
    profile_manager: &ProfileManager,
    running_states: &HashMap<String, bool>,
) -> tauri::Result<Menu<R>> {
    let proxy_config_mgr = ProxyConfigManager::new().ok();
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

    for (i, tool_id) in SUPPORTED_MENU_TOOLS.iter().enumerate() {
        let profiles = profile_manager.list_profiles(tool_id).unwrap_or_default();
        let active = profile_manager
            .get_active_profile_name(tool_id)
            .ok()
            .flatten();
        let submenu = build_tool_profile_submenu(app, tool_id, &profiles, active.as_deref())?;
        builder = builder.item(&submenu);
        if i < SUPPORTED_MENU_TOOLS.len() - 1 {
            builder = builder.separator();
        }
    }

    let proxy_title_item =
        MenuItem::with_id(app, "menu:proxy_title", "透明代理", false, None::<&str>)?;
    let check_update_item =
        MenuItem::with_id(app, "menu:check_update", "检查更新", true, None::<&str>)?;

    builder = builder.separator().item(&proxy_title_item);

    for (idx, tool_id) in SUPPORTED_MENU_TOOLS.iter().enumerate() {
        let profiles = profile_manager.list_profiles(tool_id).unwrap_or_default();
        let selected_profile = proxy_config_mgr
            .as_ref()
            .and_then(|mgr| mgr.get_config(tool_id).ok().flatten())
            .and_then(|config| config.real_profile_name);
        let is_running = running_states.get(*tool_id).copied().unwrap_or(false);
        let submenu = build_proxy_tool_submenu(
            app,
            tool_id,
            &profiles,
            selected_profile.as_deref(),
            is_running,
        )?;
        builder = builder.item(&submenu);

        if idx < SUPPORTED_MENU_TOOLS.len() - 1 {
            builder = builder.separator();
        }
    }

    builder = builder
        .separator()
        .item(&check_update_item)
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

fn focus_and_navigate<R: Runtime>(app: &AppHandle<R>, path: &str) {
    super::focus_main_window(app);
    if let Err(err) = app.emit("navigate-to", path) {
        tracing::error!(error = ?err, path = %path, "菜单导航事件发送失败");
    }
}

async fn load_proxy_running_states<R: Runtime>(app: &AppHandle<R>) -> HashMap<String, bool> {
    let proxy_state = app.state::<ProxyManagerState>();
    let current_statuses = proxy_state.manager.get_all_status().await;

    SUPPORTED_MENU_TOOLS
        .iter()
        .map(|tool_id| {
            (
                (*tool_id).to_string(),
                current_statuses.get(*tool_id).copied().unwrap_or(false),
            )
        })
        .collect()
}

async fn build_tray_menu_for_app<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let profile_state = app.state::<ProfileManagerState>();
    let profile_manager = profile_state.manager.read().await;
    let running_states = load_proxy_running_states(app).await;
    create_tray_menu(app, &profile_manager, &running_states)
}

fn has_proxy_start_prerequisites(tool_id: &str) -> bool {
    if !is_supported_proxy_tool(tool_id) {
        return false;
    }

    let proxy_config_mgr = match ProxyConfigManager::new() {
        Ok(manager) => manager,
        Err(error) => {
            tracing::error!(error = ?error, "读取代理配置管理器失败");
            return false;
        }
    };

    let config = match proxy_config_mgr.get_config(tool_id) {
        Ok(Some(config)) => config,
        Ok(None) => return false,
        Err(error) => {
            tracing::error!(error = ?error, tool_id = %tool_id, "读取工具代理配置失败");
            return false;
        }
    };

    has_required_proxy_fields(&config)
}

fn has_required_proxy_fields(config: &ToolProxyConfig) -> bool {
    config.enabled
        && config.local_api_key.is_some()
        && config.real_api_key.is_some()
        && config.real_base_url.is_some()
        && config.real_profile_name.is_some()
}

fn should_navigate_to_proxy_page_for_start_error(error: &str) -> bool {
    error.contains("代理配置不存在")
        || error.contains("透明代理未启用")
        || error.contains("保护密钥未设置")
        || error.contains("真实 API Key 或 Base URL 未设置")
        || error.contains("内置 Profile 不存在")
}

fn ensure_proxy_enabled_for_auto_start(tool_id: &str) -> Result<(), String> {
    let proxy_config_mgr = ProxyConfigManager::new().map_err(|e| e.to_string())?;
    let mut config = proxy_config_mgr
        .get_config(tool_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("工具 {} 的代理配置不存在", tool_id))?;

    if config.enabled {
        return Ok(());
    }

    config.enabled = true;
    proxy_config_mgr
        .update_config(tool_id, config)
        .map_err(|e| e.to_string())?;

    tracing::info!(tool_id = %tool_id, "从菜单选择配置时自动开启透明代理开关");
    Ok(())
}

fn handle_proxy_menu_action<R: Runtime>(app: &AppHandle<R>, action: ProxyMenuAction<'_>) {
    match action {
        ProxyMenuAction::Open(tool_id) => {
            focus_and_navigate(app, &proxy_page_path(tool_id));
        }
        ProxyMenuAction::Config(tool_id, profile_name) => {
            if profile_name == "empty" {
                return;
            }
            if profile_name == "more" {
                focus_and_navigate(app, &proxy_page_path(tool_id));
                return;
            }

            let app_handle = app.clone();
            let tool_id_owned = tool_id.to_string();
            let profile_name_owned = profile_name.to_string();
            tauri::async_runtime::spawn(async move {
                suppress_external_detection_for_tool(
                    &tool_id_owned,
                    std::time::Duration::from_secs(3),
                );

                let proxy_state = app_handle.state::<ProxyManagerState>();
                let profile_state = app_handle.state::<ProfileManagerState>();
                if let Err(error) = update_proxy_from_profile_internal(
                    &tool_id_owned,
                    &profile_name_owned,
                    &proxy_state,
                    &profile_state,
                )
                .await
                {
                    tracing::error!(
                        error = %error,
                        tool_id = %tool_id_owned,
                        profile_name = %profile_name_owned,
                        "从菜单切换透明代理配置失败"
                    );
                } else {
                    tracing::info!(
                        tool_id = %tool_id_owned,
                        profile_name = %profile_name_owned,
                        "从菜单切换透明代理配置成功"
                    );

                    let is_running = proxy_state.manager.is_running(&tool_id_owned).await;
                    if !is_running {
                        if let Err(error) = ensure_proxy_enabled_for_auto_start(&tool_id_owned) {
                            tracing::error!(
                                error = %error,
                                tool_id = %tool_id_owned,
                                "从菜单自动开启透明代理开关失败"
                            );
                            if should_navigate_to_proxy_page_for_start_error(&error) {
                                focus_and_navigate(&app_handle, &proxy_page_path(&tool_id_owned));
                            }
                        } else {
                            match start_tool_proxy_internal(
                                &tool_id_owned,
                                &proxy_state,
                                &profile_state,
                            )
                            .await
                            {
                                Ok(message) => {
                                    tracing::info!(
                                        tool_id = %tool_id_owned,
                                        message = %message,
                                        "从菜单选择配置后自动启动透明代理成功"
                                    );
                                }
                                Err(error) => {
                                    tracing::error!(
                                        error = %error,
                                        tool_id = %tool_id_owned,
                                        "从菜单选择配置后自动启动透明代理失败"
                                    );
                                    if should_navigate_to_proxy_page_for_start_error(&error) {
                                        focus_and_navigate(
                                            &app_handle,
                                            &proxy_page_path(&tool_id_owned),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                if let Err(error) = refresh_app_menu_internal_async(&app_handle).await {
                    tracing::error!(error = ?error, "刷新菜单失败");
                }
            });
        }
        ProxyMenuAction::Start(tool_id) => {
            if !has_proxy_start_prerequisites(tool_id) {
                focus_and_navigate(app, &proxy_page_path(tool_id));
                return;
            }

            let app_handle = app.clone();
            let tool_id_owned = tool_id.to_string();
            tauri::async_runtime::spawn(async move {
                let proxy_state = app_handle.state::<ProxyManagerState>();
                let profile_state = app_handle.state::<ProfileManagerState>();
                match start_tool_proxy_internal(&tool_id_owned, &proxy_state, &profile_state).await
                {
                    Ok(message) => {
                        tracing::info!(tool_id = %tool_id_owned, message = %message, "从菜单启动透明代理成功");
                    }
                    Err(error) => {
                        tracing::error!(error = %error, tool_id = %tool_id_owned, "从菜单启动透明代理失败");
                        if should_navigate_to_proxy_page_for_start_error(&error) {
                            focus_and_navigate(&app_handle, &proxy_page_path(&tool_id_owned));
                        }
                    }
                }

                if let Err(error) = refresh_app_menu_internal_async(&app_handle).await {
                    tracing::error!(error = ?error, "刷新菜单失败");
                }
            });
        }
        ProxyMenuAction::Stop(tool_id) => {
            let app_handle = app.clone();
            let tool_id_owned = tool_id.to_string();
            tauri::async_runtime::spawn(async move {
                let proxy_state = app_handle.state::<ProxyManagerState>();
                let profile_state = app_handle.state::<ProfileManagerState>();
                match stop_tool_proxy_internal(&tool_id_owned, &proxy_state, &profile_state).await {
                    Ok(message) => {
                        tracing::info!(tool_id = %tool_id_owned, message = %message, "从菜单停止透明代理成功");
                    }
                    Err(error) => {
                        tracing::error!(error = %error, tool_id = %tool_id_owned, "从菜单停止透明代理失败");
                    }
                }

                if let Err(error) = refresh_app_menu_internal_async(&app_handle).await {
                    tracing::error!(error = ?error, "刷新菜单失败");
                }
            });
        }
    }
}

/// 设置应用菜单栏（仅 macOS）
pub fn setup_app_menu(app: &tauri::App) -> tauri::Result<()> {
    // 创建菜单栏图标（显示在右上角）
    let tray_menu = tauri::async_runtime::block_on(build_tray_menu_for_app(app.handle()))?;

    let _tray = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&tray_menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| {
            let id = event.id.as_ref();
            tracing::debug!(menu_id = %id, "菜单栏图标菜单事件");

            if let Some((tool_id, profile_name)) = parse_profile_menu_id(id) {
                if profile_name == "empty" {
                    return;
                }
                if profile_name == "more" {
                    focus_and_navigate(app, "/profile");
                    return;
                }

                handle_profile_activation(app, tool_id, profile_name);
                return;
            }

            if let Some(action) = parse_proxy_menu_id(id) {
                handle_proxy_menu_action(app, action);
                return;
            }

            match id {
                "menu:settings" => {
                    let _ = app.emit("navigate-to", "/settings");
                }
                "menu:check_update" => {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let update_state = app_handle.state::<UpdateServiceState>();
                        if let Err(error) =
                            trigger_check_update_internal(&app_handle, &update_state).await
                        {
                            tracing::error!(error = %error, "从菜单后台检查更新失败");
                        }
                    });
                }
                "menu:show" => {
                    super::focus_main_window(app);
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}

/// 处理 Profile 激活
fn handle_profile_activation<R: Runtime>(app: &AppHandle<R>, tool_id: &str, profile_name: &str) {
    suppress_external_detection_for_tool(tool_id, std::time::Duration::from_secs(3));

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
fn refresh_app_menu_internal<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    tauri::async_runtime::block_on(refresh_app_menu_internal_async(app))
}

async fn refresh_app_menu_internal_async<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let menu = build_tray_menu_for_app(app).await?;

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
    fn test_parse_proxy_menu_id() {
        assert_eq!(
            parse_proxy_menu_id("proxy:start:claude-code"),
            Some(ProxyMenuAction::Start("claude-code"))
        );
        assert_eq!(
            parse_proxy_menu_id("proxy:stop:codex"),
            Some(ProxyMenuAction::Stop("codex"))
        );
        assert_eq!(
            parse_proxy_menu_id("proxy:open:gemini-cli"),
            Some(ProxyMenuAction::Open("gemini-cli"))
        );
        assert_eq!(
            parse_proxy_menu_id("proxy:config:codex:test:with:colons"),
            Some(ProxyMenuAction::Config("codex", "test:with:colons"))
        );
        assert_eq!(parse_proxy_menu_id("proxy:start:amp-code"), None);
        assert_eq!(parse_proxy_menu_id("proxy:unknown:codex"), None);
        assert_eq!(parse_proxy_menu_id("other:config:codex:test"), None);
    }

    #[test]
    fn test_should_navigate_to_proxy_page_for_start_error() {
        assert!(should_navigate_to_proxy_page_for_start_error(
            "内置 Profile 不存在，请先保存代理配置: dc_proxy_codex"
        ));
        assert!(should_navigate_to_proxy_page_for_start_error(
            "真实 API Key 或 Base URL 未设置"
        ));
        assert!(!should_navigate_to_proxy_page_for_start_error(
            "codex 代理已在运行"
        ));
    }

    #[test]
    fn test_has_required_proxy_fields() {
        let mut config = ToolProxyConfig::new(8788);
        assert!(!has_required_proxy_fields(&config));

        config.enabled = true;
        config.local_api_key = Some("local-key".to_string());
        config.real_api_key = Some("real-key".to_string());
        config.real_base_url = Some("https://api.example.com".to_string());
        config.real_profile_name = Some("default".to_string());

        assert!(has_required_proxy_fields(&config));
    }

    #[test]
    fn test_tool_display_name() {
        assert_eq!(tool_display_name("claude-code"), "Claude Code");
        assert_eq!(tool_display_name("codex"), "Codex");
        assert_eq!(tool_display_name("gemini-cli"), "Gemini CLI");
        assert_eq!(tool_display_name("unknown"), "Unknown");
    }
}
