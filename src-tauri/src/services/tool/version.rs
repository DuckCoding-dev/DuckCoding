use crate::models::Tool;
use crate::services::tool::DetectorRegistry;
use crate::utils::CommandExecutor;
use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use semver::Version;
use serde::{Deserialize, Serialize};

/// 版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub tool_id: String,
    pub installed_version: Option<String>,
    pub latest_version: Option<String>,
    pub mirror_version: Option<String>, // 镜像实际可安装的版本
    pub mirror_is_stale: bool,          // 镜像是否滞后（用于前端显示警告）
    pub has_update: bool,
    pub source: VersionSource,
}

/// 版本来源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VersionSource {
    Local,          // 本地命令检查
    Mirror,         // 镜像站 API
    MirrorFallback, // 镜像站不可用，回退到本地
}

/// 镜像站 API 响应
#[derive(Debug, Deserialize)]
struct MirrorApiResponse {
    tools: Vec<ToolVersionFromMirror>,
    #[allow(dead_code)]
    updated_at: Option<String>,
    #[allow(dead_code)]
    status: Option<String>,
    #[allow(dead_code)]
    check_duration_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct ToolVersionFromMirror {
    id: String,
    #[allow(dead_code)]
    name: Option<String>,
    latest_version: String,         // 官方最新版本（通常来自 npm）
    mirror_version: Option<String>, // 镜像实际可安装的版本
    is_stale: Option<bool>,         // 镜像是否滞后
    #[allow(dead_code)]
    release_date: Option<String>,
    #[allow(dead_code)]
    download_url: Option<String>,
    #[allow(dead_code)]
    release_notes_url: Option<String>,
    #[allow(dead_code)]
    source: Option<String>,
    #[allow(dead_code)]
    package_name: Option<String>,
    #[allow(dead_code)]
    repository: Option<String>,
    #[allow(dead_code)]
    updated_at: Option<String>,
}

/// 版本服务
pub struct VersionService {
    detector_registry: DetectorRegistry,
    command_executor: CommandExecutor,
    mirror_api_url: String,
    #[allow(dead_code)]
    use_local_fallback: bool, // 是否启用本地 fallback
}

impl VersionService {
    pub fn new() -> Self {
        // 检查是否启用本地 fallback（开发/测试模式）
        let use_local_fallback = std::env::var("DUCKCODING_USE_LOCAL_VERSIONS")
            .ok()
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(false);

        VersionService {
            detector_registry: DetectorRegistry::new(),
            command_executor: CommandExecutor::new(),
            mirror_api_url: "https://mirror.duckcoding.com/api/v1/tools".to_string(),
            use_local_fallback,
        }
    }

    pub fn with_mirror_url(mirror_url: String) -> Self {
        let use_local_fallback = std::env::var("DUCKCODING_USE_LOCAL_VERSIONS")
            .ok()
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(false);

        VersionService {
            detector_registry: DetectorRegistry::new(),
            command_executor: CommandExecutor::new(),
            mirror_api_url: mirror_url,
            use_local_fallback,
        }
    }

    /// 检查工具版本（新架构：使用 tool_id）
    pub async fn check_version(&self, tool: &Tool) -> Result<VersionInfo> {
        self.check_version_by_id(&tool.id).await
    }

    /// 检查工具版本（通过 tool_id）
    pub async fn check_version_by_id(&self, tool_id: &str) -> Result<VersionInfo> {
        // 获取 Detector
        let detector = self
            .detector_registry
            .get(tool_id)
            .ok_or_else(|| anyhow::anyhow!("未知的工具 ID: {}", tool_id))?;

        // 使用 Detector 获取已安装版本
        let installed_version = detector.get_version(&self.command_executor).await;

        // 1. 尝试从镜像站获取最新版本
        match self.get_latest_from_mirror(tool_id).await {
            Ok((latest_version, mirror_version, mirror_is_stale)) => {
                // 使用镜像版本判断是否有更新（因为这是实际能安装的版本）
                let version_to_compare = mirror_version.as_ref().unwrap_or(&latest_version);
                let has_update =
                    Self::compare_versions(installed_version.as_deref(), version_to_compare);

                return Ok(VersionInfo {
                    tool_id: tool_id.to_string(),
                    installed_version,
                    latest_version: Some(latest_version),
                    mirror_version,
                    mirror_is_stale, // 传递镜像滞后状态
                    has_update,
                    source: VersionSource::Mirror,
                });
            }
            Err(e) => {
                tracing::warn!(error = ?e, "镜像站 API 不可用");
            }
        }

        // 2. 回退：无法获取远程版本，仅返回本地版本
        Ok(VersionInfo {
            tool_id: tool_id.to_string(),
            installed_version: installed_version.clone(),
            latest_version: installed_version,
            mirror_version: None,
            mirror_is_stale: false,
            has_update: false,
            source: VersionSource::MirrorFallback,
        })
    }

