use crate::models::{SSHConfig, Tool, ToolInstance, ToolSource, ToolType};
use crate::services::tool::{ToolInstanceDB, ToolStatusCache};
use crate::utils::{CommandExecutor, WSLExecutor};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 工具检测进度（用于前端显示）
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolDetectionProgress {
    pub tool_id: String,
    pub tool_name: String,
    pub status: String, // "pending", "detecting", "done"
    pub installed: Option<bool>,
    pub version: Option<String>,
}

/// 工具注册表 - 统一管理所有工具实例
pub struct ToolRegistry {
    db: Arc<Mutex<ToolInstanceDB>>,
    cache: Arc<ToolStatusCache>,
    command_executor: CommandExecutor,
    wsl_executor: WSLExecutor,
}

impl ToolRegistry {
    /// 创建新的工具注册表
    pub async fn new() -> Result<Self> {
        let db = ToolInstanceDB::new()?;
        db.init_tables()?;

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            cache: Arc::new(ToolStatusCache::new()),
            command_executor: CommandExecutor::new(),
            wsl_executor: WSLExecutor::new(),
        })
    }

    /// 检查数据库中是否已有本地工具数据
    pub async fn has_local_tools_in_db(&self) -> Result<bool> {
        let db = self.db.lock().await;
        db.has_local_tools()
    }

    /// 获取所有工具实例（按工具ID分组）- 只从数据库读取
    pub async fn get_all_grouped(&self) -> Result<HashMap<String, Vec<ToolInstance>>> {
        tracing::debug!("开始从数据库获取所有工具实例");
        let mut grouped: HashMap<String, Vec<ToolInstance>> = HashMap::new();

        // 从数据库读取所有实例
        let db = self.db.lock().await;
        let db_instances = match db.get_all_instances() {
            Ok(instances) => {
                tracing::debug!("从数据库读取到 {} 个实例", instances.len());
                instances
            }
            Err(e) => {
                tracing::warn!("从数据库读取实例失败: {}, 使用空列表", e);
                Vec::new()
            }
        };
        drop(db);

        for instance in db_instances {
            grouped
                .entry(instance.base_id.clone())
                .or_default()
                .push(instance);
        }

        // 确保所有工具都有条目（即使没有实例）
        for tool_id in &["claude-code", "codex", "gemini-cli"] {
            grouped.entry(tool_id.to_string()).or_default();
        }

        tracing::debug!("完成获取所有工具实例，共 {} 个工具", grouped.len());
        Ok(grouped)
    }

    /// 检测本地工具并持久化到数据库（并行检测，用于新手引导）
    pub async fn detect_and_persist_local_tools(&self) -> Result<Vec<ToolInstance>> {
        let tools = Tool::all();
        tracing::info!("开始并行检测 {} 个本地工具", tools.len());

        // 并行检测所有工具
        let futures: Vec<_> = tools
            .iter()
            .map(|tool| self.detect_single_tool(tool.clone()))
            .collect();

        let results = futures_util::future::join_all(futures).await;

        // 收集结果并保存到数据库
        let mut instances = Vec::new();
        let db = self.db.lock().await;

        for instance in results {
            tracing::info!(
                "工具 {} 检测完成: installed={}, version={:?}",
                instance.tool_name,
                instance.installed,
                instance.version
            );
            // 使用 upsert 避免重复插入
            if let Err(e) = db.upsert_instance(&instance) {
                tracing::warn!("保存工具实例失败: {}", e);
            }
            instances.push(instance);
        }
        drop(db);

        tracing::info!("本地工具检测并持久化完成");
        Ok(instances)
    }

    /// 检测单个工具（内部使用，用于并行检测）
    async fn detect_single_tool(&self, tool: Tool) -> ToolInstance {
        tracing::debug!("检测工具: {}", tool.name);

        // 检测安装状态
        let installed = self
            .command_executor
            .command_exists_async(&tool.check_command)
            .await;

        // 如果已安装，获取版本和路径
        let (version, install_path) = if installed {
            let version = self.get_local_version(&tool).await;
            let path = self.get_local_install_path(&tool.check_command).await;
            (version, path)
        } else {
            (None, None)
        };

        tracing::debug!(
            "工具 {} 检测结果: installed={}, version={:?}, path={:?}",
            tool.name,
            installed,
            version,
            install_path
        );

        ToolInstance::from_tool_local(&tool, installed, version, install_path)
    }

    /// 刷新本地工具状态（重新检测，更新存在的，删除不存在的）
    pub async fn refresh_local_tools(&self) -> Result<Vec<ToolInstance>> {
        tracing::info!("刷新本地工具状态（重新检测）");
        self.cache.clear().await;

        let tools = Tool::all();

        // 并行检测所有工具
        let futures: Vec<_> = tools
            .iter()
            .map(|tool| self.detect_single_tool(tool.clone()))
            .collect();

        let results = futures_util::future::join_all(futures).await;

        // 获取数据库中现有的本地工具实例
        let db = self.db.lock().await;
        let existing_local = db.get_local_instances().unwrap_or_default();

        // 收集检测到的工具 ID
        let detected_ids: std::collections::HashSet<String> = results
            .iter()
            .filter(|r| r.installed)
            .map(|r| r.instance_id.clone())
            .collect();

        // 删除数据库中存在但本地已不存在的工具
        for existing in &existing_local {
            if !detected_ids.contains(&existing.instance_id) {
                tracing::info!("工具 {} 已不存在，从数据库删除", existing.tool_name);
                if let Err(e) = db.delete_instance(&existing.instance_id) {
                    tracing::warn!("删除工具实例失败: {}", e);
                }
            }
        }

        // 更新或插入检测到的工具
        let mut instances = Vec::new();
        for instance in results {
            if instance.installed {
                tracing::info!(
                    "工具 {} 检测完成: installed={}, version={:?}",
                    instance.tool_name,
                    instance.installed,
                    instance.version
                );
                if let Err(e) = db.upsert_instance(&instance) {
                    tracing::warn!("保存工具实例失败: {}", e);
                }
                instances.push(instance);
            }
        }
        drop(db);

        tracing::info!("本地工具刷新完成，共 {} 个已安装工具", instances.len());
        Ok(instances)
    }

    /// 获取本地工具版本
    async fn get_local_version(&self, tool: &Tool) -> Option<String> {
        let result = if tool.use_proxy_for_version_check {
            self.command_executor
                .execute_async(&tool.check_command)
                .await
        } else {
            self.execute_without_proxy(&tool.check_command).await
        };

        if result.success {
            self.extract_version(&result.stdout)
        } else {
            None
        }
    }

    /// 执行命令但不使用代理
    async fn execute_without_proxy(&self, command_str: &str) -> crate::utils::CommandResult {
        use crate::utils::platform::PlatformInfo;
        use std::process::Command;

        #[cfg(target_os = "windows")]
        use std::os::windows::process::CommandExt;

        let command_str = command_str.to_string();
        let platform = PlatformInfo::current();

        tokio::task::spawn_blocking(move || {
            let enhanced_path = platform.build_enhanced_path();

            let output = if platform.is_windows {
                #[cfg(target_os = "windows")]
                {
                    Command::new("cmd")
                        .args(["/C", &command_str])
                        .creation_flags(0x08000000)
                        .env("PATH", &enhanced_path)
                        .env_remove("HTTP_PROXY")
                        .env_remove("HTTPS_PROXY")
                        .env_remove("ALL_PROXY")
                        .env_remove("http_proxy")
                        .env_remove("https_proxy")
                        .env_remove("all_proxy")
                        .output()
                }
                #[cfg(not(target_os = "windows"))]
                {
                    Command::new("cmd")
                        .args(["/C", &command_str])
                        .env("PATH", &enhanced_path)
                        .env_remove("HTTP_PROXY")
                        .env_remove("HTTPS_PROXY")
                        .env_remove("ALL_PROXY")
                        .env_remove("http_proxy")
                        .env_remove("https_proxy")
                        .env_remove("all_proxy")
                        .output()
                }
            } else {
                Command::new("sh")
                    .args(["-c", &command_str])
                    .env("PATH", &enhanced_path)
                    .env_remove("HTTP_PROXY")
                    .env_remove("HTTPS_PROXY")
                    .env_remove("ALL_PROXY")
                    .env_remove("http_proxy")
                    .env_remove("https_proxy")
                    .env_remove("all_proxy")
                    .output()
            };

            match output {
                Ok(output) => crate::utils::CommandResult {
                    success: output.status.success(),
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    exit_code: output.status.code(),
                },
                Err(e) => crate::utils::CommandResult {
                    success: false,
                    stdout: String::new(),
                    stderr: e.to_string(),
                    exit_code: None,
                },
            }
        })
        .await
        .unwrap_or_else(|_| crate::utils::CommandResult {
            success: false,
            stdout: String::new(),
            stderr: "执行失败".to_string(),
            exit_code: None,
        })
    }

    /// 获取本地工具安装路径
    async fn get_local_install_path(&self, command: &str) -> Option<String> {
        let cmd_name = command.split_whitespace().next()?;

        #[cfg(target_os = "windows")]
        let which_cmd = format!("where {}", cmd_name);
        #[cfg(not(target_os = "windows"))]
        let which_cmd = format!("which {}", cmd_name);

        let result = self.command_executor.execute_async(&which_cmd).await;
        if result.success {
            let path = result.stdout.lines().next()?.trim();
            if !path.is_empty() {
                return Some(path.to_string());
            }
        }
        None
    }

    /// 从输出中提取版本号
    fn extract_version(&self, output: &str) -> Option<String> {
        let re = regex::Regex::new(r"v?(\d+\.\d+\.\d+(?:-[\w.]+)?)").ok()?;
        re.captures(output)?.get(1).map(|m| m.as_str().to_string())
    }

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
        let db = self.db.lock().await;
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
        let db = self.db.lock().await;
        if db.instance_exists(&instance.instance_id)? {
            return Err(anyhow::anyhow!("该SSH实例已存在"));
        }
        db.add_instance(&instance)?;
        drop(db);

        Ok(instance)
    }

    /// 删除工具实例（仅限SSH类型）
    pub async fn delete_instance(&self, instance_id: &str) -> Result<()> {
        let db = self.db.lock().await;

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

    /// 刷新所有工具实例（重新检测本地工具并更新数据库）
    pub async fn refresh_all(&self) -> Result<HashMap<String, Vec<ToolInstance>>> {
        // 清除缓存
        self.cache.clear().await;

        // 重新检测本地工具并保存
        self.detect_and_persist_local_tools().await?;

        // 返回所有工具实例
        self.get_all_grouped().await
    }

    /// 检测工具来源（用于前端显示）
    pub async fn detect_sources(&self) -> Result<HashMap<String, ToolSource>> {
        let mut sources = HashMap::new();

        let tools = Tool::all();
        for tool in tools {
            if let Some(path) = self.get_local_install_path(&tool.check_command).await {
                let source = ToolSource::from_install_path(&path);
                sources.insert(tool.id.clone(), source);
            }
        }

        Ok(sources)
    }
}
