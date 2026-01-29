// Tool Instance DB - 工具实例存储管理（JSON 版本）
//
// 从 SQLite 迁移到 JSON 文件，支持版本控制和多端同步

use crate::data::DataManager;
use crate::models::{ToolInstance, ToolType};
use crate::services::tool::tools_config::{
    LocalToolInstance, SSHToolInstance, ToolsConfig, WSLToolInstance,
};
use anyhow::{Context, Result};
use std::path::PathBuf;

/// 工具实例数据库管理（JSON 存储）
pub struct ToolInstanceDB {
    config_path: PathBuf,
    data_manager: DataManager,
}

impl ToolInstanceDB {
    /// 创建新的数据库实例
    pub fn new() -> Result<Self> {
        let home_dir = dirs::home_dir().context("无法获取用户主目录")?;
        let duckcoding_dir = home_dir.join(".duckcoding");

        // 确保目录存在
        std::fs::create_dir_all(&duckcoding_dir).context("无法创建 .duckcoding 目录")?;

        let config_path = duckcoding_dir.join("tools.json");
        let data_manager = DataManager::new();

        Ok(Self {
            config_path,
            data_manager,
        })
    }

    /// 初始化配置文件（如果不存在）
    pub fn init_tables(&self) -> Result<()> {
        if !self.config_path.exists() {
            tracing::info!("初始化 tools.json 配置文件");
            let default_config = ToolsConfig::default();
            self.save_config(&default_config)?;
        }
        Ok(())
    }

    /// 读取配置
    fn load_config(&self) -> Result<ToolsConfig> {
        let json_value = self
            .data_manager
            .json()
            .read(&self.config_path)
            .context("读取 tools.json 失败")?;

        // 将 JSON Value 转换为 ToolsConfig
        serde_json::from_value(json_value).context("解析 tools.json 失败")
    }

    /// 保存配置
    fn save_config(&self, config: &ToolsConfig) -> Result<()> {
        // 将 ToolsConfig 转换为 JSON Value
        let json_value = serde_json::to_value(config).context("序列化 ToolsConfig 失败")?;

        self.data_manager
            .json()
            .write(&self.config_path, &json_value)
            .context("保存 tools.json 失败")
    }

    /// 获取所有工具实例
    pub fn get_all_instances(&self) -> Result<Vec<ToolInstance>> {
        let config = self.load_config()?;
        Ok(config.to_instances())
    }

    /// 添加工具实例
    pub fn add_instance(&self, instance: &ToolInstance) -> Result<()> {
        let mut config = self.load_config()?;

        // 查找/创建对应的 ToolGroup（兼容旧 tools.json 缺少分组的情况）
        if !config.tools.iter().any(|g| g.id == instance.base_id) {
            tracing::warn!("tools.json 缺少工具分组 {}，将自动补齐", instance.base_id);
            config
                .tools
                .push(crate::services::tool::tools_config::ToolGroup {
                    id: instance.base_id.clone(),
                    name: instance.tool_name.clone(),
                    local_tools: vec![],
                    wsl_tools: vec![],
                    ssh_tools: vec![],
                });
        }

        let tool_group = config
            .tools
            .iter_mut()
            .find(|g| g.id == instance.base_id)
            .ok_or_else(|| anyhow::anyhow!("未找到工具分组: {}", instance.base_id))?;

        // 根据类型添加到对应列表
        match instance.tool_type {
            ToolType::Local => {
                tool_group.local_tools.push(LocalToolInstance {
                    instance_id: instance.instance_id.clone(),
                    installed: instance.installed,
                    version: instance.version.clone(),
                    install_path: instance.install_path.clone(),
                    installer_path: instance.installer_path.clone(), // 新增
                    install_method: instance.install_method.clone(),
                    is_builtin: instance.is_builtin,
                    created_at: instance.created_at,
                    updated_at: instance.updated_at,
                });
            }
            ToolType::WSL => {
                if let Some(ref distro_name) = instance.wsl_distro {
                    tool_group.wsl_tools.push(WSLToolInstance {
                        instance_id: instance.instance_id.clone(),
                        distro_name: distro_name.clone(),
                        installed: instance.installed,
                        version: instance.version.clone(),
                        install_path: instance.install_path.clone(),
                        install_method: instance.install_method.clone(),
                        is_builtin: instance.is_builtin,
                        created_at: instance.created_at,
                        updated_at: instance.updated_at,
                    });
                }
            }
            ToolType::SSH => {
                if let Some(ref ssh_config) = instance.ssh_config {
                    tool_group.ssh_tools.push(SSHToolInstance {
                        instance_id: instance.instance_id.clone(),
                        ssh_config: ssh_config.clone(),
                        installed: instance.installed,
                        version: instance.version.clone(),
                        install_path: instance.install_path.clone(),
                        install_method: instance.install_method.clone(),
                        is_builtin: instance.is_builtin,
                        created_at: instance.created_at,
                        updated_at: instance.updated_at,
                    });
                }
            }
        }

        config.updated_at = chrono::Utc::now().to_rfc3339();
        self.save_config(&config)?;
        Ok(())
    }

