// Claude Code Detector
//
// Claude Code 工具的检测、安装、配置管理实现

use super::super::detector_trait::ToolDetector;
use crate::data::DataManager;
use crate::models::InstallMethod;
use crate::services::version::{VersionInfo, VersionService};
use crate::utils::CommandExecutor;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use std::process::Command;

/// Claude Code 工具检测器
pub struct ClaudeCodeDetector {
    config_dir: PathBuf,
}

impl ClaudeCodeDetector {
    pub fn new() -> Self {
        let home_dir = dirs::home_dir().expect("无法获取用户主目录");
        Self {
            config_dir: home_dir.join(".claude"),
        }
    }

    /// 检测 Windows 系统上可用的 PowerShell 版本
    #[cfg(target_os = "windows")]
    fn detect_powershell() -> (&'static str, bool) {
        // 优先检测 PowerShell 7+ (pwsh.exe)
        if Command::new("pwsh").arg("-Version").output().is_ok() {
            return ("pwsh", true);
        }

        // 回退到 PowerShell 5 (powershell.exe)，不支持 -OutputEncoding
        ("powershell", false)
    }
}

impl Default for ClaudeCodeDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolDetector for ClaudeCodeDetector {
    // ==================== 基础信息 ====================

    fn tool_id(&self) -> &str {
        "claude-code"
    }

    fn tool_name(&self) -> &str {
        "Claude Code"
    }

    fn config_dir(&self) -> PathBuf {
        self.config_dir.clone()
    }

    fn config_file(&self) -> &str {
        "settings.json"
    }

    fn npm_package(&self) -> &str {
        "@anthropic-ai/claude-code"
    }

    fn check_command(&self) -> &str {
        "claude --version"
    }

    fn use_proxy_for_version_check(&self) -> bool {
        // Claude Code 在代理环境下会出现 URL 协议错误
        false
    }

    // ==================== 检测逻辑 ====================

    async fn detect_install_method(&self, executor: &CommandExecutor) -> Option<InstallMethod> {
        // 检查是否通过 npm 安装
        if executor.command_exists_async("npm").await {
            let stderr_redirect = if cfg!(windows) {
                "2>nul"
            } else {
                "2>/dev/null"
            };
            let cmd = format!("npm list -g @anthropic-ai/claude-code {stderr_redirect}");
            let result = executor.execute_async(&cmd).await;
            if result.success {
                return Some(InstallMethod::Npm);
            }
        }

        // 默认使用官方安装方式
        Some(InstallMethod::Official)
    }

    // ==================== 安装逻辑 ====================

    async fn install(
        &self,
        executor: &CommandExecutor,
        method: &InstallMethod,
        force: bool,
    ) -> Result<()> {
        match method {
            InstallMethod::Official => self.install_official(executor, force).await,
            InstallMethod::Npm => self.install_npm(executor, force).await,
            InstallMethod::Brew => {
                anyhow::bail!("Claude Code 不支持 Homebrew 安装，请使用官方安装或 npm")
            }
            InstallMethod::Other => {
                anyhow::bail!("不支持 APP 内安装，请手动安装")
            }
        }
    }

    async fn update(&self, executor: &CommandExecutor, force: bool) -> Result<()> {
        // 检测当前安装方法
        let method = self.detect_install_method(executor).await;

        match method {
            Some(InstallMethod::Official) => {
                // 官方安装：重新执行安装脚本即可更新
                self.install_official(executor, force).await
            }
            Some(InstallMethod::Npm) => {
                // npm 安装：使用 npm update
                self.update_npm(executor).await
            }
            _ => anyhow::bail!("无法检测到安装方法，无法更新"),
        }
    }

    // ==================== 配置管理 ====================

    async fn read_config(&self, manager: &DataManager) -> Result<Value> {
        let config_path = self.config_dir.join(self.config_file());

        // 使用 uncached 避免配置文件变更不被检测
        let content = manager.json_uncached().read(&config_path)?;
        Ok(content)
    }

    async fn save_config(&self, manager: &DataManager, config: Value) -> Result<()> {
        let config_path = self.config_dir.join(self.config_file());

        // 使用 uncached 确保立即写入
        manager.json_uncached().write(&config_path, &config)?;
        Ok(())
    }
}

