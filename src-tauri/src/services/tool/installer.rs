use crate::models::{InstallMethod, Tool, ToolInstance, UpdateResult};
use crate::services::tool::DetectorRegistry;
use crate::utils::parse_version_string;
use anyhow::Result;
use tokio::time::{timeout, Duration};

/// 安装服务（新架构：委托给 Detector）
pub struct InstallerService {
    detector_registry: DetectorRegistry,
    command_executor: crate::utils::CommandExecutor,
}

impl InstallerService {
    pub fn new() -> Self {
        InstallerService {
            detector_registry: DetectorRegistry::new(),
            command_executor: crate::utils::CommandExecutor::new(),
        }
    }

    /// 安装工具（委托给 Detector）
    pub async fn install(&self, tool: &Tool, method: &InstallMethod, force: bool) -> Result<()> {
        let detector = self
            .detector_registry
            .get(&tool.id)
            .ok_or_else(|| anyhow::anyhow!("未知的工具 ID: {}", tool.id))?;

        tracing::info!("使用 Detector 安装工具: {}", tool.name);
        detector
            .install(&self.command_executor, method, force)
            .await
    }

    /// 更新工具（委托给 Detector）
    pub async fn update(&self, tool: &Tool, force: bool) -> Result<()> {
        let detector = self
            .detector_registry
            .get(&tool.id)
            .ok_or_else(|| anyhow::anyhow!("未知的工具 ID: {}", tool.id))?;

        tracing::info!("使用 Detector 更新工具: {}", tool.name);
        detector.update(&self.command_executor, force).await
    }

    /// 检查工具是否已安装（委托给 Detector）
    pub async fn is_installed(&self, tool: &Tool) -> bool {
        if let Some(detector) = self.detector_registry.get(&tool.id) {
            detector.is_installed(&self.command_executor).await
        } else {
            false
        }
    }

    /// 获取已安装版本（委托给 Detector）
    pub async fn get_installed_version(&self, tool: &Tool) -> Option<String> {
        if let Some(detector) = self.detector_registry.get(&tool.id) {
            detector.get_version(&self.command_executor).await
        } else {
            None
        }
    }

    /// 通过安装器更新工具实例
    ///
    /// 使用实例配置的安装器路径和安装方法执行更新
    ///
    /// # 参数
    /// - instance: 工具实例信息
    /// - force: 是否强制更新
    ///
    /// # 返回
    /// - Ok(UpdateResult): 更新结果（包含新版本信息）
    /// - Err: 更新失败的错误信息
    pub async fn update_instance_by_installer(
        &self,
        instance: &ToolInstance,
        force: bool,
    ) -> Result<UpdateResult> {
        // 1. 检查是否有安装方法
        let install_method = instance
            .install_method
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("该实例未配置安装方法，无法执行快捷更新"))?;

        // 2. 不支持的安装方式：直接返回（避免误报“缺少安装器路径”）
        match install_method {
            InstallMethod::Official => {
                anyhow::bail!("官方安装方式暂不支持快捷更新，请手动重新安装");
            }
            InstallMethod::Other => {
                anyhow::bail!("「其他」类型不支持 APP 内快捷更新，请手动更新");
            }
            InstallMethod::Npm | InstallMethod::Brew => {}
        }

        // 3. Npm/Brew：需要安装器路径
        let installer_path = instance.installer_path.as_ref().ok_or_else(|| {
            anyhow::anyhow!("该实例未配置安装器路径，无法执行快捷更新。请手动更新或重新添加实例。")
        })?;

        // 4. 根据安装方法构建更新命令
        let tool_obj = Tool::by_id(&instance.base_id).ok_or_else(|| anyhow::anyhow!("未知工具"))?;

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
            InstallMethod::Official | InstallMethod::Other => {
                unreachable!("InstallMethod::Official/Other 已在前置 match 中提前返回")
            }
        };

        // 3. 执行更新命令（120秒超时）
        tracing::info!("使用安装器 {} 执行更新: {}", installer_path, update_cmd);

        let update_future = {
            let executor = self.command_executor.clone();
            let cmd = update_cmd.clone();
            async move { executor.execute_async(&cmd).await }
        };

        let update_result = timeout(Duration::from_secs(120), update_future).await;

        match update_result {
            Ok(result) if result.success => {
                // 4. 更新成功，获取新版本
                let install_path = instance
                    .install_path
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("实例缺少安装路径"))?;

                let version_cmd = format!("{} --version", install_path);
                let version_result = self.command_executor.execute_async(&version_cmd).await;

                let new_version = if version_result.success {
                    let raw = version_result.stdout.trim();
                    Some(parse_version_string(raw))
                } else {
                    None
                };

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
            Ok(result) => {
                // 命令执行失败
                anyhow::bail!(
                    "更新失败\n\nstderr: {}\nstdout: {}",
                    result.stderr,
                    result.stdout
                );
            }
            Err(_) => {
                anyhow::bail!("更新超时（120秒）");
            }
        }
    }
}

