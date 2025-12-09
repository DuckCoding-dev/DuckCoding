// CodeX Detector
//
// CodeX 工具的检测、安装、配置管理实现

use super::super::detector_trait::ToolDetector;
use crate::data::DataManager;
use crate::models::InstallMethod;
use crate::services::version::{VersionInfo, VersionService};
use crate::utils::CommandExecutor;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;

/// CodeX 工具检测器
pub struct CodeXDetector {
    config_dir: PathBuf,
}

impl CodeXDetector {
    pub fn new() -> Self {
        let home_dir = dirs::home_dir().expect("无法获取用户主目录");
        Self {
            config_dir: home_dir.join(".codex"),
        }
    }
}

impl Default for CodeXDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolDetector for CodeXDetector {
    // ==================== 基础信息 ====================

    fn tool_id(&self) -> &str {
        "codex"
    }

    fn tool_name(&self) -> &str {
        "CodeX"
    }

    fn config_dir(&self) -> PathBuf {
        self.config_dir.clone()
    }

    fn config_file(&self) -> &str {
        "config.toml"
    }

    fn npm_package(&self) -> &str {
        "@openai/codex"
    }

    fn check_command(&self) -> &str {
        "codex --version"
    }

    fn use_proxy_for_version_check(&self) -> bool {
        // CodeX 可以使用代理
        true
    }

    // ==================== 检测逻辑 ====================

    async fn detect_install_method(&self, executor: &CommandExecutor) -> Option<InstallMethod> {
        // 1. 检查是否通过 Homebrew cask 安装
        if executor.command_exists_async("brew").await {
            let result = executor
                .execute_async("brew list --cask codex 2>/dev/null")
                .await;
            if result.success && result.stdout.contains("codex") {
                return Some(InstallMethod::Brew);
            }
        }

        // 2. 检查是否通过 npm 安装
        if executor.command_exists_async("npm").await {
            let stderr_redirect = if cfg!(windows) {
                "2>nul"
            } else {
                "2>/dev/null"
            };
            let cmd = format!("npm list -g @openai/codex {stderr_redirect}");
            let result = executor.execute_async(&cmd).await;
            if result.success {
                return Some(InstallMethod::Npm);
            }
        }

        // 3. 默认使用官方安装（虽然未实现）
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
            InstallMethod::Official => {
                anyhow::bail!("CodeX 官方安装方法尚未实现，请使用 npm 或 Homebrew")
            }
            InstallMethod::Npm => self.install_npm(executor, force).await,
            InstallMethod::Brew => self.install_brew(executor).await,
            InstallMethod::Other => {
                anyhow::bail!("不支持 APP 内安装，请手动安装")
            }
        }
    }

    async fn update(&self, executor: &CommandExecutor, _force: bool) -> Result<()> {
        let method = self.detect_install_method(executor).await;

        match method {
            Some(InstallMethod::Npm) => self.update_npm(executor).await,
            Some(InstallMethod::Brew) => self.update_brew(executor).await,
            _ => anyhow::bail!("无法检测到安装方法"),
        }
    }

    // ==================== 配置管理 ====================

    async fn read_config(&self, manager: &DataManager) -> Result<Value> {
        let config_path = self.config_dir.join(self.config_file());

        // CodeX 使用 TOML 格式，需要转换为 JSON
        let toml_value = manager.toml().read(&config_path)?;

        // 转换为 serde_json::Value
        let json_str = serde_json::to_string(&toml_value)?;
        let json_value: Value = serde_json::from_str(&json_str)?;

        Ok(json_value)
    }

    async fn save_config(&self, manager: &DataManager, config: Value) -> Result<()> {
        let config_path = self.config_dir.join(self.config_file());

        // 读取原有文档（保留注释和格式）
        let mut doc = manager.toml().read_document(&config_path)?;

        // 将 JSON Value 转换为 TOML 并更新每个键值
        // 注意：这里需要逐个更新以保留原有格式和注释
        if let Some(obj) = config.as_object() {
            for (key, value) in obj {
                // 将 JSON value 转换为字符串，再解析为 TOML
                let toml_value_str = serde_json::to_string(value)?;
                // 简单类型直接转换
                match value {
                    Value::String(s) => {
                        doc[key] = toml_edit::value(s.clone());
                    }
                    Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            doc[key] = toml_edit::value(i);
                        } else if let Some(f) = n.as_f64() {
                            doc[key] = toml_edit::value(f);
                        }
                    }
                    Value::Bool(b) => {
                        doc[key] = toml_edit::value(*b);
                    }
                    _ => {
                        // 复杂类型：使用字符串解析
                        if let Ok(parsed) = toml_value_str.parse::<toml_edit::Value>() {
                            doc[key] = toml_edit::value(parsed);
                        }
                    }
                }
            }
        }

        manager.toml().write(&config_path, &doc)?;
        Ok(())
    }
}

// ==================== 私有实现方法 ====================

impl CodeXDetector {
    /// 使用 npm 安装
    async fn install_npm(&self, executor: &CommandExecutor, force: bool) -> Result<()> {
        if !executor.command_exists_async("npm").await {
            anyhow::bail!("npm 未安装");
        }

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
            Some(version) if !version.is_empty() => format!("@openai/codex@{}", version),
            _ => "@openai/codex@latest".to_string(),
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

    /// 使用 Homebrew 安装
    async fn install_brew(&self, executor: &CommandExecutor) -> Result<()> {
        if !cfg!(target_os = "macos") {
            anyhow::bail!("❌ Homebrew 仅支持 macOS");
        }

        if !executor.command_exists_async("brew").await {
            anyhow::bail!("❌ Homebrew 未安装");
        }

        let command = "brew install --cask codex";
        let result = executor.execute_async(command).await;

        if result.success {
            Ok(())
        } else {
            anyhow::bail!("❌ Homebrew 安装失败\n\n{}", result.stderr)
        }
    }

    /// 使用 npm 更新
    async fn update_npm(&self, executor: &CommandExecutor) -> Result<()> {
        let command = "npm update -g @openai/codex --registry https://registry.npmmirror.com";
        let result = executor.execute_async(command).await;

        if result.success {
            Ok(())
        } else {
            anyhow::bail!("❌ npm 更新失败\n\n{}", result.stderr)
        }
    }

    /// 使用 Homebrew 更新
    async fn update_brew(&self, executor: &CommandExecutor) -> Result<()> {
        let command = "brew upgrade --cask codex";
        let result = executor.execute_async(command).await;

        if result.success {
            Ok(())
        } else {
            let error_str = result.stderr;

            // 检查是否是 Homebrew 版本滞后
            if error_str.contains("Not upgrading") && error_str.contains("already installed") {
                anyhow::bail!(
                    "⚠️ Homebrew版本滞后\n\n推荐切换到 npm 安装：\n\
                     1. brew uninstall --cask codex\n\
                     2. npm install -g @openai/codex --registry https://registry.npmmirror.com"
                );
            }

            anyhow::bail!("❌ Homebrew 更新失败\n\n{}", error_str)
        }
    }

    /// 转换为旧版 Tool 结构
    fn to_legacy_tool(&self) -> crate::models::Tool {
        crate::models::Tool::codex()
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
        let detector = CodeXDetector::new();
        assert_eq!(detector.tool_id(), "codex");
        assert_eq!(detector.tool_name(), "CodeX");
        assert_eq!(detector.npm_package(), "@openai/codex");
        assert!(detector.use_proxy_for_version_check());
    }
}
