//! 查询与辅助工具模块
//!
//! 负责工具状态查询、扫描、验证等辅助操作

use super::ToolRegistry;
use crate::models::{ToolInstance, ToolType};
use crate::utils::{
    parse_version_string, scan_installer_paths, scan_tool_executables, ToolCandidate,
};
use anyhow::Result;
use std::collections::HashMap;

impl ToolRegistry {
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

    /// 刷新所有工具实例（重新检测本地工具并更新数据库）
    pub async fn refresh_all(&self) -> Result<HashMap<String, Vec<ToolInstance>>> {
        // 重新检测本地工具并保存
        self.detect_and_persist_local_tools().await?;

        // 返回所有工具实例
        self.get_all_grouped().await
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
                if let Some(local_instance) =
                    instances.iter().find(|i| i.tool_type == ToolType::Local)
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

    /// 扫描所有工具候选（用于自动扫描）
    ///
    /// # 参数
    /// - tool_id: 工具ID（如 "claude-code"）
    ///
    /// # 返回
    /// - Ok(Vec<ToolCandidate>): 候选列表
    /// - Err: 扫描失败
    pub async fn scan_tool_candidates(&self, tool_id: &str) -> Result<Vec<ToolCandidate>> {
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

            candidates.push(ToolCandidate {
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
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[tokio::test]
    async fn test_has_local_tools_in_db() {
        let registry = ToolRegistry::new().await.expect("创建 Registry 失败");

        // 这个测试依赖于实际数据库状态，仅验证方法可调用
        let result = registry.has_local_tools_in_db().await;
        assert!(result.is_ok(), "has_local_tools_in_db 应该可以执行");
    }

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
