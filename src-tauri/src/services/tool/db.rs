use crate::models::{SSHConfig, ToolInstance, ToolSource, ToolType};
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::PathBuf;

/// 数据库表定义
const CREATE_TOOL_INSTANCES_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS tool_instances (
    instance_id TEXT PRIMARY KEY,
    base_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    tool_type TEXT NOT NULL,
    tool_source TEXT NOT NULL,
    installed INTEGER NOT NULL DEFAULT 0,
    version TEXT,
    install_path TEXT,

    -- WSL配置字段
    wsl_distro TEXT,

    -- SSH配置字段
    ssh_display_name TEXT,
    ssh_host TEXT,
    ssh_port INTEGER,
    ssh_user TEXT,
    ssh_key_path TEXT,

    -- 元数据
    is_builtin INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_base_id ON tool_instances(base_id);
CREATE INDEX IF NOT EXISTS idx_tool_type ON tool_instances(tool_type);
CREATE INDEX IF NOT EXISTS idx_tool_source ON tool_instances(tool_source);
"#;

/// 工具实例数据库管理
pub struct ToolInstanceDB {
    db_path: PathBuf,
}

impl ToolInstanceDB {
    /// 创建新的数据库实例
    pub fn new() -> Result<Self> {
        let home_dir = dirs::home_dir().context("无法获取用户主目录")?;
        let duckcoding_dir = home_dir.join(".duckcoding");

        // 确保目录存在
        std::fs::create_dir_all(&duckcoding_dir).context("无法创建 .duckcoding 目录")?;

        let db_path = duckcoding_dir.join("tool_instances.db");

        Ok(Self { db_path })
    }

    /// 获取数据库连接
    fn get_connection(&self) -> Result<Connection> {
        Connection::open(&self.db_path)
            .with_context(|| format!("无法打开数据库: {:?}", self.db_path))
    }

    /// 初始化数据库表
    pub fn init_tables(&self) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute_batch(CREATE_TOOL_INSTANCES_TABLE)
            .context("初始化数据库表失败")?;

        // 执行数据库迁移
        self.migrate_schema(&conn)?;

        Ok(())
    }

    /// 数据库schema迁移
    fn migrate_schema(&self, conn: &Connection) -> Result<()> {
        // 检查并添加缺失的列
        let columns = self.get_table_columns(conn, "tool_instances")?;

        // 迁移: 添加 wsl_distro 列
        if !columns.contains(&"wsl_distro".to_string()) {
            tracing::info!("迁移数据库: 添加 wsl_distro 列");
            conn.execute("ALTER TABLE tool_instances ADD COLUMN wsl_distro TEXT", [])
                .context("添加 wsl_distro 列失败")?;
        }

        // 迁移: 添加 ssh_display_name 列
        if !columns.contains(&"ssh_display_name".to_string()) {
            tracing::info!("迁移数据库: 添加 ssh_display_name 列");
            conn.execute(
                "ALTER TABLE tool_instances ADD COLUMN ssh_display_name TEXT",
                [],
            )
            .context("添加 ssh_display_name 列失败")?;
        }

        // 迁移: 添加 ssh_host 列
        if !columns.contains(&"ssh_host".to_string()) {
            tracing::info!("迁移数据库: 添加 ssh_host 列");
            conn.execute("ALTER TABLE tool_instances ADD COLUMN ssh_host TEXT", [])
                .context("添加 ssh_host 列失败")?;
        }

        // 迁移: 添加 ssh_port 列
        if !columns.contains(&"ssh_port".to_string()) {
            tracing::info!("迁移数据库: 添加 ssh_port 列");
            conn.execute("ALTER TABLE tool_instances ADD COLUMN ssh_port INTEGER", [])
                .context("添加 ssh_port 列失败")?;
        }

        // 迁移: 添加 ssh_user 列
        if !columns.contains(&"ssh_user".to_string()) {
            tracing::info!("迁移数据库: 添加 ssh_user 列");
            conn.execute("ALTER TABLE tool_instances ADD COLUMN ssh_user TEXT", [])
                .context("添加 ssh_user 列失败")?;
        }

        // 迁移: 添加 ssh_key_path 列
        if !columns.contains(&"ssh_key_path".to_string()) {
            tracing::info!("迁移数据库: 添加 ssh_key_path 列");
            conn.execute(
                "ALTER TABLE tool_instances ADD COLUMN ssh_key_path TEXT",
                [],
            )
            .context("添加 ssh_key_path 列失败")?;
        }

        Ok(())
    }

    /// 获取表的所有列名
    fn get_table_columns(&self, conn: &Connection, table_name: &str) -> Result<Vec<String>> {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table_name))?;
        let columns = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()
            .context("获取表列信息失败")?;
        Ok(columns)
    }

    /// 获取所有工具实例
    pub fn get_all_instances(&self) -> Result<Vec<ToolInstance>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT instance_id, base_id, tool_name, tool_type, tool_source,
                    installed, version, install_path, wsl_distro,
                    ssh_display_name, ssh_host, ssh_port, ssh_user, ssh_key_path,
                    is_builtin, created_at, updated_at
             FROM tool_instances
             ORDER BY base_id, tool_type",
        )?;

        let instances = stmt.query_map([], |row| {
            let tool_type_str: String = row.get(3)?;
            let tool_source_str: String = row.get(4)?;
            let installed_int: i32 = row.get(5)?;
            let is_builtin_int: i32 = row.get(14)?;

            // 解析SSH配置
            let ssh_config = if tool_type_str == "SSH" {
                Some(SSHConfig {
                    display_name: row.get(9)?,
                    host: row.get(10)?,
                    port: row.get::<_, i32>(11)? as u16,
                    user: row.get(12)?,
                    key_path: row.get(13)?,
                })
            } else {
                None
            };

            Ok(ToolInstance {
                instance_id: row.get(0)?,
                base_id: row.get(1)?,
                tool_name: row.get(2)?,
                tool_type: ToolType::parse(&tool_type_str).unwrap_or(ToolType::Local),
                tool_source: ToolSource::parse(&tool_source_str).unwrap_or(ToolSource::External),
                installed: installed_int != 0,
                version: row.get(6)?,
                install_path: row.get(7)?,
                wsl_distro: row.get(8)?,
                ssh_config,
                is_builtin: is_builtin_int != 0,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        })?;

        instances
            .collect::<Result<Vec<_>, _>>()
            .context("解析工具实例数据失败")
    }

    /// 添加工具实例
    pub fn add_instance(&self, instance: &ToolInstance) -> Result<()> {
        let conn = self.get_connection()?;

        let (ssh_display_name, ssh_host, ssh_port, ssh_user, ssh_key_path) =
            if let Some(ref ssh) = instance.ssh_config {
                (
                    Some(ssh.display_name.clone()),
                    Some(ssh.host.clone()),
                    Some(ssh.port as i32),
                    Some(ssh.user.clone()),
                    ssh.key_path.clone(),
                )
            } else {
                (None, None, None, None, None)
            };

        conn.execute(
            "INSERT INTO tool_instances (
                instance_id, base_id, tool_name, tool_type, tool_source,
                installed, version, install_path, wsl_distro,
                ssh_display_name, ssh_host, ssh_port, ssh_user, ssh_key_path,
                is_builtin, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                instance.instance_id,
                instance.base_id,
                instance.tool_name,
                instance.tool_type.as_str(),
                instance.tool_source.as_str(),
                if instance.installed { 1 } else { 0 },
                instance.version,
                instance.install_path,
                instance.wsl_distro,
                ssh_display_name,
                ssh_host,
                ssh_port,
                ssh_user,
                ssh_key_path,
                if instance.is_builtin { 1 } else { 0 },
                instance.created_at,
                instance.updated_at,
            ],
        )
        .context("添加工具实例失败")?;

        Ok(())
    }

    /// 更新工具实例
    pub fn update_instance(&self, instance: &ToolInstance) -> Result<()> {
        let conn = self.get_connection()?;

        let (ssh_display_name, ssh_host, ssh_port, ssh_user, ssh_key_path) =
            if let Some(ref ssh) = instance.ssh_config {
                (
                    Some(ssh.display_name.clone()),
                    Some(ssh.host.clone()),
                    Some(ssh.port as i32),
                    Some(ssh.user.clone()),
                    ssh.key_path.clone(),
                )
            } else {
                (None, None, None, None, None)
            };

        conn.execute(
            "UPDATE tool_instances SET
                base_id = ?1, tool_name = ?2, tool_type = ?3, tool_source = ?4,
                installed = ?5, version = ?6, install_path = ?7,
                ssh_display_name = ?8, ssh_host = ?9, ssh_port = ?10,
                ssh_user = ?11, ssh_key_path = ?12,
                is_builtin = ?13, updated_at = ?14
             WHERE instance_id = ?15",
            params![
                instance.base_id,
                instance.tool_name,
                instance.tool_type.as_str(),
                instance.tool_source.as_str(),
                if instance.installed { 1 } else { 0 },
                instance.version,
                instance.install_path,
                ssh_display_name,
                ssh_host,
                ssh_port,
                ssh_user,
                ssh_key_path,
                if instance.is_builtin { 1 } else { 0 },
                instance.updated_at,
                instance.instance_id,
            ],
        )
        .context("更新工具实例失败")?;

        Ok(())
    }

    /// 删除工具实例
    pub fn delete_instance(&self, instance_id: &str) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "DELETE FROM tool_instances WHERE instance_id = ?1",
            params![instance_id],
        )
        .context("删除工具实例失败")?;
        Ok(())
    }

    /// 根据instance_id获取实例
    pub fn get_instance(&self, instance_id: &str) -> Result<Option<ToolInstance>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT instance_id, base_id, tool_name, tool_type, tool_source,
                    installed, version, install_path, wsl_distro,
                    ssh_display_name, ssh_host, ssh_port, ssh_user, ssh_key_path,
                    is_builtin, created_at, updated_at
             FROM tool_instances
             WHERE instance_id = ?1",
        )?;

        let mut instances = stmt.query_map([instance_id], |row| {
            let tool_type_str: String = row.get(3)?;
            let tool_source_str: String = row.get(4)?;
            let installed_int: i32 = row.get(5)?;
            let is_builtin_int: i32 = row.get(14)?;

            let ssh_config = if tool_type_str == "SSH" {
                Some(SSHConfig {
                    display_name: row.get(9)?,
                    host: row.get(10)?,
                    port: row.get::<_, i32>(11)? as u16,
                    user: row.get(12)?,
                    key_path: row.get(13)?,
                })
            } else {
                None
            };

            Ok(ToolInstance {
                instance_id: row.get(0)?,
                base_id: row.get(1)?,
                tool_name: row.get(2)?,
                tool_type: ToolType::parse(&tool_type_str).unwrap_or(ToolType::Local),
                tool_source: ToolSource::parse(&tool_source_str).unwrap_or(ToolSource::External),
                installed: installed_int != 0,
                version: row.get(6)?,
                install_path: row.get(7)?,
                wsl_distro: row.get(8)?,
                ssh_config,
                is_builtin: is_builtin_int != 0,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        })?;

        instances.next().transpose().context("查询工具实例失败")
    }

    /// 检查实例是否存在
    pub fn instance_exists(&self, instance_id: &str) -> Result<bool> {
        let conn = self.get_connection()?;
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM tool_instances WHERE instance_id = ?1",
            [instance_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// 检查是否有本地工具实例（用于判断是否需要执行首次检测）
    pub fn has_local_tools(&self) -> Result<bool> {
        let conn = self.get_connection()?;
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM tool_instances WHERE tool_type = 'Local'",
            [],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// 更新或插入实例（upsert）
    pub fn upsert_instance(&self, instance: &ToolInstance) -> Result<()> {
        let conn = self.get_connection()?;

        let (ssh_display_name, ssh_host, ssh_port, ssh_user, ssh_key_path) =
            if let Some(ref ssh) = instance.ssh_config {
                (
                    Some(ssh.display_name.clone()),
                    Some(ssh.host.clone()),
                    Some(ssh.port as i32),
                    Some(ssh.user.clone()),
                    ssh.key_path.clone(),
                )
            } else {
                (None, None, None, None, None)
            };

        // 先尝试更新，如果没有更新到任何行则插入
        let updated = conn.execute(
            "UPDATE tool_instances SET
                tool_source = ?1,
                installed = ?2,
                version = ?3,
                install_path = ?4,
                updated_at = ?5
             WHERE instance_id = ?6",
            params![
                instance.tool_source.as_str(),
                if instance.installed { 1 } else { 0 },
                instance.version,
                instance.install_path,
                instance.updated_at,
                instance.instance_id,
            ],
        )?;

        if updated == 0 {
            // 不存在，执行插入
            conn.execute(
                "INSERT INTO tool_instances (
                    instance_id, base_id, tool_name, tool_type, tool_source,
                    installed, version, install_path, wsl_distro,
                    ssh_display_name, ssh_host, ssh_port, ssh_user, ssh_key_path,
                    is_builtin, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                params![
                    instance.instance_id,
                    instance.base_id,
                    instance.tool_name,
                    instance.tool_type.as_str(),
                    instance.tool_source.as_str(),
                    if instance.installed { 1 } else { 0 },
                    instance.version,
                    instance.install_path,
                    instance.wsl_distro,
                    ssh_display_name,
                    ssh_host,
                    ssh_port,
                    ssh_user,
                    ssh_key_path,
                    if instance.is_builtin { 1 } else { 0 },
                    instance.created_at,
                    instance.updated_at,
                ],
            )?;
        }

        Ok(())
    }

    /// 获取本地工具实例
    pub fn get_local_instances(&self) -> Result<Vec<ToolInstance>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT instance_id, base_id, tool_name, tool_type, tool_source,
                    installed, version, install_path, wsl_distro,
                    ssh_display_name, ssh_host, ssh_port, ssh_user, ssh_key_path,
                    is_builtin, created_at, updated_at
             FROM tool_instances
             WHERE tool_type = 'Local'
             ORDER BY base_id",
        )?;

        let instances = stmt.query_map([], |row| {
            let tool_type_str: String = row.get(3)?;
            let tool_source_str: String = row.get(4)?;
            let installed_int: i32 = row.get(5)?;
            let is_builtin_int: i32 = row.get(14)?;

            Ok(ToolInstance {
                instance_id: row.get(0)?,
                base_id: row.get(1)?,
                tool_name: row.get(2)?,
                tool_type: ToolType::parse(&tool_type_str).unwrap_or(ToolType::Local),
                tool_source: ToolSource::parse(&tool_source_str).unwrap_or(ToolSource::External),
                installed: installed_int != 0,
                version: row.get(6)?,
                install_path: row.get(7)?,
                wsl_distro: row.get(8)?,
                ssh_config: None, // Local 类型不需要 SSH 配置
                is_builtin: is_builtin_int != 0,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        })?;

        instances
            .collect::<Result<Vec<_>, _>>()
            .context("解析本地工具实例数据失败")
    }
}

impl Default for ToolInstanceDB {
    fn default() -> Self {
        Self::new().expect("无法创建 ToolInstanceDB")
    }
}
