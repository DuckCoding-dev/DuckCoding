use crate::models::{Tool, InstallMethod};
use crate::utils::CommandExecutor;
use anyhow::{Result, Context};

/// 安装服务
pub struct InstallerService {
    pub executor: CommandExecutor,
}

impl InstallerService {
    pub fn new() -> Self {
        InstallerService {
            executor: CommandExecutor::new(),
        }
    }

    /// 检测 Windows 系统上可用的 PowerShell 版本
    /// 返回：(可执行文件名, 是否支持 -OutputEncoding 参数)
    #[cfg(windows)]
    fn detect_powershell() -> (&'static str, bool) {
        use std::process::Command;

        // 优先检测 PowerShell 7+ (pwsh.exe)
        if Command::new("pwsh").arg("-Version").output().is_ok() {
            return ("pwsh", true);
        }

        // 回退到 PowerShell 5 (powershell.exe)，不支持 -OutputEncoding
        ("powershell", false)
    }

    /// 检查工具是否已安装
    pub async fn is_installed(&self, tool: &Tool) -> bool {
        self.executor.command_exists_async(&tool.check_command.split_whitespace().next().unwrap()).await
    }

    /// 获取已安装版本
    pub async fn get_installed_version(&self, tool: &Tool) -> Option<String> {
        let result = self.executor.execute_async(&tool.check_command).await;

        if result.success {
            Self::extract_version(&result.stdout)
        } else {
            None
        }
    }

    /// 从输出中提取版本号
    fn extract_version(output: &str) -> Option<String> {
        // 匹配版本号格式: v1.2.3 或 1.2.3
        let re = regex::Regex::new(r"v?(\d+\.\d+\.\d+(?:-[\w.]+)?)").ok()?;
        re.captures(output)?
            .get(1)
            .map(|m| m.as_str().to_string())
    }

    /// 检测工具的安装方法
    pub async fn detect_install_method(&self, tool: &Tool) -> Option<InstallMethod> {
        match tool.id.as_str() {
            "codex" => {
                // 检查是否通过 Homebrew cask 安装
                if self.executor.command_exists_async("brew").await {
                    let result = self.executor.execute_async("brew list --cask codex 2>/dev/null").await;
                    if result.success && result.stdout.contains("codex") {
                        return Some(InstallMethod::Brew);
                    }
                }

                // 检查是否通过 npm 安装
                if self.executor.command_exists_async("npm").await {
                    let stderr_redirect = if cfg!(windows) { "2>nul" } else { "2>/dev/null" };
                    let cmd = format!("npm list -g @openai/codex {}", stderr_redirect);
                    let result = self.executor.execute_async(&cmd).await;
                    if result.success {
                        return Some(InstallMethod::Npm);
                    }
                }

                Some(InstallMethod::Official)
            }
            "claude-code" => {
                // 检查是否通过 npm 安装
                if self.executor.command_exists_async("npm").await {
                    let stderr_redirect = if cfg!(windows) { "2>nul" } else { "2>/dev/null" };
                    let cmd = format!("npm list -g @anthropic-ai/claude-code {}", stderr_redirect);
                    let result = self.executor.execute_async(&cmd).await;
                    if result.success {
                        return Some(InstallMethod::Npm);
                    }
                }

                Some(InstallMethod::Official)
            }
            "gemini-cli" => {
                Some(InstallMethod::Npm)
            }
            _ => None,
        }
    }

    /// 安装工具
    pub async fn install(&self, tool: &Tool, method: &InstallMethod) -> Result<()> {
        match method {
            InstallMethod::Official => self.install_official(tool).await,
            InstallMethod::Npm => self.install_npm(tool).await,
            InstallMethod::Brew => self.install_brew(tool).await,
        }
    }

