use crate::commands::tool_management::ToolRegistryState;
use crate::commands::types::{InstallResult, ToolStatus};
use ::duckcoding::models::{InstallMethod, Tool};
use ::duckcoding::services::InstallerService;
use ::duckcoding::utils::config::apply_proxy_if_configured;

/// 检查所有工具的安装状态（新架构：优先从数据库读取）
///
/// 工作流程：
/// 1. 检查数据库是否有数据
/// 2. 如果没有 → 执行首次检测并保存到数据库
/// 3. 从数据库读取并返回轻量级 ToolStatus
///
/// 性能：数据库读取 < 10ms，首次检测约 1.3s
#[tauri::command]
pub async fn check_installations(
    registry_state: tauri::State<'_, ToolRegistryState>,
) -> Result<Vec<ToolStatus>, String> {
    let registry = registry_state.registry.lock().await;
    registry
        .get_local_tool_status()
        .await
        .map_err(|e| format!("检查工具状态失败: {}", e))
}

/// 刷新工具状态（仅从数据库读取，不重新检测）
///
/// 修改说明：不再自动检测所有工具，仅返回数据库中已有的工具状态
/// 如需添加新工具或验证已有工具，请使用：
/// - 添加新工具：工具管理页面 → 添加实例
/// - 验证单个工具：使用 detect_single_tool 命令
#[tauri::command]
pub async fn refresh_tool_status(
    registry_state: tauri::State<'_, ToolRegistryState>,
) -> Result<Vec<ToolStatus>, String> {
    let registry = registry_state.registry.lock().await;
    registry
        .get_local_tool_status()
        .await
        .map_err(|e| format!("获取工具状态失败: {}", e))
}

/// 安装指定工具
#[tauri::command]
pub async fn install_tool(
    tool: String,
    method: String,
    force: Option<bool>,
) -> Result<InstallResult, String> {
    // 应用代理配置（如果已配置）
    apply_proxy_if_configured();

    let force = force.unwrap_or(false);
    #[cfg(debug_assertions)]
    tracing::debug!(tool = %tool, method = %method, force = force, "安装工具（使用InstallerService）");

    // 获取工具定义
    let tool_obj =
        Tool::by_id(&tool).ok_or_else(|| "❌ 未知的工具\n\n请联系开发者报告此问题".to_string())?;

    // 转换安装方法
    let install_method = match method.as_str() {
        "npm" => InstallMethod::Npm,
        "brew" => InstallMethod::Brew,
        "official" => InstallMethod::Official,
        _ => return Err(format!("❌ 未知的安装方法: {method}")),
    };

    // 使用 InstallerService 安装
    let installer = InstallerService::new();

    match installer.install(&tool_obj, &install_method, force).await {
        Ok(_) => {
            // 安装成功（前端会调用 refresh_tool_status 更新数据库）

            // 构造成功消息
            let message = match method.as_str() {
                "npm" => format!("✅ {} 安装成功！(通过 npm)", tool_obj.name),
                "brew" => format!("✅ {} 安装成功！(通过 Homebrew)", tool_obj.name),
                "official" => format!("✅ {} 安装成功！", tool_obj.name),
                _ => format!("✅ {} 安装成功！", tool_obj.name),
            };

            Ok(InstallResult {
                success: true,
                message,
                output: String::new(),
            })
        }
        Err(e) => {
            // 安装失败，返回错误信息
            Err(e.to_string())
        }
    }
}
