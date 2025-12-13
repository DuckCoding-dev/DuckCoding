//! 版本检查与更新模块
//!
//! 负责工具版本的检查、更新、刷新操作

use super::ToolRegistry;
use crate::models::{InstallMethod, Tool, ToolType, UpdateResult};
use crate::services::{tool::InstallerService, VersionService};
use crate::utils::parse_version_string;
use anyhow::Result;
use std::collections::HashMap;

impl ToolRegistry {
    /// 更新工具实例（使用配置的安装器）
    ///
    /// # 参数
    /// - instance_id: 实例ID
    /// - force: 是否强制更新
    ///
    /// # 返回
    /// - Ok(UpdateResult): 更新结果（包含新版本）
    /// - Err: 更新失败
    pub async fn update_instance(&self, instance_id: &str, force: bool) -> Result<UpdateResult> {
        // 1. 从数据库获取实例信息
        let db = self.db.lock().await;
        let all_instances = db.get_all_instances()?;
        drop(db);

        let instance = all_instances
            .iter()
            .find(|inst| inst.instance_id == instance_id && inst.tool_type == ToolType::Local)
            .ok_or_else(|| anyhow::anyhow!("未找到实例: {}", instance_id))?;

        // 2. 使用 InstallerService 执行更新
        let installer = InstallerService::new();
        let result = installer
            .update_instance_by_installer(instance, force)
            .await?;

        // 3. 如果更新成功，更新数据库中的版本号
        if result.success {
            if let Some(ref new_version) = result.current_version {
                let db = self.db.lock().await;
                let mut updated_instance = instance.clone();
                updated_instance.version = Some(new_version.clone());
                updated_instance.updated_at = chrono::Utc::now().timestamp();

                if let Err(e) = db.update_instance(&updated_instance) {
                    tracing::warn!("更新数据库版本失败: {}", e);
                }
            }
        }

        Ok(result)
    }

    /// 检查工具实例更新（使用配置的路径）
    ///
    /// # 参数
    /// - instance_id: 实例ID
    ///
    /// # 返回
    /// - Ok(UpdateResult): 更新信息（包含当前版本和最新版本）
    /// - Err: 检查失败
    pub async fn check_update_for_instance(&self, instance_id: &str) -> Result<UpdateResult> {
        // 1. 从数据库获取实例信息
        let db = self.db.lock().await;
        let all_instances = db.get_all_instances()?;
        drop(db);

        let instance = all_instances
            .iter()
            .find(|inst| inst.instance_id == instance_id && inst.tool_type == ToolType::Local)
            .ok_or_else(|| anyhow::anyhow!("未找到实例: {}", instance_id))?;

        // 2. 使用 install_path 执行 --version 获取当前版本
        let current_version = if let Some(path) = &instance.install_path {
            let version_cmd = format!("{} --version", path);
            tracing::info!("实例 {} 版本检查命令: {:?}", instance_id, version_cmd);

            let result = self.command_executor.execute_async(&version_cmd).await;

            if result.success {
                let raw_version = result.stdout.trim();
                Some(parse_version_string(raw_version))
            } else {
                anyhow::bail!("版本号获取错误：无法执行命令 {}", version_cmd);
            }
        } else {
            // 没有路径，使用数据库中的版本
            instance.version.clone()
        };

        // 3. 检查远程最新版本
        let tool_id = &instance.base_id;
        let version_service = VersionService::new();
        let version_info = version_service
            .check_version(
                &Tool::by_id(tool_id).ok_or_else(|| anyhow::anyhow!("未知工具: {}", tool_id))?,
            )
            .await;

        let update_result = match version_info {
            Ok(info) => UpdateResult {
                success: true,
                message: "检查完成".to_string(),
                has_update: info.has_update,
                current_version: current_version.clone(),
                latest_version: info.latest_version,
                mirror_version: info.mirror_version,
                mirror_is_stale: Some(info.mirror_is_stale),
                tool_id: Some(tool_id.clone()),
            },
            Err(e) => UpdateResult {
                success: true,
                message: format!("无法检查更新: {e}"),
                has_update: false,
                current_version: current_version.clone(),
                latest_version: None,
                mirror_version: None,
                mirror_is_stale: None,
                tool_id: Some(tool_id.clone()),
            },
        };

        // 4. 如果当前版本有变化，更新数据库
        if current_version != instance.version {
            let db = self.db.lock().await;
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

        Ok(update_result)
    }

    /// 刷新数据库中所有工具的版本号（使用配置的路径检测）
    ///
    /// # 返回
    /// - Ok(Vec<ToolStatus>): 更新后的工具状态列表
    /// - Err: 刷新失败
    pub async fn refresh_all_tool_versions(&self) -> Result<Vec<crate::models::ToolStatus>> {
        let db = self.db.lock().await;
        let all_instances = db.get_all_instances()?;
        drop(db);

        let mut statuses = Vec::new();

        for instance in all_instances
            .iter()
            .filter(|i| i.tool_type == ToolType::Local)
        {
            // 使用 install_path 检测版本
            let new_version = if let Some(path) = &instance.install_path {
                let version_cmd = format!("{} --version", path);
                tracing::info!("工具 {} 版本检查: {:?}", instance.tool_name, version_cmd);

                let result = self.command_executor.execute_async(&version_cmd).await;

                if result.success {
                    let raw_version = result.stdout.trim();
                    Some(parse_version_string(raw_version))
                } else {
                    // 版本获取失败，保持原版本
                    tracing::warn!("工具 {} 版本检测失败，保持原版本", instance.tool_name);
                    instance.version.clone()
                }
            } else {
                tracing::warn!("工具 {} 缺少安装路径，保持原版本", instance.tool_name);
                instance.version.clone()
            };

            tracing::info!("工具 {} 新版本号: {:?}", instance.tool_name, new_version);

            // 如果版本号有变化，更新数据库
            if new_version != instance.version {
                let db = self.db.lock().await;
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
            statuses.push(crate::models::ToolStatus {
                id: instance.base_id.clone(),
                name: instance.tool_name.clone(),
                installed: instance.installed,
                version: new_version,
            });
        }

        Ok(statuses)
    }

    /// 检测工具的安装方式（用于更新时选择正确的方法）
    pub async fn detect_install_methods(&self) -> Result<HashMap<String, InstallMethod>> {
        let mut methods = HashMap::new();

        let detectors = self.detector_registry.all_detectors();
        for detector in detectors {
            let tool_id = detector.tool_id();
            if let Some(method) = detector.detect_install_method(&self.command_executor).await {
                methods.insert(tool_id.to_string(), method);
            }
        }

        Ok(methods)
    }
}
