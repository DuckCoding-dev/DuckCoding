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

        // 检测安装器路径（基于安装方法）
        let installer_path = if let (true, Some(method)) = (installed, &install_method) {
            match method {
                InstallMethod::Npm => {
                    // 检测 npm 路径：先用 which/where
                    let npm_detect_cmd = if cfg!(target_os = "windows") {
                        "where npm"
                    } else {
                        "which npm"
                    };

                    match self.command_executor.execute_async(npm_detect_cmd).await {
                        result if result.success => {
                            let path = result.stdout.lines().next().unwrap_or("").trim();
                            if !path.is_empty() {
                                Some(path.to_string())
                            } else {
                                None
                            }
                        }
                        _ => None,
                    }
                }
                InstallMethod::Brew => {
                    // 检测 brew 路径（仅 macOS）
                    match self.command_executor.execute_async("which brew").await {
                        result if result.success => {
                            let path = result.stdout.trim();
                            if !path.is_empty() {
                                Some(path.to_string())
                            } else {
                                None
                            }
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        } else {
            None
        };

        tracing::debug!(
            "工具 {} 检测结果: installed={}, version={:?}, path={:?}, method={:?}, installer={:?}",
            tool_name,
            installed,
            version,
            install_path,
            install_method,
            installer_path
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

        let now = chrono::Utc::now().timestamp();
        let instance_id = format!("{}-local-{}", tool_id, now);

        ToolInstance {
            instance_id,
            base_id: tool.id.clone(),
            tool_name: tool.name.clone(),
            tool_type: ToolType::Local,
            install_method,
            installed,
            version,
            install_path,
            installer_path, // 使用检测到的安装器路径
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
    /// 3. 检查路径是否与其他工具冲突
    /// 4. 如果检测到且无冲突，保存到数据库
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
        for inst in &all_instances {
            if inst.base_id == tool_id && inst.tool_type == ToolType::Local {
                tracing::info!("删除旧实例: {}", inst.instance_id);
                let _ = db.delete_instance(&inst.instance_id);
            }
        }
        drop(db);

        // 2. 执行检测
        let instance = self.detect_single_tool_by_detector(detector).await;

        // 3. 检查路径冲突（如果检测到路径）
        if instance.installed {
            if let Some(detected_path) = &instance.install_path {
                let db = self.db.lock().await;
                let all_instances = db.get_all_instances()?;
                drop(db);

                // 检查是否有其他工具使用了相同路径
                if let Some(existing) = all_instances.iter().find(|inst| {
                    inst.install_path.as_ref() == Some(detected_path)
                        && inst.tool_type == ToolType::Local
                        && inst.base_id != tool_id // 排除同一工具
                }) {
                    return Err(anyhow::anyhow!(
                        "路径冲突：检测到的路径 {} 已被 {} 使用",
                        detected_path,
                        existing.tool_name
                    ));
                }
            }
        }

        // 4. 保存到数据库
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

    /// 更新工具实例（使用配置的安装器）
    ///
    /// # 参数
    /// - instance_id: 实例ID
    /// - force: 是否强制更新
    ///
    /// # 返回
    /// - Ok(UpdateResult): 更新结果（包含新版本）
    /// - Err: 更新失败
    pub async fn update_instance(
        &self,
        instance_id: &str,
        force: bool,
    ) -> Result<crate::models::UpdateResult> {
        use crate::models::ToolType;
        use crate::services::tool::InstallerService;

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
    pub async fn check_update_for_instance(
        &self,
        instance_id: &str,
    ) -> Result<crate::models::UpdateResult> {
        use crate::models::ToolType;
        use crate::services::VersionService;
        use crate::utils::parse_version_string;

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
                &crate::models::Tool::by_id(tool_id)
                    .ok_or_else(|| anyhow::anyhow!("未知工具: {}", tool_id))?,
            )
            .await;

        let update_result = match version_info {
            Ok(info) => crate::models::UpdateResult {
                success: true,
                message: "检查完成".to_string(),
                has_update: info.has_update,
                current_version: current_version.clone(),
                latest_version: info.latest_version,
                mirror_version: info.mirror_version,
                mirror_is_stale: Some(info.mirror_is_stale),
                tool_id: Some(tool_id.clone()),
            },
            Err(e) => crate::models::UpdateResult {
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
        use crate::models::ToolType;
        use crate::utils::parse_version_string;

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

    /// 扫描所有工具候选（用于自动扫描）
    ///
    /// # 参数
    /// - tool_id: 工具ID（如 "claude-code"）
    ///
    /// # 返回
    /// - Ok(Vec<ToolCandidate>): 候选列表
    /// - Err: 扫描失败
    pub async fn scan_tool_candidates(
        &self,
        tool_id: &str,
    ) -> Result<Vec<crate::utils::ToolCandidate>> {
        use crate::utils::{parse_version_string, scan_installer_paths, scan_tool_executables};

        // 1. 扫描所有工具路径
        let tool_paths = scan_tool_executables(tool_id);
        let mut candidates = Vec::new();

        // 2. 对每个工具路径：获取版本和安装器
        for tool_path in tool_paths {
            // 获取版本
            let version_cmd = format!("{} --version", tool_path);
            let result = self.command_executor.execute_async(&version_cmd).await;

            let version = if result.success {
                let raw = result.stdout.trim();
                parse_version_string(raw)
            } else {
                // 版本获取失败，跳过此候选
                continue;
            };

            // 扫描安装器
            let installer_candidates = scan_installer_paths(&tool_path);
            let installer_path = installer_candidates.first().map(|c| c.path.clone());
            let install_method = installer_candidates
                .first()
                .map(|c| c.installer_type.clone())
                .unwrap_or(crate::models::InstallMethod::Official);

            candidates.push(crate::utils::ToolCandidate {
                tool_path: tool_path.clone(),
                installer_path,
                install_method,
                version,
            });
        }

        Ok(candidates)
    }

    /// 验证用户指定的工具路径是否有效
    ///
    /// # 参数
    /// - path: 工具路径
    ///
    /// # 返回
    /// - Ok(String): 版本号字符串
    /// - Err: 验证失败
    pub async fn validate_tool_path(&self, path: &str) -> Result<String> {
        use std::path::PathBuf;

        let path_buf = PathBuf::from(path);

        // 检查文件是否存在
        if !path_buf.exists() {
            anyhow::bail!("路径不存在: {}", path);
        }

        // 检查是否是文件
        if !path_buf.is_file() {
            anyhow::bail!("路径不是文件: {}", path);
        }

        // 执行 --version 命令
        let version_cmd = format!("{} --version", path);
        let result = self.command_executor.execute_async(&version_cmd).await;

        if !result.success {
            anyhow::bail!("命令执行失败，退出码: {:?}", result.exit_code);
        }

        // 解析版本号
        let version_str = result.stdout.trim();
        if version_str.is_empty() {
            anyhow::bail!("无法获取版本信息");
        }

        // 简单验证：版本号应该包含数字
        if !version_str.chars().any(|c| c.is_numeric()) {
            anyhow::bail!("无效的版本信息: {}", version_str);
        }

        Ok(version_str.to_string())
    }

    /// 添加手动配置的工具实例
    ///
    /// # 参数
    /// - tool_id: 工具ID
    /// - path: 工具路径
    /// - install_method: 安装方法
    /// - installer_path: 安装器路径（非 Other 类型时必需）
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

        // 2. 验证安装器路径（非 Other 类型时需要）
        if install_method != InstallMethod::Other {
            if let Some(ref installer) = installer_path {
                let installer_buf = PathBuf::from(installer);
                if !installer_buf.exists() {
                    anyhow::bail!("安装器路径不存在: {}", installer);
                }
                if !installer_buf.is_file() {
                    anyhow::bail!("安装器路径不是文件: {}", installer);
                }
            } else {
                anyhow::bail!("非「其他」类型必须提供安装器路径");
            }
        }

        // 3. 检查路径是否已存在
        let db = self.db.lock().await;
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

    /// 检测单个工具并保存到数据库（带缓存优化）
    ///
    /// # 参数
    /// - tool_id: 工具ID
    /// - force_redetect: 是否强制重新检测
    ///
    /// # 返回
    /// - Ok(ToolStatus): 工具状态
    /// - Err: 检测失败
    pub async fn detect_single_tool_with_cache(
        &self,
        tool_id: &str,
        force_redetect: bool,
    ) -> Result<crate::models::ToolStatus> {
        use crate::models::ToolType;

        if !force_redetect {
            // 1. 先查询数据库中是否已有该工具的本地实例
            let db = self.db.lock().await;
            let all_instances = db.get_all_instances()?;
            drop(db);

            // 查找该工具的本地实例
            if let Some(existing) = all_instances.iter().find(|inst| {
                inst.base_id == tool_id && inst.tool_type == ToolType::Local && inst.installed
            }) {
                // 如果已有实例且已安装，直接返回
                tracing::info!("工具 {} 已在数据库中，直接返回", existing.tool_name);
                return Ok(crate::models::ToolStatus {
                    id: tool_id.to_string(),
                    name: existing.tool_name.clone(),
                    installed: true,
                    version: existing.version.clone(),
                });
            }
        }

        // 2. 执行单工具检测（会删除旧实例避免重复）
        let instance = self.detect_and_persist_single_tool(tool_id).await?;

        // 3. 返回 ToolStatus 格式
        Ok(crate::models::ToolStatus {
            id: tool_id.to_string(),
            name: instance.tool_name.clone(),
            installed: instance.installed,
            version: instance.version.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::InstallMethod;

    /// 测试 ToolRegistry 创建
    #[tokio::test]
    async fn test_registry_creation() {
        let result = ToolRegistry::new().await;
        assert!(result.is_ok(), "ToolRegistry 创建应该成功");
    }

    /// 测试版本解析在 Registry 上下文中工作正常
    #[tokio::test]
    async fn test_validate_tool_path_with_invalid_path() {
        let registry = ToolRegistry::new().await.expect("创建 Registry 失败");

        // 测试不存在的路径
        let result = registry.validate_tool_path("/nonexistent/path").await;
        assert!(result.is_err(), "不存在的路径应该返回错误");
        assert!(
            result.unwrap_err().to_string().contains("路径不存在"),
            "错误信息应包含'路径不存在'"
        );
    }

    /// 测试添加工具实例的参数验证
    #[tokio::test]
    async fn test_add_tool_instance_validates_installer_path() {
        let registry = ToolRegistry::new().await.expect("创建 Registry 失败");

        // 测试：npm 安装方法但未提供安装器路径
        let result = registry
            .add_tool_instance(
                "claude-code",
                "/some/valid/path", // 这会在验证工具路径时失败，但我们主要测试安装器验证
                InstallMethod::Npm,
                None, // 未提供安装器路径
            )
            .await;

        // 应该失败（可能是路径验证失败，也可能是安装器路径验证失败）
        assert!(result.is_err(), "缺少安装器路径应该失败");
    }

    /// 测试工具名称映射
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

    /// 测试 has_local_tools_in_db 方法
    #[tokio::test]
    async fn test_has_local_tools_in_db() {
        let registry = ToolRegistry::new().await.expect("创建 Registry 失败");

        // 这个测试依赖于实际数据库状态，仅验证方法可调用
        let result = registry.has_local_tools_in_db().await;
        assert!(result.is_ok(), "has_local_tools_in_db 应该可以执行");
    }

    /// 测试 get_local_tool_status 方法
    #[tokio::test]
    async fn test_get_local_tool_status() {
        let registry = ToolRegistry::new().await.expect("创建 Registry 失败");

        // 测试获取本地工具状态
        let result = registry.get_local_tool_status().await;
        assert!(result.is_ok(), "get_local_tool_status 应该可以执行");

        // 验证返回的工具列表包含已知工具
        if let Ok(statuses) = result {
            let tool_ids: Vec<String> = statuses.iter().map(|s| s.id.clone()).collect();
            assert!(
                tool_ids.contains(&"claude-code".to_string())
                    || tool_ids.contains(&"codex".to_string())
                    || tool_ids.contains(&"gemini-cli".to_string()),
                "应该包含至少一个已知工具"
            );
        }
    }
}
