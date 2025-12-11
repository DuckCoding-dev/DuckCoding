use super::utils::parse_version_string;
use crate::commands::tool_management::ToolRegistryState;
use crate::commands::types::{ToolStatus, UpdateResult};
use ::duckcoding::models::{InstallMethod, Tool, ToolType};
use ::duckcoding::services::{tool::ToolInstanceDB, VersionService};
use ::duckcoding::utils::config::apply_proxy_if_configured;
use std::process::Command;
use tokio::time::{timeout, Duration};

/// 检查工具更新（不执行更新）
#[tauri::command]
pub async fn check_update(tool: String) -> Result<UpdateResult, String> {
    // 应用代理配置（如果已配置）
    apply_proxy_if_configured();

    #[cfg(debug_assertions)]
    tracing::debug!(tool = %tool, "检查更新（使用VersionService）");

    let tool_obj = Tool::by_id(&tool).ok_or_else(|| format!("未知工具: {tool}"))?;

    let version_service = VersionService::new();

    match version_service.check_version(&tool_obj).await {
        Ok(version_info) => Ok(UpdateResult {
            success: true,
            message: "检查完成".to_string(),
            has_update: version_info.has_update,
            current_version: version_info.installed_version,
            latest_version: version_info.latest_version,
            mirror_version: version_info.mirror_version,
            mirror_is_stale: Some(version_info.mirror_is_stale),
            tool_id: Some(tool.clone()),
        }),
        Err(e) => {
            // 降级：如果检查失败，返回无法检查但不报错
            Ok(UpdateResult {
                success: true,
                message: format!("无法检查更新: {e}"),
                has_update: false,
                current_version: None,
                latest_version: None,
                mirror_version: None,
                mirror_is_stale: None,
                tool_id: Some(tool.clone()),
            })
        }
    }
}

/// 检查工具更新（基于实例ID，使用配置的路径）
///
/// 工作流程：
/// 1. 从数据库获取实例信息
/// 2. 使用 install_path 执行 --version 获取当前版本
/// 3. 检查远程最新版本
///
/// 返回：更新信息
#[tauri::command]
pub async fn check_update_for_instance(
    instance_id: String,
    _registry_state: tauri::State<'_, ToolRegistryState>,
) -> Result<UpdateResult, String> {
    // 1. 从数据库获取实例信息
    let db = ToolInstanceDB::new().map_err(|e| format!("初始化数据库失败: {}", e))?;
    let all_instances = db
        .get_all_instances()
        .map_err(|e| format!("读取数据库失败: {}", e))?;

    let instance = all_instances
        .iter()
        .find(|inst| inst.instance_id == instance_id && inst.tool_type == ToolType::Local)
        .ok_or_else(|| format!("未找到实例: {}", instance_id))?;

    // 2. 使用 install_path 执行 --version 获取当前版本
    let current_version = if let Some(path) = &instance.install_path {
        let version_cmd = format!("{} --version", path);
        tracing::info!("实例 {} 版本更新命令: {:?}", instance_id, version_cmd);

        #[cfg(target_os = "windows")]
        let output = Command::new("cmd").arg("/C").arg(&version_cmd).output();

        #[cfg(not(target_os = "windows"))]
        let output = Command::new("sh").arg("-c").arg(&version_cmd).output();

        match output {
            Ok(out) if out.status.success() => {
                let raw_version = String::from_utf8_lossy(&out.stdout).trim().to_string();
                Some(parse_version_string(&raw_version))
            }
            Ok(_) => {
                return Err(format!("版本号获取错误：无法执行命令 {}", version_cmd));
            }
            Err(e) => {
                return Err(format!("版本号获取错误：执行失败 - {}", e));
            }
        }
    } else {
        // 没有路径，使用数据库中的版本
        instance.version.clone()
    };

    // 3. 检查远程最新版本
    let tool_id = &instance.base_id;
    let update_result = check_update(tool_id.clone()).await?;

    // 4. 如果当前版本有变化，更新数据库
    if current_version != instance.version {
        let mut updated_instance = instance.clone();
        updated_instance.version = current_version.clone();
        updated_instance.updated_at = chrono::Utc::now().timestamp();

        if let Err(e) = db.update_instance(&updated_instance) {
            tracing::warn!("更新实例 {} 版本失败: {}", instance_id, e);
        } else {
            tracing::info!(
                "实例 {} 版本已同步更新: {:?} -> {:?}",
                instance_id,
                instance.version,
                current_version
            );
        }
    }

    // 5. 返回结果，使用路径检测的版本号
    Ok(UpdateResult {
        success: update_result.success,
        message: update_result.message,
        has_update: update_result.has_update,
        current_version,
        latest_version: update_result.latest_version,
        mirror_version: update_result.mirror_version,
        mirror_is_stale: update_result.mirror_is_stale,
        tool_id: Some(tool_id.clone()),
    })
}