    /// 从镜像站 API 获取最新版本
    async fn get_latest_from_mirror(
        &self,
        tool_id: &str,
    ) -> Result<(String, Option<String>, bool)> {
        // 统一通过带代理的 Client 进行请求
        let client = crate::http_client::build_client().map_err(|e| anyhow::anyhow!(e))?;
        let response = client
            .get(&self.mirror_api_url)
            .send()
            .await?
            .json::<MirrorApiResponse>()
            .await?;

        response
            .tools
            .iter()
            .find(|t| t.id == tool_id)
            .map(|t| {
                let mirror_is_stale = t.is_stale.unwrap_or(false);
                (
                    t.latest_version.clone(),
                    t.mirror_version.clone(),
                    mirror_is_stale,
                )
            })
            .ok_or_else(|| anyhow::anyhow!("工具 {tool_id} 不在镜像站 API 中"))
    }

    /// 比较版本号
    fn compare_versions(installed: Option<&str>, latest: &str) -> bool {
        let latest_semver = Self::parse_version(latest);

        match (installed, latest_semver) {
            (None, _) => false, // 未安装不算"有更新"
            (Some(installed_str), Some(latest_version)) => {
                if let Some(installed_version) = Self::parse_version(installed_str) {
                    installed_version < latest_version
                } else {
                    installed_str.trim() != latest.trim()
                }
            }
            (Some(installed_str), None) => installed_str.trim() != latest.trim(),
        }
    }

