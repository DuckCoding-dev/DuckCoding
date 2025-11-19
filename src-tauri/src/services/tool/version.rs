use crate::models::Tool;
use crate::services::InstallerService;
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
    installer: InstallerService,
    mirror_api_url: String,
}

impl VersionService {
    pub fn new() -> Self {
        VersionService {
            installer: InstallerService::new(),
            mirror_api_url: "https://mirror.duckcoding.com/api/v1/tools".to_string(),
        }
    }

    pub fn with_mirror_url(mirror_url: String) -> Self {
        VersionService {
            installer: InstallerService::new(),
            mirror_api_url: mirror_url,
        }
    }

    /// 检查工具版本（优先使用镜像站 API）
    pub async fn check_version(&self, tool: &Tool) -> Result<VersionInfo> {
        let installed_version = self.installer.get_installed_version(tool).await;

        // 1. 尝试从镜像站获取最新版本
        match self.get_latest_from_mirror(&tool.id).await {
            Ok((latest_version, mirror_version, mirror_is_stale)) => {
                // 使用镜像版本判断是否有更新（因为这是实际能安装的版本）
                let version_to_compare = mirror_version.as_ref().unwrap_or(&latest_version);
                let has_update =
                    Self::compare_versions(installed_version.as_deref(), version_to_compare);

                return Ok(VersionInfo {
                    tool_id: tool.id.clone(),
                    installed_version,
                    latest_version: Some(latest_version),
                    mirror_version,
                    mirror_is_stale, // 传递镜像滞后状态
                    has_update,
                    source: VersionSource::Mirror,
                });
            }
            Err(e) => {
                eprintln!("⚠️  镜像站 API 不可用: {}", e);
            }
        }

        // 2. 回退到本地命令检查
        let latest_version = self.get_latest_from_local(tool).await?;
        let has_update = Self::compare_versions(installed_version.as_deref(), &latest_version);

        Ok(VersionInfo {
            tool_id: tool.id.clone(),
            installed_version,
            latest_version: Some(latest_version.clone()),
            mirror_version: None,   // 本地检查没有镜像版本信息
            mirror_is_stale: false, // 本地检查无法判断镜像状态
            has_update,
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
            .ok_or_else(|| anyhow::anyhow!("工具 {} 不在镜像站 API 中", tool_id))
    }

    /// 从本地命令获取最新版本（npm registry）
    async fn get_latest_from_local(&self, tool: &Tool) -> Result<String> {
        // 使用 npm view 获取最新版本
        let command = format!("npm view {} version", tool.npm_package);
        let result = self.installer.executor.execute_async(&command).await;

        if result.success {
            Ok(result.stdout.trim().to_string())
        } else {
            anyhow::bail!("无法获取最新版本: {}", result.stderr)
        }
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
        println!("🔍 正在请求镜像站 API: {}", &self.mirror_api_url);

        // 统一通过带代理的 Client 进行请求
        let client = crate::http_client::build_client().map_err(|e| anyhow::anyhow!(e))?;
        let response = client.get(&self.mirror_api_url).send().await?;

        #[cfg(debug_assertions)]
        println!("✅ 收到响应，状态码: {}", response.status());

        let json_response = response.json::<MirrorApiResponse>().await?;

        #[cfg(debug_assertions)]
        println!("✅ 成功解析 JSON，工具数量: {}", json_response.tools.len());

        Ok(json_response)
    }

    /// 批量检查所有工具（优化：单次 API 请求）
    pub async fn check_all_tools(&self) -> Vec<VersionInfo> {
        let tools = Tool::all();
        let mut results = Vec::new();

        #[cfg(debug_assertions)]
        println!("📦 开始批量检查 {} 个工具", tools.len());

        // 1. 尝试一次性从镜像站获取所有工具版本
        match self.get_all_from_mirror().await {
            Ok(mirror_data) => {
                #[cfg(debug_assertions)]
                println!("✅ 镜像站数据获取成功");

                // 成功获取镜像站数据，为每个工具构建 VersionInfo
                for tool in &tools {
                    let installed_version = self.installer.get_installed_version(tool).await;

                    // 从镜像站数据中查找该工具
                    if let Some(mirror_tool) = mirror_data.tools.iter().find(|t| t.id == tool.id) {
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
                        println!("  {} - 已安装: {:?}, 官方最新: {}, 镜像版本: {:?}, 镜像滞后: {}, 有更新: {}",
                            tool.id, installed_version, mirror_tool.latest_version,
                            mirror_tool.mirror_version, mirror_is_stale, has_update);

                        results.push(VersionInfo {
                            tool_id: tool.id.clone(),
                            installed_version,
                            latest_version: Some(mirror_tool.latest_version.clone()),
                            mirror_version: mirror_tool.mirror_version.clone(),
                            mirror_is_stale, // 传递镜像滞后状态
                            has_update,
                            source: VersionSource::Mirror,
                        });
                    } else {
                        // 镜像站没有该工具数据，回退到本地检查
                        if let Ok(info) = self.check_version_local(tool, installed_version).await {
                            results.push(info);
                        }
                    }
                }
            }
            Err(e) => {
                // 镜像站不可用，逐个回退到本地检查（跳过镜像重试）
                eprintln!("⚠️  镜像站 API 不可用，回退到本地检查: {}", e);
                for tool in &tools {
                    let installed_version = self.installer.get_installed_version(tool).await;
                    if let Ok(info) = self.check_version_local(tool, installed_version).await {
                        results.push(info);
                    }
                }
            }
        }

        #[cfg(debug_assertions)]
        println!("📊 批量检查完成，返回 {} 个结果", results.len());

        results
    }

    /// 本地版本检查（内部辅助方法）
    async fn check_version_local(
        &self,
        tool: &Tool,
        installed_version: Option<String>,
    ) -> Result<VersionInfo> {
        let latest_version = self.get_latest_from_local(tool).await?;
        let has_update = Self::compare_versions(installed_version.as_deref(), &latest_version);

        Ok(VersionInfo {
            tool_id: tool.id.clone(),
            installed_version,
            latest_version: Some(latest_version),
            mirror_version: None,   // 本地检查没有镜像版本信息
            mirror_is_stale: false, // 本地检查无法判断镜像状态
            has_update,
            source: VersionSource::MirrorFallback,
        })
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