/// 刷新数据库中所有工具的版本号（使用配置的路径检测）
///
/// 工作流程：
/// 1. 读取数据库中所有本地工具实例
/// 2. 对每个有路径的实例，执行 --version 获取最新版本号
/// 3. 更新数据库中的版本号
///
/// 返回：更新后的工具状态列表
#[tauri::command]
pub async fn refresh_all_tool_versions(
    _registry_state: tauri::State<'_, ToolRegistryState>,
) -> Result<Vec<ToolStatus>, String> {
    let db = ToolInstanceDB::new().map_err(|e| format!("初始化数据库失败: {}", e))?;
    let all_instances = db
        .get_all_instances()
        .map_err(|e| format!("读取数据库失败: {}", e))?;

    let mut statuses = Vec::new();

    for instance in all_instances
        .iter()
        .filter(|i| i.tool_type == ToolType::Local)
    {
        // 使用 install_path 检测版本
        let new_version = if let Some(path) = &instance.install_path {
            let version_cmd = format!("{} --version", path);
            tracing::info!("工具 {} 版本检查: {:?}", instance.tool_name, version_cmd);

            #[cfg(target_os = "windows")]
            let output = Command::new("cmd").arg("/C").arg(&version_cmd).output();

            #[cfg(not(target_os = "windows"))]
            let output = Command::new("sh").arg("-c").arg(&version_cmd).output();

            match output {
                Ok(out) if out.status.success() => {
                    let raw_version = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    Some(parse_version_string(&raw_version))
                }
                _ => {
                    // 版本获取失败，保持原版本
                    tracing::warn!("工具 {} 版本检测失败1，保持原版本", instance.tool_name);
                    instance.version.clone()
                }
            }
        } else {
            tracing::warn!("工具 {} 版本检测失败2，保持原版本", instance.tool_name);
            instance.version.clone()
        };

        tracing::info!("工具 {} 新版本号: {:?}", instance.tool_name, new_version);

        // 如果版本号有变化，更新数据库
        if new_version != instance.version {
            let mut updated_instance = instance.clone();
            updated_instance.version = new_version.clone();
            updated_instance.updated_at = chrono::Utc::now().timestamp();

            if let Err(e) = db.update_instance(&updated_instance) {
                tracing::warn!("更新实例 {} 失败: {}", instance.instance_id, e);
            } else {
                tracing::info!(
                    "工具 {} 版本已更新: {:?} -> {:?}",
                    instance.tool_name,
                    instance.version,
                    new_version
                );
            }
        }

        // 添加到返回列表
        statuses.push(ToolStatus {
            id: instance.base_id.clone(),
            name: instance.tool_name.clone(),
            installed: instance.installed,
            version: new_version,
        });
    }

    Ok(statuses)
}

/// 批量检查所有工具更新
#[tauri::command]
pub async fn check_all_updates() -> Result<Vec<UpdateResult>, String> {
    // 应用代理配置（如果已配置）
    apply_proxy_if_configured();

    #[cfg(debug_assertions)]
    tracing::debug!("批量检查所有工具更新");

    let version_service = VersionService::new();
    let version_infos = version_service.check_all_tools().await;

    let results = version_infos
        .into_iter()
        .map(|info| UpdateResult {
            success: true,
            message: "检查完成".to_string(),
            has_update: info.has_update,
            current_version: info.installed_version,
            latest_version: info.latest_version,
            mirror_version: info.mirror_version,
            mirror_is_stale: Some(info.mirror_is_stale),
            tool_id: Some(info.tool_id),
        })
        .collect();

    Ok(results)
}