    /// 使用官方脚本安装（使用 DuckCoding 镜像加速）
    async fn install_official(&self, tool: &Tool) -> Result<()> {
        let command = match tool.id.as_str() {
            "claude-code" => {
                if cfg!(windows) {
                    // Windows: 检测 PowerShell 版本并生成兼容命令
                    #[cfg(windows)]
                    {
                        let (ps_exe, supports_encoding) = Self::detect_powershell();

                        if supports_encoding {
                            // PowerShell 7+ 支持 -OutputEncoding
                            format!(
                                "{} -NoProfile -ExecutionPolicy Bypass -OutputEncoding UTF8 -Command \"[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; irm https://mirror.duckcoding.com/claude-code/install.ps1 | iex\"",
                                ps_exe
                            )
                        } else {
                            // PowerShell 5 不支持 -OutputEncoding，使用 chcp 处理编码
                            format!(
                                "cmd /C \"chcp 65001 >nul && {} -NoProfile -ExecutionPolicy Bypass -Command \\\"[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; irm https://mirror.duckcoding.com/claude-code/install.ps1 | iex\\\"\"",
                                ps_exe
                            )
                        }
                    }
                    #[cfg(not(windows))]
                    {
                        String::new() // 不会执行到这里
                    }
                } else {
                    // macOS/Linux: 使用 DuckCoding 镜像
                    "curl -fsSL https://mirror.duckcoding.com/claude-code/install.sh | bash".to_string()
                }
            }
            "codex" => {
                // CodeX 官方安装命令（需要根据实际情况调整）
                anyhow::bail!("CodeX 官方安装方法尚未实现，请使用 npm 或 Homebrew")
            }
            _ => anyhow::bail!("工具 {} 不支持官方安装方法", tool.name),
        };

        let result = self.executor.execute_async(&command).await;

        if result.success {
            Ok(())
        } else {
            anyhow::bail!("❌ 安装失败\n\n错误信息：\n{}", result.stderr)
        }
    }

    /// 使用 npm 安装（使用国内镜像加速）
    async fn install_npm(&self, tool: &Tool) -> Result<()> {
        if !self.executor.command_exists_async("npm").await {
            anyhow::bail!("npm 未安装或未找到\n\n请先安装 Node.js (包含 npm):\n1. 访问 https://nodejs.org 下载安装\n2. 或使用官方安装方式（无需 npm）");
        }

        // 使用国内镜像加速
        let command = format!("npm install -g {} --registry https://registry.npmmirror.com", tool.npm_package);
        let result = self.executor.execute_async(&command).await;

        if result.success {
            Ok(())
        } else {
            anyhow::bail!("❌ npm 安装失败\n\n错误信息：\n{}", result.stderr)
        }
    }

    /// 使用 Homebrew 安装
    async fn install_brew(&self, tool: &Tool) -> Result<()> {
        if !cfg!(target_os = "macos") {
            anyhow::bail!("❌ Homebrew 仅支持 macOS\n\n请使用 npm 安装方式");
        }

        if !self.executor.command_exists_async("brew").await {
            anyhow::bail!("❌ Homebrew 未安装\n\n请先安装 Homebrew:\n访问 https://brew.sh 查看安装方法");
        }

        let command = match tool.id.as_str() {
            "codex" => "brew install --cask codex".to_string(),
            _ => anyhow::bail!("工具 {} 不支持 Homebrew 安装", tool.name),
        };

        let result = self.executor.execute_async(&command).await;

        if result.success {
            Ok(())
        } else {
            anyhow::bail!("❌ Homebrew 安装失败\n\n错误信息：\n{}", result.stderr)
        }
    }

    /// 更新工具
    pub async fn update(&self, tool: &Tool) -> Result<()> {
        let method = self.detect_install_method(tool).await
            .context("无法检测安装方法")?;

        match method {
            InstallMethod::Npm => {
                // 使用国内镜像加速
                let command = format!("npm install -g {}@latest --registry https://registry.npmmirror.com", tool.npm_package);
                let result = self.executor.execute_async(&command).await;

                if result.success {
                    Ok(())
                } else {
                    anyhow::bail!("❌ npm 更新失败\n\n错误信息：\n{}", result.stderr)
                }
            }
            InstallMethod::Brew => {
                let command = match tool.id.as_str() {
                    "codex" => "brew upgrade --cask codex",
                    _ => anyhow::bail!("工具 {} 不支持 Homebrew 更新", tool.name),
                };

                let result = self.executor.execute_async(command).await;

                if result.success {
                    Ok(())
                } else {
                    anyhow::bail!("❌ Homebrew 更新失败\n\n错误信息：\n{}", result.stderr)
                }
            }
            InstallMethod::Official => {
                // 官方安装方法通常需要重新运行安装脚本（使用DuckCoding镜像）
                self.install_official(tool).await
            }
        }
    }
}

impl Default for InstallerService {
    fn default() -> Self {
        Self::new()
    }
}