impl Default for InstallerService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_creation() {
        let service = InstallerService::new();
        // 验证 detector_registry 已初始化
        assert!(service.detector_registry.contains("claude-code"));
        assert!(service.detector_registry.contains("codex"));
        assert!(service.detector_registry.contains("gemini-cli"));
    }

    /// 测试 update_instance_by_installer 方法参数验证
    #[tokio::test]
    async fn test_update_instance_by_installer_validates_installer_path() {
        use crate::models::{InstallMethod, ToolInstance, ToolType};

        let service = InstallerService::new();

        // 创建一个没有安装器路径的实例
        let instance = ToolInstance {
            instance_id: "test-instance".to_string(),
            base_id: "claude-code".to_string(),
            tool_name: "Claude Code".to_string(),
            tool_type: ToolType::Local,
            install_method: Some(InstallMethod::Npm),
            installed: true,
            version: Some("1.0.0".to_string()),
            install_path: Some("/usr/local/bin/claude".to_string()),
            installer_path: None, // 缺少安装器路径
            wsl_distro: None,
            ssh_config: None,
            is_builtin: false,
            created_at: 0,
            updated_at: 0,
        };

        // 测试：缺少安装器路径应该失败
        let result = service.update_instance_by_installer(&instance, false).await;
        assert!(result.is_err(), "缺少安装器路径应该失败");
        assert!(
            result.unwrap_err().to_string().contains("未配置安装器路径"),
            "错误信息应包含'未配置安装器路径'"
        );
    }

    /// 测试 update_instance_by_installer 拒绝 Official 和 Other 安装方法
    #[tokio::test]
    async fn test_update_instance_by_installer_rejects_unsupported_methods() {
        use crate::models::{InstallMethod, ToolInstance, ToolType};

        let service = InstallerService::new();

        // 测试 Official 方法
        let instance_official = ToolInstance {
            instance_id: "test-official".to_string(),
            base_id: "claude-code".to_string(),
            tool_name: "Claude Code".to_string(),
            tool_type: ToolType::Local,
            install_method: Some(InstallMethod::Official),
            installed: true,
            version: Some("1.0.0".to_string()),
            install_path: Some("/usr/local/bin/claude".to_string()),
            installer_path: Some("/usr/bin/install".to_string()),
            wsl_distro: None,
            ssh_config: None,
            is_builtin: false,
            created_at: 0,
            updated_at: 0,
        };

        let result = service
            .update_instance_by_installer(&instance_official, false)
            .await;
        assert!(result.is_err(), "Official 方法应该拒绝快捷更新");
        assert!(
            result.unwrap_err().to_string().contains("官方安装方式"),
            "错误信息应包含'官方安装方式'"
        );

        // 测试 Other 方法
        let instance_other = ToolInstance {
            instance_id: "test-other".to_string(),
            base_id: "claude-code".to_string(),
            tool_name: "Claude Code".to_string(),
            tool_type: ToolType::Local,
            install_method: Some(InstallMethod::Other),
            installed: true,
            version: Some("1.0.0".to_string()),
            install_path: Some("/usr/local/bin/claude".to_string()),
            installer_path: Some("/usr/bin/install".to_string()),
            wsl_distro: None,
            ssh_config: None,
            is_builtin: false,
            created_at: 0,
            updated_at: 0,
        };

        let result = service
            .update_instance_by_installer(&instance_other, false)
            .await;
        assert!(result.is_err(), "Other 方法应该拒绝快捷更新");
        assert!(
            result.unwrap_err().to_string().contains("其他"),
            "错误信息应包含'其他'"
        );
    }
}