/// 更新工具实例（使用配置的安装器路径）
///
/// 工作流程：
/// 1. 从数据库读取实例信息
/// 2. 使用 installer_path 和 install_method 执行更新
/// 3. 更新数据库中的版本号
///
/// 返回：更新结果
#[tauri::command]
pub async fn update_tool_instance(
    instance_id: String,
    force: Option<bool>,
) -> Result<UpdateResult, String> {
    let force = force.unwrap_or(false);

    // 1. 从数据库读取实例信息
    let db = ToolInstanceDB::new().map_err(|e| format!("初始化数据库失败: {}", e))?;
    let all_instances = db
        .get_all_instances()
        .map_err(|e| format!("读取数据库失败: {}", e))?;

    let instance = all_instances
        .iter()
        .find(|inst| inst.instance_id == instance_id && inst.tool_type == ToolType::Local)
        .ok_or_else(|| format!("未找到实例: {}", instance_id))?;

    // 2. 检查是否有安装器路径和安装方法
    let installer_path = instance.installer_path.as_ref().ok_or_else(|| {
        "该实例未配置安装器路径，无法执行快捷更新。请手动更新或重新添加实例。".to_string()
    })?;

    let install_method = instance
        .install_method
        .as_ref()
        .ok_or_else(|| "该实例未配置安装方法，无法执行快捷更新".to_string())?;

    // 3. 根据安装方法构建更新命令
    let tool_obj = Tool::by_id(&instance.base_id).ok_or_else(|| "未知工具".to_string())?;

    let update_cmd = match install_method {
        InstallMethod::Npm => {
            let package_name = &tool_obj.npm_package;
            if force {
                format!("{} install -g {} --force", installer_path, package_name)
            } else {
                format!("{} update -g {}", installer_path, package_name)
            }
        }
        InstallMethod::Brew => {
            let tool_id = &instance.base_id;
            format!("{} upgrade {}", installer_path, tool_id)
        }
        InstallMethod::Official => {
            return Err("官方安装方式暂不支持快捷更新，请手动重新安装".to_string());
        }
        InstallMethod::Other => {
            return Err("「其他」类型不支持 APP 内快捷更新，请手动更新".to_string());
        }
    };

    // 4. 执行更新命令（120秒超时）
    tracing::info!("使用安装器 {} 执行更新: {}", installer_path, update_cmd);

    let update_future = async {
        #[cfg(target_os = "windows")]
        let output = Command::new("cmd").arg("/C").arg(&update_cmd).output();

        #[cfg(not(target_os = "windows"))]
        let output = Command::new("sh").arg("-c").arg(&update_cmd).output();

        output
    };

    let update_result = timeout(Duration::from_secs(120), update_future).await;

    match update_result {
        Ok(Ok(output)) if output.status.success() => {
            // 5. 更新成功，获取新版本
            let version_cmd = format!("{} --version", instance.install_path.as_ref().unwrap());

            #[cfg(target_os = "windows")]
            let version_output = Command::new("cmd").arg("/C").arg(&version_cmd).output();

            #[cfg(not(target_os = "windows"))]
            let version_output = Command::new("sh").arg("-c").arg(&version_cmd).output();

            let new_version = match version_output {
                Ok(out) if out.status.success() => {
                    let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    Some(parse_version_string(&raw))
                }
                _ => None,
            };

            // 6. 更新数据库中的版本号
            if let Some(ref version) = new_version {
                let mut updated_instance = instance.clone();
                updated_instance.version = Some(version.clone());
                updated_instance.updated_at = chrono::Utc::now().timestamp();

                if let Err(e) = db.update_instance(&updated_instance) {
                    tracing::warn!("更新数据库版本失败: {}", e);
                }
            }

            Ok(UpdateResult {
                success: true,
                message: "✅ 更新成功！".to_string(),
                has_update: false,
                current_version: new_version.clone(),
                latest_version: new_version,
                mirror_version: None,
                mirror_is_stale: None,
                tool_id: Some(instance.base_id.clone()),
            })
        }
        Ok(Ok(output)) => {
            // 命令执行失败
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            Err(format!(
                "更新失败\n\nstderr: {}\nstdout: {}",
                stderr, stdout
            ))
        }
        Ok(Err(e)) => Err(format!("执行命令失败: {}", e)),
        Err(_) => Err("更新超时（120秒）".to_string()),
    }
}
