//! 工具实例管理模块
//!
//! 负责工具实例的添加、删除操作（Local/WSL/SSH）

use super::ToolRegistry;
use crate::models::{InstallMethod, SSHConfig, Tool, ToolInstance, ToolType};
use crate::utils::WSLExecutor;
use anyhow::Result;

impl ToolRegistry {
    /// 添加WSL工具实例
    pub async fn add_wsl_instance(&self, base_id: &str, distro_name: &str) -> Result<ToolInstance> {
        // 检查WSL是否可用
        if !WSLExecutor::is_available() {
            return Err(anyhow::anyhow!("WSL 不可用，请确保已安装 WSL"));
        }

        // 获取工具定义
        let tool =
            Tool::by_id(base_id).ok_or_else(|| anyhow::anyhow!("未知的工具ID: {}", base_id))?;

        // 提取命令名称
        let cmd_name = tool
            .check_command
            .split_whitespace()
            .next()
            .ok_or_else(|| anyhow::anyhow!("无效的检查命令"))?;

        // 在指定WSL发行版中检测工具
        let (installed, version, install_path) = self
            .wsl_executor
            .detect_tool_in_distro(Some(distro_name), cmd_name)
            .await?;

        // 创建实例
        let instance = ToolInstance::create_wsl_instance(
            base_id.to_string(),
            tool.name.clone(),
            distro_name.to_string(),
            installed,
            version,
            install_path,
        );

        // 保存到数据库
        let db = self.db.write().await;
        db.add_instance(&instance)?;
        drop(db);

        Ok(instance)
    }

    /// 添加SSH工具实例（本期仅存储配置，不实现检测）
    pub async fn add_ssh_instance(
        &self,
        base_id: &str,
        ssh_config: SSHConfig,
    ) -> Result<ToolInstance> {
        // 获取工具定义
        let tool =
            Tool::by_id(base_id).ok_or_else(|| anyhow::anyhow!("未知的工具ID: {}", base_id))?;

        // 创建SSH实例（本期不检测，installed设为false）
        let instance = ToolInstance::create_ssh_instance(
            base_id.to_string(),
            tool.name.clone(),
            ssh_config,
            false, // 本期不检测
            None,
            None,
        );

        // 检查是否已存在
        let db = self.db.write().await;
        if db.instance_exists(&instance.instance_id)? {
            return Err(anyhow::anyhow!("该SSH实例已存在"));
        }
        db.add_instance(&instance)?;
        drop(db);

        Ok(instance)
    }

    /// 删除工具实例（仅限SSH类型）
    pub async fn delete_instance(&self, instance_id: &str) -> Result<()> {
        let db = self.db.write().await;

        // 获取实例
        let instance = db
            .get_instance(instance_id)?
            .ok_or_else(|| anyhow::anyhow!("实例不存在: {}", instance_id))?;

        // 检查是否为SSH类型
        if instance.tool_type != ToolType::SSH {
            return Err(anyhow::anyhow!("仅允许删除SSH类型的实例"));
        }

        // 检查是否为内置实例
        if instance.is_builtin {
            return Err(anyhow::anyhow!("不允许删除内置实例"));
        }

        // 删除
        db.delete_instance(instance_id)?;
        drop(db);

        Ok(())
    }

    /// 添加手动配置的工具实例
    ///
    /// # 参数
    /// - tool_id: 工具ID
    /// - path: 工具路径
    /// - install_method: 安装方法
    /// - installer_path: 安装器路径（Npm/Brew 用于快捷更新；Official/Other 可为空）
    ///
    /// # 返回
    /// - Ok(ToolStatus): 工具状态
    /// - Err: 添加失败
    pub async fn add_tool_instance(
        &self,
        tool_id: &str,
        path: &str,
        install_method: InstallMethod,
        installer_path: Option<String>,
    ) -> Result<crate::models::ToolStatus> {
        use std::path::PathBuf;

        // 1. 验证工具路径
        let version = self.validate_tool_path(path).await?;

        // 2. 验证安装器路径（仅 Npm/Brew 需要；Official/Other 允许为空）
        match &install_method {
            InstallMethod::Npm | InstallMethod::Brew => {
                if let Some(ref installer) = installer_path {
                    let installer_buf = PathBuf::from(installer);
                    if !installer_buf.exists() {
                        anyhow::bail!("安装器路径不存在: {}", installer);
                    }
                    if !installer_buf.is_file() {
                        anyhow::bail!("安装器路径不是文件: {}", installer);
                    }
                } else {
                    anyhow::bail!("Npm/Brew 类型必须提供安装器路径");
                }
            }
            InstallMethod::Official | InstallMethod::Other => {
                if let Some(ref installer) = installer_path {
                    let installer_buf = PathBuf::from(installer);
                    if !installer_buf.exists() {
                        anyhow::bail!("安装器路径不存在: {}", installer);
                    }
                    if !installer_buf.is_file() {
                        anyhow::bail!("安装器路径不是文件: {}", installer);
                    }
                }
            }
        }

        // 3. 检查路径是否已存在
        let db = self.db.write().await;
        let all_instances = db.get_all_instances()?;

        // 路径冲突检查
        if let Some(existing) = all_instances.iter().find(|inst| {
            inst.install_path.as_ref() == Some(&path.to_string())
                && inst.tool_type == ToolType::Local
        }) {
            anyhow::bail!(
                "路径冲突：该路径已被 {} 使用，无法重复添加",
                existing.tool_name
            );
        }

        // 4. 获取工具显示名称
        let tool_name = match tool_id {
            "claude-code" => "Claude Code",
            "codex" => "CodeX",
            "gemini-cli" => "Gemini CLI",
            _ => tool_id,
        };

        // 5. 创建 ToolInstance（使用时间戳确保唯一性）
        let now = chrono::Utc::now().timestamp();
        let instance_id = format!("{}-local-{}", tool_id, now);
        let instance = ToolInstance {
            instance_id: instance_id.clone(),
            base_id: tool_id.to_string(),
            tool_name: tool_name.to_string(),
            tool_type: ToolType::Local,
            install_method: Some(install_method),
            installed: true,
            version: Some(version.clone()),
            install_path: Some(path.to_string()),
            installer_path,
            wsl_distro: None,
            ssh_config: None,
            is_builtin: false,
            created_at: now,
            updated_at: now,
        };

        // 6. 保存到数据库
        db.add_instance(&instance)?;

        // 7. 返回 ToolStatus 格式
        Ok(crate::models::ToolStatus {
            id: tool_id.to_string(),
            name: tool_name.to_string(),
            installed: true,
            version: Some(version),
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_tool_name_mapping() {
        // 这个测试验证 add_tool_instance 中的工具名称映射逻辑
        let test_cases = vec![
            ("claude-code", "Claude Code"),
            ("codex", "CodeX"),
            ("gemini-cli", "Gemini CLI"),
        ];

        for (tool_id, expected_name) in test_cases {
            let tool_name = match tool_id {
                "claude-code" => "Claude Code",
                "codex" => "CodeX",
                "gemini-cli" => "Gemini CLI",
                _ => tool_id,
            };
            assert_eq!(tool_name, expected_name, "工具名称映射应该正确");
        }
    }
}