    /// 更新工具实例
    pub fn update_instance(&self, instance: &ToolInstance) -> Result<()> {
        let mut config = self.load_config()?;

        // 查找/创建对应的 ToolGroup（避免旧数据导致更新失败）
        if !config.tools.iter().any(|g| g.id == instance.base_id) {
            tracing::warn!("tools.json 缺少工具分组 {}，将自动补齐", instance.base_id);
            config
                .tools
                .push(crate::services::tool::tools_config::ToolGroup {
                    id: instance.base_id.clone(),
                    name: instance.tool_name.clone(),
                    local_tools: vec![],
                    wsl_tools: vec![],
                    ssh_tools: vec![],
                });
        }

        let tool_group = config
            .tools
            .iter_mut()
            .find(|g| g.id == instance.base_id)
            .ok_or_else(|| anyhow::anyhow!("未找到工具分组: {}", instance.base_id))?;

        // 根据类型更新
        let updated = match instance.tool_type {
            ToolType::Local => {
                if let Some(local) = tool_group
                    .local_tools
                    .iter_mut()
                    .find(|t| t.instance_id == instance.instance_id)
                {
                    local.installed = instance.installed;
                    local.version = instance.version.clone();
                    local.install_path = instance.install_path.clone();
                    local.install_method = instance.install_method.clone();
                    local.updated_at = instance.updated_at;
                    true
                } else {
                    false
                }
            }
            ToolType::WSL => {
                if let Some(wsl) = tool_group
                    .wsl_tools
                    .iter_mut()
                    .find(|t| t.instance_id == instance.instance_id)
                {
                    wsl.installed = instance.installed;
                    wsl.version = instance.version.clone();
                    wsl.install_path = instance.install_path.clone();
                    wsl.install_method = instance.install_method.clone();
                    wsl.updated_at = instance.updated_at;
                    true
                } else {
                    false
                }
            }
            ToolType::SSH => {
                if let Some(ssh) = tool_group
                    .ssh_tools
                    .iter_mut()
                    .find(|t| t.instance_id == instance.instance_id)
                {
                    ssh.installed = instance.installed;
                    ssh.version = instance.version.clone();
                    ssh.install_path = instance.install_path.clone();
                    ssh.install_method = instance.install_method.clone();
                    ssh.updated_at = instance.updated_at;
                    true
                } else {
                    false
                }
            }
        };

        if !updated {
            return Err(anyhow::anyhow!("实例不存在: {}", instance.instance_id));
        }

        config.updated_at = chrono::Utc::now().to_rfc3339();
        self.save_config(&config)?;
        Ok(())
    }

    /// 删除工具实例
    pub fn delete_instance(&self, instance_id: &str) -> Result<()> {
        let mut config = self.load_config()?;

        let mut deleted = false;

        for tool_group in &mut config.tools {
            // 尝试从各个列表中删除
            tool_group
                .local_tools
                .retain(|t| t.instance_id != instance_id);
            tool_group
                .wsl_tools
                .retain(|t| t.instance_id != instance_id);
            tool_group
                .ssh_tools
                .retain(|t| t.instance_id != instance_id);

            deleted = true;
        }

        if deleted {
            config.updated_at = chrono::Utc::now().to_rfc3339();
            self.save_config(&config)?;
        }

        Ok(())
    }

    /// 根据 instance_id 获取实例
    pub fn get_instance(&self, instance_id: &str) -> Result<Option<ToolInstance>> {
        let instances = self.get_all_instances()?;
        Ok(instances.into_iter().find(|i| i.instance_id == instance_id))
    }

    /// 检查实例是否存在
    pub fn instance_exists(&self, instance_id: &str) -> Result<bool> {
        Ok(self.get_instance(instance_id)?.is_some())
    }

    /// 检查是否有本地工具实例（用于判断是否需要执行首次检测）
    pub fn has_local_tools(&self) -> Result<bool> {
        let config = self.load_config()?;
        let has_tools = config
            .tools
            .iter()
            .any(|group| !group.local_tools.is_empty());
        Ok(has_tools)
    }

    /// 更新或插入实例（upsert）
    pub fn upsert_instance(&self, instance: &ToolInstance) -> Result<()> {
        if self.instance_exists(&instance.instance_id)? {
            self.update_instance(instance)
        } else {
            self.add_instance(instance)
        }
    }

