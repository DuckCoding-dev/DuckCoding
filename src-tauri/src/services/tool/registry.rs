use crate::models::{InstallMethod, SSHConfig, Tool, ToolInstance, ToolType};
use crate::services::tool::{DetectorRegistry, ToolInstanceDB};
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
    detector_registry: DetectorRegistry,
    command_executor: CommandExecutor,
    wsl_executor: WSLExecutor,
}

impl ToolRegistry {
    /// 创建新的工具注册表
    pub async fn new() -> Result<Self> {
        let db = ToolInstanceDB::new()?;

        // 初始化配置文件（如果不存在）
        // 注意：迁移逻辑已移到 MigrationManager，这里仅初始化
        db.init_tables()?;

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            detector_registry: DetectorRegistry::new(),
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
        let detectors = self.detector_registry.all_detectors();
        tracing::info!("开始并行检测 {} 个本地工具", detectors.len());

        // 并行检测所有工具
        let futures: Vec<_> = detectors
            .iter()
            .map(|detector| self.detect_single_tool_by_detector(detector.clone()))
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

    /// 使用 Detector 检测单个工具（新方法）
    async fn detect_single_tool_by_detector(
        &self,
        detector: std::sync::Arc<dyn crate::services::tool::ToolDetector>,
    ) -> ToolInstance {
        let tool_id = detector.tool_id();
        let tool_name = detector.tool_name();
        tracing::debug!("检测工具: {}", tool_name);

        // 使用 Detector 进行检测
        let installed = detector.is_installed(&self.command_executor).await;

        let (version, install_path, install_method) = if installed {
            let version = detector.get_version(&self.command_executor).await;
            let path = detector.get_install_path(&self.command_executor).await;
            let method = detector.detect_install_method(&self.command_executor).await;
            (version, path, method)
        } else {
            (None, None, None)
        };

        tracing::debug!(
            "工具 {} 检测结果: installed={}, version={:?}, path={:?}, method={:?}",
            tool_name,
            installed,
            version,
            install_path,
            install_method
        );

        // 创建 ToolInstance（需要获取 Tool 的完整信息）
        let tool = Tool::by_id(tool_id).unwrap_or_else(|| {
            tracing::warn!("未找到工具定义: {}, 使用对应的静态方法", tool_id);
            match tool_id {
                "claude-code" => Tool::claude_code(),
                "codex" => Tool::codex(),
                "gemini-cli" => Tool::gemini_cli(),
                _ => panic!("未知工具ID: {}", tool_id),
            }
        });

        let instance_id = format!("{}-local", tool_id);
        let now = chrono::Utc::now().timestamp();

        ToolInstance {
            instance_id,
            base_id: tool.id.clone(),
            tool_name: tool.name.clone(),
            tool_type: ToolType::Local,
            install_method,
            installed,
            version,
            install_path,
            wsl_distro: None,
            ssh_config: None,
            is_builtin: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// 检测单个本地工具并持久化（公开方法）
    ///
    /// 工作流程：
    /// 1. 删除该工具的所有现有本地实例（避免重复）
    /// 2. 执行检测
    /// 3. 如果检测到，保存到数据库
    ///
    /// 返回：工具实例
    pub async fn detect_and_persist_single_tool(&self, tool_id: &str) -> Result<ToolInstance> {
        let detector = self
            .detector_registry
            .get(tool_id)
            .ok_or_else(|| anyhow::anyhow!("未找到工具 {} 的检测器", tool_id))?;

        tracing::info!("开始检测单个工具: {}", tool_id);

        // 1. 删除该工具的所有本地实例（避免重复）
        let db = self.db.lock().await;
        let all_instances = db.get_all_instances()?;
        for inst in all_instances {
            if inst.base_id == tool_id && inst.tool_type == ToolType::Local {
                tracing::info!("删除旧实例: {}", inst.instance_id);
                let _ = db.delete_instance(&inst.instance_id);
            }
        }
        drop(db);

        // 2. 执行检测
        let instance = self.detect_single_tool_by_detector(detector).await;

        // 3. 保存到数据库
        let db = self.db.lock().await;
        if instance.installed {
            db.upsert_instance(&instance)?;
            tracing::info!("工具 {} 检测并保存成功", instance.tool_name);
        } else {
            tracing::info!("工具 {} 未检测到", instance.tool_name);
        }
        drop(db);

        Ok(instance)
    }

    /// 刷新本地工具状态（重新检测，更新存在的，删除不存在的）
    pub async fn refresh_local_tools(&self) -> Result<Vec<ToolInstance>> {
        tracing::info!("刷新本地工具状态（重新检测）");

        let detectors = self.detector_registry.all_detectors();

        // 并行检测所有工具
        let futures: Vec<_> = detectors
            .iter()
            .map(|detector| self.detect_single_tool_by_detector(detector.clone()))
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
        // 重新检测本地工具并保存
        self.detect_and_persist_local_tools().await?;

        // 返回所有工具实例
        self.get_all_grouped().await
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

    /// 获取本地工具的轻量级状态（供 Dashboard 使用）
    /// 优先从数据库读取，如果数据库为空则执行检测并持久化
    pub async fn get_local_tool_status(&self) -> Result<Vec<crate::models::ToolStatus>> {
        tracing::debug!("获取本地工具轻量级状态");

        // 从数据库读取所有实例（不主动检测）
        let grouped = self.get_all_grouped().await?;

        // 转换为轻量级 ToolStatus
        let mut statuses = Vec::new();
        let detectors = self.detector_registry.all_detectors();

        for detector in detectors {
            let tool_id = detector.tool_id();
            let tool_name = detector.tool_name();

            if let Some(instances) = grouped.get(tool_id) {
                // 找到 Local 类型的实例
                if let Some(local_instance) = instances
                    .iter()
                    .find(|i| i.tool_type == crate::models::ToolType::Local)
                {
                    statuses.push(crate::models::ToolStatus {
                        id: tool_id.to_string(),
                        name: tool_name.to_string(),
                        installed: local_instance.installed,
                        version: local_instance.version.clone(),
                    });
                } else {
                    // 没有本地实例，返回未安装状态
                    statuses.push(crate::models::ToolStatus {
                        id: tool_id.to_string(),
                        name: tool_name.to_string(),
                        installed: false,
                        version: None,
                    });
                }
            } else {
                // 数据库中没有该工具的任何实例
                statuses.push(crate::models::ToolStatus {
                    id: tool_id.to_string(),
                    name: tool_name.to_string(),
                    installed: false,
                    version: None,
                });
            }
        }

        tracing::debug!("获取本地工具状态完成，共 {} 个工具", statuses.len());
        Ok(statuses)
    }

    /// 刷新本地工具状态并返回轻量级视图（供刷新按钮使用）
    /// 重新检测 → 更新数据库 → 返回 ToolStatus
    pub async fn refresh_and_get_local_status(&self) -> Result<Vec<crate::models::ToolStatus>> {
        tracing::info!("刷新本地工具状态（重新检测）");

        // 重新检测本地工具
        let instances = self.refresh_local_tools().await?;

        // 转换为轻量级状态
        let mut statuses = Vec::new();
        let detectors = self.detector_registry.all_detectors();

        for detector in detectors {
            let tool_id = detector.tool_id();
            let tool_name = detector.tool_name();

            if let Some(instance) = instances.iter().find(|i| i.base_id == tool_id) {
                statuses.push(crate::models::ToolStatus {
                    id: tool_id.to_string(),
                    name: tool_name.to_string(),
                    installed: instance.installed,
                    version: instance.version.clone(),
                });
            } else {
                statuses.push(crate::models::ToolStatus {
                    id: tool_id.to_string(),
                    name: tool_name.to_string(),
                    installed: false,
                    version: None,
                });
            }
        }

        tracing::info!("刷新完成，共 {} 个已安装工具", instances.len());
        Ok(statuses)
    }
}
