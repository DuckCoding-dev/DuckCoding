//! 工具检测模块
//!
//! 负责工具的自动检测、持久化和缓存管理

use super::ToolRegistry;
use crate::models::{InstallMethod, Tool, ToolInstance, ToolType};
use anyhow::Result;

impl ToolRegistry {
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
            tracing::warn!("未找到工具定义: {}, 使用静态方法", tool_id);
            match tool_id {
                "claude-code" => Tool::claude_code(),
                "codex" => Tool::codex(),
                "gemini-cli" => Tool::gemini_cli(),
                _ => {
                    // 返回一个默认 Tool，避免 panic
                    tracing::error!("未知工具ID: {}", tool_id);
                    Tool::claude_code() // 默认返回 claude-code
                }
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
