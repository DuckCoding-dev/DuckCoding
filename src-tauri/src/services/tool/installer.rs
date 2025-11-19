use crate::models::{InstallMethod, Tool};
use crate::services::version::{VersionInfo, VersionService};
use crate::utils::{platform::PlatformInfo, CommandExecutor};
use anyhow::{Context, Result};
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

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
        if let Some(command) = tool.check_command.split_whitespace().next() {
            self.executor.command_exists_async(command).await
        } else {
            false
        }
    }

    /// 获取已安装版本
    pub async fn get_installed_version(&self, tool: &Tool) -> Option<String> {
        // 根据工具配置决定是否使用代理
        let result = if tool.use_proxy_for_version_check {
            self.executor.execute_async(&tool.check_command).await
        } else {
            self.execute_without_proxy(&tool.check_command).await
        };

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
        re.captures(output)?.get(1).map(|m| m.as_str().to_string())
    }

    /// 执行命令但不使用代理（用于版本检查）
    async fn execute_without_proxy(&self, command_str: &str) -> crate::utils::CommandResult {
        let command_str = command_str.to_string();
        let platform = PlatformInfo::current();

        tokio::task::spawn_blocking(move || {
            let enhanced_path = platform.build_enhanced_path();

            // 创建不继承代理环境的命令
            let output = if platform.is_windows {
                #[cfg(target_os = "windows")]
                {
                    Command::new("cmd")
                        .args(["/C", &command_str])
                        .creation_flags(0x08000000) // CREATE_NO_WINDOW
                        .env("PATH", enhanced_path)
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
                        .env("PATH", enhanced_path)
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
                    .env("PATH", enhanced_path)
                    .env_remove("HTTP_PROXY")
                    .env_remove("HTTPS_PROXY")
                    .env_remove("ALL_PROXY")
                    .env_remove("http_proxy")
                    .env_remove("https_proxy")
                    .env_remove("all_proxy")
                    .output()
            };

            match output {
                Ok(output) => crate::utils::CommandResult::from_output(output),
                Err(e) => crate::utils::CommandResult::from_error(e),
            }
        })
        .await
        .unwrap_or_else(|e| crate::utils::CommandResult {
            success: false,
            stdout: String::new(),
            stderr: format!("任务执行失败: {}", e),
            exit_code: None,
        })
    }

    /// 检测工具的安装方法
    pub async fn detect_install_method(&self, tool: &Tool) -> Option<InstallMethod> {
        match tool.id.as_str() {
            "codex" => {
                // 检查是否通过 Homebrew cask 安装
                if self.executor.command_exists_async("brew").await {
                    let result = self
                        .executor
                        .execute_async("brew list --cask codex 2>/dev/null")
                        .await;
                    if result.success && result.stdout.contains("codex") {
                        return Some(InstallMethod::Brew);
                    }
                }

                // 检查是否通过 npm 安装
                if self.executor.command_exists_async("npm").await {
                    let stderr_redirect = if cfg!(windows) {
                        "2>nul"
                    } else {
                        "2>/dev/null"
                    };
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
                    let stderr_redirect = if cfg!(windows) {
                        "2>nul"
                    } else {
                        "2>/dev/null"
                    };
                    let cmd = format!("npm list -g @anthropic-ai/claude-code {}", stderr_redirect);
                    let result = self.executor.execute_async(&cmd).await;
                    if result.success {
                        return Some(InstallMethod::Npm);
                    }
                }

                Some(InstallMethod::Official)
            }
            "gemini-cli" => Some(InstallMethod::Npm),
            _ => None,
        }
    }

    /// 安装工具
    pub async fn install(&self, tool: &Tool, method: &InstallMethod, force: bool) -> Result<()> {
        // 官方脚本 / npm 安装需要提前获取版本信息
        let mut version_info: Option<VersionInfo> = None;
        if matches!(method, InstallMethod::Official | InstallMethod::Npm) {
            let version_service = VersionService::new();
            match version_service.check_version(tool).await {
                Ok(info) => version_info = Some(info),
                Err(e) => eprintln!("⚠️  无法检查镜像状态: {}", e),
            }
        }

        // 如果使用官方脚本（镜像）安装，且未强制执行，则先检查镜像状态
        if matches!(method, InstallMethod::Official) && !force {
            if let Some(info) = &version_info {
                if info.mirror_is_stale {
                    let mirror_ver = info.mirror_version.clone().unwrap_or_default();
                    let official_ver = info.latest_version.clone().unwrap_or_default();

                    anyhow::bail!("MIRROR_STALE|{}|{}", mirror_ver, official_ver);
                }
            }
        }

        // 针对 npm 安装，优先使用镜像/官方最新的具体版本号，避免 @latest 无法获取 preview 等版本
        let npm_version_hint = version_info.as_ref().and_then(Self::preferred_npm_version);

        // 执行安装
        match method {
            InstallMethod::Official => self.install_official(tool).await,
            InstallMethod::Npm => self.install_npm(tool, npm_version_hint.as_deref()).await,
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
                    "curl -fsSL https://mirror.duckcoding.com/claude-code/install.sh | bash"
                        .to_string()
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
    async fn install_npm(&self, tool: &Tool, version_hint: Option<&str>) -> Result<()> {
        if !self.executor.command_exists_async("npm").await {
            anyhow::bail!("npm 未安装或未找到\n\n请先安装 Node.js (包含 npm):\n1. 访问 https://nodejs.org 下载安装\n2. 或使用官方安装方式（无需 npm）");
        }

        let package_spec = match version_hint {
            Some(version) if !version.is_empty() => format!("{}@{}", tool.npm_package, version),
            _ => format!("{}@latest", tool.npm_package),
        };

        // 使用国内镜像加速
        let command = format!(
            "npm install -g {} --registry https://registry.npmmirror.com",
            package_spec
        );
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
            anyhow::bail!(
                "❌ Homebrew 未安装\n\n请先安装 Homebrew:\n访问 https://brew.sh 查看安装方法"
            );
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
    pub async fn update(&self, tool: &Tool, force: bool) -> Result<()> {
        let method = self
            .detect_install_method(tool)
            .await
            .context("无法检测安装方法")?;

        // 官方脚本 / npm 更新需要提前获取版本信息
        let mut version_info: Option<VersionInfo> = None;
        if matches!(method, InstallMethod::Official | InstallMethod::Npm) {
            let version_service = VersionService::new();
            match version_service.check_version(tool).await {
                Ok(info) => version_info = Some(info),
                Err(e) => eprintln!("⚠️  无法检查镜像状态: {}", e),
            }
        }

        // 如果使用官方脚本（镜像）更新，且未强制执行，则先检查镜像状态
        if matches!(method, InstallMethod::Official) && !force {
            if let Some(info) = &version_info {
                if info.mirror_is_stale {
                    let mirror_ver = info.mirror_version.clone().unwrap_or_default();
                    let official_ver = info.latest_version.clone().unwrap_or_default();

                    anyhow::bail!("MIRROR_STALE|{}|{}", mirror_ver, official_ver);
                }
            }
        }

        let npm_version_hint = version_info.as_ref().and_then(Self::preferred_npm_version);

        match method {
            InstallMethod::Npm => self.install_npm(tool, npm_version_hint.as_deref()).await,
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

    /// 选择适合 npm 安装的目标版本（优先镜像已同步的版本，滞后时使用官方最新）
    fn preferred_npm_version(info: &VersionInfo) -> Option<String> {
        let candidate = if info.mirror_is_stale {
            info.latest_version.as_deref()
        } else {
            info.mirror_version
                .as_deref()
                .or(info.latest_version.as_deref())
        }?;

        Self::extract_version(candidate)
    }
}

impl Default for InstallerService {
    fn default() -> Self {
        Self::new()
    }
}