    /// 获取本地工具实例
    pub fn get_local_instances(&self) -> Result<Vec<ToolInstance>> {
        let instances = self.get_all_instances()?;
        Ok(instances
            .into_iter()
            .filter(|i| i.tool_type == ToolType::Local)
            .collect())
    }

    /// 从 SQLite 迁移到 JSON（一次性迁移）
    pub fn migrate_from_sqlite(&self) -> Result<()> {
        use rusqlite::Connection;

        let home_dir = dirs::home_dir().context("无法获取用户主目录")?;
        let old_db_path = home_dir.join(".duckcoding").join("tool_instances.db");

        if !old_db_path.exists() {
            tracing::info!("SQLite 数据库不存在，跳过迁移");
            return Ok(());
        }

        tracing::info!("开始从 SQLite 迁移到 JSON");

        let conn = Connection::open(&old_db_path)?;

        // 读取所有实例数据
        let mut stmt = conn.prepare(
            "SELECT instance_id, base_id, tool_name, tool_type,
                    installed, version, install_path, wsl_distro,
                    ssh_display_name, ssh_host, ssh_port, ssh_user, ssh_key_path,
                    is_builtin, created_at, updated_at
             FROM tool_instances",
        )?;

        let instances = stmt.query_map([], |row| {
            let tool_type_str: String = row.get(3)?;
            let installed_int: i32 = row.get(4)?;
            let is_builtin_int: i32 = row.get(13)?;

            let ssh_config = if tool_type_str == "SSH" {
                Some(crate::models::SSHConfig {
                    display_name: row.get(8)?,
                    host: row.get(9)?,
                    port: row.get::<_, i32>(10)? as u16,
                    user: row.get(11)?,
                    key_path: row.get(12)?,
                })
            } else {
                None
            };

            Ok(ToolInstance {
                instance_id: row.get(0)?,
                base_id: row.get(1)?,
                tool_name: row.get(2)?,
                tool_type: ToolType::parse(&tool_type_str).unwrap_or(ToolType::Local),
                install_method: None, // 旧数据没有 install_method，需要重新检测
                installed: installed_int != 0,
                version: row.get(5)?,
                install_path: row.get(6)?,
                installer_path: None, // 旧数据没有，需要后续检测
                wsl_distro: row.get(7)?,
                ssh_config,
                is_builtin: is_builtin_int != 0,
                created_at: row.get(14)?,
                updated_at: row.get(15)?,
            })
        })?;

        let instances: Vec<ToolInstance> = instances.collect::<Result<_, _>>()?;

        tracing::info!("从 SQLite 读取到 {} 个实例", instances.len());

        // 转换为 ToolsConfig 并保存
        let config = ToolsConfig::from_instances(instances);
        self.save_config(&config)?;

        tracing::info!("迁移完成，已保存到 {}", self.config_path.display());

        // 备份旧数据库
        let backup_path = old_db_path.with_extension("db.backup");
        std::fs::rename(&old_db_path, &backup_path)?;
        tracing::info!("旧数据库已备份到 {}", backup_path.display());

        Ok(())
    }
}

impl Default for ToolInstanceDB {
    fn default() -> Self {
        Self::new().expect("无法创建 ToolInstanceDB")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::InstallMethod;

    #[test]
    fn test_db_creation() {
        let db = ToolInstanceDB::new();
        assert!(db.is_ok());
    }

    #[test]
    fn test_config_round_trip() {
        let db = ToolInstanceDB::new().unwrap();

        // 创建测试实例
        let instance = ToolInstance {
            instance_id: "test-tool-local".to_string(),
            base_id: "claude-code".to_string(),
            tool_name: "Claude Code".to_string(),
            tool_type: ToolType::Local,
            install_method: Some(InstallMethod::Npm),
            installed: true,
            version: Some("1.0.0".to_string()),
            install_path: Some("/usr/local/bin/test".to_string()),
            installer_path: Some("/usr/local/bin/npm".to_string()),
            wsl_distro: None,
            ssh_config: None,
            is_builtin: true,
            created_at: 1733299200,
            updated_at: 1733299200,
        };

        // 添加实例
        let add_result = db.add_instance(&instance);
        assert!(add_result.is_ok());

        // 读取实例
        let loaded = db.get_instance("test-tool-local");
        assert!(loaded.is_ok());
        let loaded_instance = loaded.unwrap();
        assert!(loaded_instance.is_some());
        assert_eq!(
            loaded_instance.unwrap().install_method,
            Some(InstallMethod::Npm)
        );

        // 清理
        let _ = db.delete_instance("test-tool-local");
    }
}