    /// 解析版本号为可比较的元组
    fn parse_version(version: &str) -> Option<Version> {
        static VERSION_REGEX: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"(\d+\.\d+\.\d+(?:-[0-9A-Za-z\.-]+)?)").expect("invalid version regex")
        });

        let trimmed = version.trim();
        let captures = VERSION_REGEX.captures(trimmed)?;
        let matched = captures.get(1)?.as_str();

        Version::parse(matched).ok()
    }

    /// 批量从镜像站获取所有工具版本（优化：一次请求）
    async fn get_all_from_mirror(&self) -> Result<MirrorApiResponse> {
        #[cfg(debug_assertions)]
        tracing::debug!(api_url = %self.mirror_api_url, "请求镜像站 API");

        // 统一通过带代理的 Client 进行请求
        let client = crate::http_client::build_client().map_err(|e| anyhow::anyhow!(e))?;
        let response = client.get(&self.mirror_api_url).send().await?;

        #[cfg(debug_assertions)]
        tracing::debug!(status = %response.status(), "收到镜像站响应");

        let json_response = response.json::<MirrorApiResponse>().await?;

        #[cfg(debug_assertions)]
        tracing::debug!(tool_count = json_response.tools.len(), "成功解析 JSON");

        Ok(json_response)
    }

    /// 批量检查所有工具（优化：单次 API 请求）
    pub async fn check_all_tools(&self) -> Vec<VersionInfo> {
        let detectors = self.detector_registry.all_detectors();
        let mut results = Vec::new();

        #[cfg(debug_assertions)]
        tracing::debug!(tool_count = detectors.len(), "开始批量检查工具");

        // 1. 尝试一次性从镜像站获取所有工具版本
        match self.get_all_from_mirror().await {
            Ok(mirror_data) => {
                #[cfg(debug_assertions)]
                tracing::debug!("镜像站数据获取成功");

                // 成功获取镜像站数据，为每个工具构建 VersionInfo
                for detector in &detectors {
                    let tool_id = detector.tool_id();
                    let installed_version = detector.get_version(&self.command_executor).await;

                    // 从镜像站数据中查找该工具
                    if let Some(mirror_tool) = mirror_data.tools.iter().find(|t| t.id == tool_id) {
                        // 使用镜像版本判断是否有更新（这是实际能安装的版本）
                        let version_to_compare = mirror_tool
                            .mirror_version
                            .as_ref()
                            .unwrap_or(&mirror_tool.latest_version);

                        let has_update = Self::compare_versions(
                            installed_version.as_deref(),
                            version_to_compare,
                        );

                        let mirror_is_stale = mirror_tool.is_stale.unwrap_or(false);

                        #[cfg(debug_assertions)]
                        tracing::debug!(
                            tool_id = %tool_id,
                            installed_version = ?installed_version,
                            latest_version = %mirror_tool.latest_version,
                            mirror_version = ?mirror_tool.mirror_version,
                            mirror_is_stale = mirror_is_stale,
                            has_update = has_update,
                            "工具版本检查"
                        );

                        results.push(VersionInfo {
                            tool_id: tool_id.to_string(),
                            installed_version,
                            latest_version: Some(mirror_tool.latest_version.clone()),
                            mirror_version: mirror_tool.mirror_version.clone(),
                            mirror_is_stale, // 传递镜像滞后状态
                            has_update,
                            source: VersionSource::Mirror,
                        });
                    } else {
                        // 镜像站没有该工具数据，返回本地版本
                        results.push(VersionInfo {
                            tool_id: tool_id.to_string(),
                            installed_version: installed_version.clone(),
                            latest_version: installed_version,
                            mirror_version: None,
                            mirror_is_stale: false,
                            has_update: false,
                            source: VersionSource::MirrorFallback,
                        });
                    }
                }
            }
            Err(e) => {
                // 镜像站不可用，回退到仅本地版本（无法判断是否有更新）
                tracing::warn!(error = ?e, "镜像站 API 不可用，回退到本地检查");
                for detector in &detectors {
                    let tool_id = detector.tool_id();
                    let installed_version = detector.get_version(&self.command_executor).await;
                    results.push(VersionInfo {
                        tool_id: tool_id.to_string(),
                        installed_version: installed_version.clone(),
                        latest_version: installed_version,
                        mirror_version: None,
                        mirror_is_stale: false,
                        has_update: false,
                        source: VersionSource::MirrorFallback,
                    });
                }
            }
        }

        #[cfg(debug_assertions)]
        tracing::debug!(result_count = results.len(), "批量检查完成");

        results
    }
}

impl Default for VersionService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use semver::Version as SemverVersion;

    #[test]
    fn test_version_parsing() {
        assert_eq!(
            VersionService::parse_version("1.2.3").unwrap(),
            SemverVersion::new(1, 2, 3)
        );
        assert_eq!(
            VersionService::parse_version("v2.0.5").unwrap(),
            SemverVersion::new(2, 0, 5)
        );
        assert_eq!(
            VersionService::parse_version("1.2.3-beta").unwrap(),
            SemverVersion::parse("1.2.3-beta").unwrap()
        );
        assert_eq!(
            VersionService::parse_version("rust-v0.55.0").unwrap(),
            SemverVersion::parse("0.55.0").unwrap()
        );
        assert_eq!(
            VersionService::parse_version("0.13.0-preview.2").unwrap(),
            SemverVersion::parse("0.13.0-preview.2").unwrap()
        );
    }

    #[test]
    fn test_version_comparison() {
        assert!(VersionService::compare_versions(Some("1.0.0"), "1.0.1"));
        assert!(VersionService::compare_versions(Some("1.0.0"), "2.0.0"));
        assert!(VersionService::compare_versions(
            Some("0.12.0"),
            "0.13.0-preview.2"
        ));
        assert!(!VersionService::compare_versions(Some("2.0.0"), "1.0.0"));
        assert!(!VersionService::compare_versions(Some("1.0.0"), "1.0.0"));
        assert!(!VersionService::compare_versions(
            Some("0.55.0"),
            "rust-v0.55.0"
        ));
        assert!(!VersionService::compare_versions(None, "1.0.0"));
    }
}
