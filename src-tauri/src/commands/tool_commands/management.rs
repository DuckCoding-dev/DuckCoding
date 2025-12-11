use super::validation::validate_tool_path;
use crate::commands::tool_management::ToolRegistryState;
use crate::commands::types::ToolStatus;
use ::duckcoding::models::{InstallMethod, ToolInstance, ToolType};
use ::duckcoding::services::tool::ToolInstanceDB;
use std::path::PathBuf;

/// 手动添加工具实例（保存用户指定的路径）
///
/// 工作流程：
/// 1. 验证工具路径有效性
/// 2. 验证安装器路径有效性（非 Other 类型时）
/// 3. 检查路径是否已被其他工具使用（防止重复）
/// 4. 创建 ToolInstance
/// 5. 保存到数据库
///
/// 返回：工具状态信息
#[tauri::command]
pub async fn add_manual_tool_instance(
    tool_id: String,
    path: String,
    install_method: String, // "npm" | "brew" | "official" | "other"
    installer_path: Option<String>,
    _registry_state: tauri::State<'_, ToolRegistryState>,
) -> Result<ToolStatus, String> {
    // 1. 验证工具路径
    let version = validate_tool_path(tool_id.clone(), path.clone()).await?;

    // 2. 解析安装方法
    let parsed_method = match install_method.as_str() {
        "npm" => InstallMethod::Npm,
        "brew" => InstallMethod::Brew,
        "official" => InstallMethod::Official,
        "other" => InstallMethod::Other,
        _ => return Err(format!("未知的安装方法: {}", install_method)),
    };

    // 3. 验证安装器路径（非 Other 类型时需要）
    if parsed_method != InstallMethod::Other {
        if let Some(ref installer) = installer_path {
            let installer_buf = PathBuf::from(installer);
            if !installer_buf.exists() {
                return Err(format!("安装器路径不存在: {}", installer));
            }
            if !installer_buf.is_file() {
                return Err(format!("安装器路径不是文件: {}", installer));
            }
        } else {
            return Err("非「其他」类型必须提供安装器路径".to_string());
        }
    }

    // 4. 检查路径是否已存在
    let db = ToolInstanceDB::new().map_err(|e| format!("初始化数据库失败: {}", e))?;
    let all_instances = db
        .get_all_instances()
        .map_err(|e| format!("读取数据库失败: {}", e))?;

    // 路径冲突检查
    if let Some(existing) = all_instances
        .iter()
        .find(|inst| inst.install_path.as_ref() == Some(&path) && inst.tool_type == ToolType::Local)
    {
        return Err(format!(
            "路径冲突：该路径已被 {} 使用，无法重复添加",
            existing.tool_name
        ));
    }

    // 5. 创建工具显示名称
    let tool_name = match tool_id.as_str() {
        "claude-code" => "Claude Code",
        "codex" => "CodeX",
        "gemini-cli" => "Gemini CLI",
        _ => &tool_id,
    };

    // 6. 创建 ToolInstance（使用时间戳确保唯一性）
    let now = chrono::Utc::now().timestamp();
    let instance_id = format!("{}-local-{}", tool_id, now);
    let instance = ToolInstance {
        instance_id: instance_id.clone(),
        base_id: tool_id.clone(),
        tool_name: tool_name.to_string(),
        tool_type: ToolType::Local,
        install_method: Some(parsed_method),
        installed: true,
        version: Some(version.clone()),
        install_path: Some(path.clone()),
        installer_path, // 用户提供的安装器路径
        wsl_distro: None,
        ssh_config: None,
        is_builtin: false,
        created_at: now,
        updated_at: now,
    };

    // 7. 保存到数据库
    db.add_instance(&instance)
        .map_err(|e| format!("保存到数据库失败: {}", e))?;

    // 8. 返回 ToolStatus 格式
    Ok(ToolStatus {
        id: tool_id.clone(),
        name: tool_name.to_string(),
        installed: true,
        version: Some(version),
    })
}