// ==================== 私有实现方法 ====================

impl ClaudeCodeDetector {
    /// 使用官方脚本安装（DuckCoding 镜像）
    async fn install_official(&self, executor: &CommandExecutor, force: bool) -> Result<()> {
        // 安装前先检查镜像状态
        if !force {
            let version_service = VersionService::new();
            if let Ok(info) = version_service.check_version(&self.to_legacy_tool()).await {
                if info.mirror_is_stale {
                    let mirror_ver = info.mirror_version.clone().unwrap_or_default();
                    let official_ver = info.latest_version.clone().unwrap_or_default();
                    anyhow::bail!("MIRROR_STALE|{mirror_ver}|{official_ver}");
                }
            }
        }

        let command = if cfg!(windows) {
            #[cfg(target_os = "windows")]
            {
                let (ps_exe, supports_encoding) = Self::detect_powershell();

                if supports_encoding {
                    // PowerShell 7+ 支持 -OutputEncoding
                    format!(
                        "{ps_exe} -NoProfile -ExecutionPolicy Bypass -OutputEncoding UTF8 -Command \"[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; irm https://mirror.duckcoding.com/claude-code/install.ps1 | iex\""
                    )
                } else {
                    // PowerShell 5 不支持 -OutputEncoding
                    format!(
                        "cmd /C \"chcp 65001 >nul && {ps_exe} -NoProfile -ExecutionPolicy Bypass -Command \\\"[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; irm https://mirror.duckcoding.com/claude-code/install.ps1 | iex\\\"\""
                    )
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                String::new()
            }
        } else {
            // macOS/Linux: 使用 DuckCoding 镜像
            "curl -fsSL https://mirror.duckcoding.com/claude-code/install.sh | bash".to_string()
        };

        let result = executor.execute_async(&command).await;

        if result.success {
            Ok(())
        } else {
            anyhow::bail!("❌ 官方脚本安装失败\n\n{}", result.stderr)
        }
    }

    /// 使用 npm 安装
    async fn install_npm(&self, executor: &CommandExecutor, force: bool) -> Result<()> {
        if !executor.command_exists_async("npm").await {
            anyhow::bail!("npm 未安装，请先安装 Node.js");
        }

        // 获取推荐版本
        let version_hint = if !force {
            let version_service = VersionService::new();
            version_service
                .check_version(&self.to_legacy_tool())
                .await
                .ok()
                .and_then(|info| Self::preferred_npm_version(&info))
        } else {
            None
        };

        let package_spec = match version_hint {
            Some(version) if !version.is_empty() => {
                format!("@anthropic-ai/claude-code@{}", version)
            }
            _ => "@anthropic-ai/claude-code@latest".to_string(),
        };

        let command =
            format!("npm install -g {package_spec} --registry https://registry.npmmirror.com");
        let result = executor.execute_async(&command).await;

        if result.success {
            Ok(())
        } else {
            anyhow::bail!("❌ npm 安装失败\n\n{}", result.stderr)
        }
    }

    /// 使用 npm 更新
    async fn update_npm(&self, executor: &CommandExecutor) -> Result<()> {
        let command =
            "npm update -g @anthropic-ai/claude-code --registry https://registry.npmmirror.com";
        let result = executor.execute_async(command).await;

        if result.success {
            Ok(())
        } else {
            anyhow::bail!("❌ npm 更新失败\n\n{}", result.stderr)
        }
    }

    /// 转换为旧版 Tool 结构（用于兼容 VersionService）
    fn to_legacy_tool(&self) -> crate::models::Tool {
        crate::models::Tool::claude_code()
    }

    /// 从版本信息中提取推荐的 npm 版本
    fn preferred_npm_version(info: &VersionInfo) -> Option<String> {
        info.mirror_version
            .clone()
            .or_else(|| info.latest_version.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_info() {
        let detector = ClaudeCodeDetector::new();
        assert_eq!(detector.tool_id(), "claude-code");
        assert_eq!(detector.tool_name(), "Claude Code");
        assert_eq!(detector.npm_package(), "@anthropic-ai/claude-code");
        assert_eq!(detector.check_command(), "claude --version");
        assert!(!detector.use_proxy_for_version_check());
    }
}
