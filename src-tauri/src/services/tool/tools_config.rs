// Tools Config - tools.json 数据模型
//
// 用于版本控制和多端同步的工具配置文件

use crate::models::{InstallMethod, SSHConfig, ToolInstance, ToolType};
use serde::{Deserialize, Serialize};

/// tools.json 根配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfig {
    /// 配置文件版本
    pub version: String,
    /// 最后更新时间（ISO 8601）
    pub updated_at: String,
    /// 所有工具（按工具分组）
    pub tools: Vec<ToolGroup>,
}

/// 单个工具的配置（包含所有环境的实例）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolGroup {
    /// 工具 ID
    pub id: String,
    /// 工具名称
    pub name: String,
    /// 本地环境实例列表
    #[serde(default)]
    pub local_tools: Vec<LocalToolInstance>,
    /// WSL 环境实例列表
    #[serde(default)]
    pub wsl_tools: Vec<WSLToolInstance>,
    /// SSH 环境实例列表
    #[serde(default)]
    pub ssh_tools: Vec<SSHToolInstance>,
}

/// 本地工具实例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalToolInstance {
    pub instance_id: String,
    pub installed: bool,
    pub version: Option<String>,
    pub install_path: Option<String>,
    #[serde(default)]
    pub installer_path: Option<String>, // 安装器路径（如 npm/brew 路径）
    pub install_method: Option<InstallMethod>,
    pub is_builtin: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// WSL 工具实例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WSLToolInstance {
    pub instance_id: String,
    pub distro_name: String,
    pub installed: bool,
    pub version: Option<String>,
    pub install_path: Option<String>,
    pub install_method: Option<InstallMethod>,
    pub is_builtin: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// SSH 工具实例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSHToolInstance {
    pub instance_id: String,
    pub ssh_config: SSHConfig,
    pub installed: bool,
    pub version: Option<String>,
    pub install_path: Option<String>,
    pub install_method: Option<InstallMethod>,
    pub is_builtin: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Default for ToolsConfig {
    fn default() -> Self {
        ToolsConfig {
            version: "1.0.0".to_string(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            tools: vec![
                ToolGroup {
                    id: "claude-code".to_string(),
                    name: "Claude Code".to_string(),
                    local_tools: vec![],
                    wsl_tools: vec![],
                    ssh_tools: vec![],
                },
                ToolGroup {
                    id: "codex".to_string(),
                    name: "CodeX".to_string(),
                    local_tools: vec![],
                    wsl_tools: vec![],
                    ssh_tools: vec![],
                },
                ToolGroup {
                    id: "gemini-cli".to_string(),
                    name: "Gemini CLI".to_string(),
                    local_tools: vec![],
                    wsl_tools: vec![],
                    ssh_tools: vec![],
                },
            ],
        }
    }
}

impl ToolsConfig {
    /// 转换为扁平的 ToolInstance 列表
    pub fn to_instances(&self) -> Vec<ToolInstance> {
        let mut instances = Vec::new();

        for tool_group in &self.tools {
            // 转换本地实例
            for local in &tool_group.local_tools {
                instances.push(ToolInstance {
                    instance_id: local.instance_id.clone(),
                    base_id: tool_group.id.clone(),
                    tool_name: tool_group.name.clone(),
                    tool_type: ToolType::Local,
                    install_method: local.install_method.clone(),
                    installed: local.installed,
                    version: local.version.clone(),
                    install_path: local.install_path.clone(),
                    installer_path: local.installer_path.clone(), // 新增
                    wsl_distro: None,
                    ssh_config: None,
                    is_builtin: local.is_builtin,
                    created_at: local.created_at,
                    updated_at: local.updated_at,
                });
            }

            // 转换 WSL 实例
            for wsl in &tool_group.wsl_tools {
                instances.push(ToolInstance {
                    instance_id: wsl.instance_id.clone(),
                    base_id: tool_group.id.clone(),
                    tool_name: tool_group.name.clone(),
                    tool_type: ToolType::WSL,
                    install_method: wsl.install_method.clone(),
                    installed: wsl.installed,
                    version: wsl.version.clone(),
                    install_path: wsl.install_path.clone(),
                    installer_path: None, // WSL环境暂不支持
                    wsl_distro: Some(wsl.distro_name.clone()),
                    ssh_config: None,
                    is_builtin: wsl.is_builtin,
                    created_at: wsl.created_at,
                    updated_at: wsl.updated_at,
                });
            }

            // 转换 SSH 实例
            for ssh in &tool_group.ssh_tools {
                instances.push(ToolInstance {
                    instance_id: ssh.instance_id.clone(),
                    base_id: tool_group.id.clone(),
                    tool_name: tool_group.name.clone(),
                    tool_type: ToolType::SSH,
                    install_method: ssh.install_method.clone(),
                    installed: ssh.installed,
                    version: ssh.version.clone(),
                    install_path: ssh.install_path.clone(),
                    installer_path: None, // SSH环境暂不支持
                    wsl_distro: None,
                    ssh_config: Some(ssh.ssh_config.clone()),
                    is_builtin: ssh.is_builtin,
                    created_at: ssh.created_at,
                    updated_at: ssh.updated_at,
                });
            }
        }

        instances
    }

    /// 从 ToolInstance 列表创建配置
    pub fn from_instances(instances: Vec<ToolInstance>) -> Self {
        let mut config = ToolsConfig::default();

        // 按 base_id 分组
        let mut grouped: std::collections::HashMap<String, Vec<ToolInstance>> =
            std::collections::HashMap::new();

        for instance in instances {
            grouped
                .entry(instance.base_id.clone())
                .or_default()
                .push(instance);
        }

        // 转换为 ToolGroup
        config.tools.clear();
        for (base_id, instances) in grouped {
            let tool_name = instances
                .first()
                .map(|i| i.tool_name.clone())
                .unwrap_or_else(|| base_id.clone());

            let mut group = ToolGroup {
                id: base_id,
                name: tool_name,
                local_tools: vec![],
                wsl_tools: vec![],
                ssh_tools: vec![],
            };

            for instance in instances {
                match instance.tool_type {
                    ToolType::Local => {
                        group.local_tools.push(LocalToolInstance {
                            instance_id: instance.instance_id,
                            installed: instance.installed,
                            version: instance.version,
                            install_path: instance.install_path,
                            installer_path: instance.installer_path, // 新增
                            install_method: instance.install_method,
                            is_builtin: instance.is_builtin,
                            created_at: instance.created_at,
                            updated_at: instance.updated_at,
                        });
                    }
                    ToolType::WSL => {
                        if let Some(distro_name) = instance.wsl_distro {
                            group.wsl_tools.push(WSLToolInstance {
                                instance_id: instance.instance_id,
                                distro_name,
                                installed: instance.installed,
                                version: instance.version,
                                install_path: instance.install_path,
                                install_method: instance.install_method,
                                is_builtin: instance.is_builtin,
                                created_at: instance.created_at,
                                updated_at: instance.updated_at,
                            });
                        }
                    }
                    ToolType::SSH => {
                        if let Some(ssh_config) = instance.ssh_config {
                            group.ssh_tools.push(SSHToolInstance {
                                instance_id: instance.instance_id,
                                ssh_config,
                                installed: instance.installed,
                                version: instance.version,
                                install_path: instance.install_path,
                                install_method: instance.install_method,
                                is_builtin: instance.is_builtin,
                                created_at: instance.created_at,
                                updated_at: instance.updated_at,
                            });
                        }
                    }
                }
            }

            config.tools.push(group);
        }

        config.updated_at = chrono::Utc::now().to_rfc3339();
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ToolsConfig::default();
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.tools.len(), 3);
        assert_eq!(config.tools[0].id, "claude-code");
        assert_eq!(config.tools[1].id, "codex");
        assert_eq!(config.tools[2].id, "gemini-cli");
    }

    #[test]
    fn test_round_trip_conversion() {
        let mut config = ToolsConfig::default();

        // 添加一个本地实例
        config.tools[0].local_tools.push(LocalToolInstance {
            instance_id: "claude-code-local".to_string(),
            installed: true,
            version: Some("2.0.5".to_string()),
            install_path: Some("/usr/local/bin/claude".to_string()),
            installer_path: Some("/usr/local/bin/npm".to_string()),
            install_method: Some(InstallMethod::Npm),
            is_builtin: true,
            created_at: 1733299200,
            updated_at: 1733299200,
        });

        // 转换为 ToolInstance 列表
        let instances = config.to_instances();
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].instance_id, "claude-code-local");
        assert_eq!(instances[0].install_method, Some(InstallMethod::Npm));

        // 转换回 ToolsConfig
        let config2 = ToolsConfig::from_instances(instances);
        assert_eq!(config2.tools.len(), 1);
        assert_eq!(config2.tools[0].local_tools.len(), 1);
        assert_eq!(
            config2.tools[0].local_tools[0].install_method,
            Some(InstallMethod::Npm)
        );
    }
}
