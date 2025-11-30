use crate::utils::CommandResult;
use anyhow::{Context, Result};
use std::process::Command;
use std::time::Duration;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// WSL 命令执行器
pub struct WSLExecutor;

impl WSLExecutor {
    /// 创建新的 WSL 执行器
    pub fn new() -> Self {
        Self
    }

    /// 检测 WSL 是否可用（仅 Windows 平台）
    pub fn is_available() -> bool {
        #[cfg(target_os = "windows")]
        {
            // 尝试执行 wsl.exe --status
            match Command::new("wsl.exe")
                .arg("--status")
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .output()
            {
                Ok(output) => output.status.success(),
                Err(_) => false,
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            false
        }
    }

    /// 列出所有已安装的 WSL 发行版
    pub fn list_distributions() -> Result<Vec<String>> {
        #[cfg(target_os = "windows")]
        {
            let output = Command::new("wsl.exe")
                .arg("--list")
                .arg("--quiet")
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .output()
                .context("执行 wsl --list 失败")?;

            if !output.status.success() {
                return Err(anyhow::anyhow!("WSL --list 命令执行失败"));
            }

            // 解析输出（可能是 UTF-16 编码）
            let distros = if output.stdout.starts_with(&[0xFF, 0xFE]) {
                // UTF-16 LE BOM
                String::from_utf16_lossy(
                    &output
                        .stdout
                        .chunks_exact(2)
                        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                        .collect::<Vec<u16>>(),
                )
            } else {
                String::from_utf8_lossy(&output.stdout).to_string()
            };

            // 解析每一行，过滤空行和特殊字符
            let distributions: Vec<String> = distros
                .lines()
                .map(|line| {
                    line.trim()
                        .replace(['\0', '\u{feff}'], "") // BOM
                        .trim()
                        .to_string()
                })
                .filter(|line| !line.is_empty())
                .collect();

            Ok(distributions)
        }

        #[cfg(not(target_os = "windows"))]
        {
            Err(anyhow::anyhow!("WSL 仅在 Windows 平台可用"))
        }
    }

    /// 执行 WSL 命令（使用默认发行版）
    pub async fn execute(&self, command: &str) -> Result<CommandResult> {
        self.execute_in_distro(None, command).await
    }

    /// 在指定的 WSL 发行版中执行命令
    pub async fn execute_in_distro(
        &self,
        distro_name: Option<&str>,
        command: &str,
    ) -> Result<CommandResult> {
        #[cfg(target_os = "windows")]
        {
            self.execute_windows(distro_name, command).await
        }

        #[cfg(not(target_os = "windows"))]
        {
            Err(anyhow::anyhow!("WSL 仅在 Windows 平台可用"))
        }
    }

    /// Windows 平台下执行 WSL 命令
    #[cfg(target_os = "windows")]
    async fn execute_windows(
        &self,
        distro_name: Option<&str>,
        command: &str,
    ) -> Result<CommandResult> {
        let command = command.to_string();
        let distro_name = distro_name.map(|s| s.to_string());

        tokio::task::spawn_blocking(move || {
            let mut cmd = Command::new("wsl.exe");

            // 如果指定了发行版，添加 -d 参数
            if let Some(distro) = distro_name {
                cmd.arg("-d").arg(distro);
            }

            let output = cmd
                .arg("--exec")
                .arg("bash")
                .arg("-c")
                .arg(&command)
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .output()
                .context("执行 WSL 命令失败")?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let success = output.status.success();
            let exit_code = output.status.code();

            Ok(CommandResult {
                success,
                stdout,
                stderr,
                exit_code,
            })
        })
        .await
        .context("WSL 命令执行器 spawn 失败")?
    }

    /// 带超时的执行（使用默认发行版）
    pub async fn execute_with_timeout(
        &self,
        command: &str,
        timeout: Duration,
    ) -> Result<CommandResult> {
        self.execute_with_timeout_in_distro(None, command, timeout)
            .await
    }

    /// 带超时的执行（指定发行版）
    pub async fn execute_with_timeout_in_distro(
        &self,
        distro_name: Option<&str>,
        command: &str,
        timeout: Duration,
    ) -> Result<CommandResult> {
        match tokio::time::timeout(timeout, self.execute_in_distro(distro_name, command)).await {
            Ok(result) => result,
            Err(_) => Err(anyhow::anyhow!("WSL 命令执行超时")),
        }
    }

    /// 检测工具是否已安装（使用默认发行版）
    pub async fn check_tool_installed(&self, command: &str) -> bool {
        self.check_tool_installed_in_distro(None, command).await
    }

    /// 检测工具是否已安装（指定发行版）
    pub async fn check_tool_installed_in_distro(
        &self,
        distro_name: Option<&str>,
        command: &str,
    ) -> bool {
        let check_cmd = format!("which {}", command);
        match self
            .execute_with_timeout_in_distro(distro_name, &check_cmd, Duration::from_secs(10))
            .await
        {
            Ok(result) => result.success && !result.stdout.trim().is_empty(),
            Err(_) => false,
        }
    }

    /// 获取工具版本（使用默认发行版）
    pub async fn get_tool_version(&self, command: &str) -> Option<String> {
        self.get_tool_version_in_distro(None, command).await
    }

    /// 获取工具版本（指定发行版）
    pub async fn get_tool_version_in_distro(
        &self,
        distro_name: Option<&str>,
        command: &str,
    ) -> Option<String> {
        let version_cmd = format!("{} --version", command);
        match self
            .execute_with_timeout_in_distro(distro_name, &version_cmd, Duration::from_secs(10))
            .await
        {
            Ok(result) if result.success => self.extract_version(&result.stdout),
            _ => None,
        }
    }

    /// 获取工具安装路径（使用默认发行版）
    pub async fn get_tool_path(&self, command: &str) -> Option<String> {
        self.get_tool_path_in_distro(None, command).await
    }

    /// 获取工具安装路径（指定发行版）
    pub async fn get_tool_path_in_distro(
        &self,
        distro_name: Option<&str>,
        command: &str,
    ) -> Option<String> {
        let which_cmd = format!("which {}", command);
        match self
            .execute_with_timeout_in_distro(distro_name, &which_cmd, Duration::from_secs(10))
            .await
        {
            Ok(result) if result.success => {
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

    /// 从输出中提取版本号
    fn extract_version(&self, output: &str) -> Option<String> {
        // 匹配版本号格式: v1.2.3 或 1.2.3
        let re = regex::Regex::new(r"v?(\d+\.\d+\.\d+(?:-[\w.]+)?)").ok()?;
        re.captures(output)?.get(1).map(|m| m.as_str().to_string())
    }

    /// 检测工具的完整信息（安装状态、版本、路径）使用默认发行版
    pub async fn detect_tool(
        &self,
        command: &str,
    ) -> Result<(bool, Option<String>, Option<String>)> {
        self.detect_tool_in_distro(None, command).await
    }

    /// 检测工具的完整信息（安装状态、版本、路径）指定发行版
    pub async fn detect_tool_in_distro(
        &self,
        distro_name: Option<&str>,
        command: &str,
    ) -> Result<(bool, Option<String>, Option<String>)> {
        // 并行检测安装状态和路径
        let check_installed = self.check_tool_installed_in_distro(distro_name, command);
        let get_path = self.get_tool_path_in_distro(distro_name, command);

        let (installed, install_path) = tokio::join!(check_installed, get_path);

        // 如果已安装，获取版本
        let version = if installed {
            self.get_tool_version_in_distro(distro_name, command).await
        } else {
            None
        };

        Ok((installed, version, install_path))
    }
}

impl Default for WSLExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[cfg(target_os = "windows")]
    async fn test_wsl_available() {
        let available = WSLExecutor::is_available();
        println!("WSL available: {}", available);
    }

    #[tokio::test]
    #[cfg(target_os = "windows")]
    async fn test_execute_simple_command() {
        if !WSLExecutor::is_available() {
            println!("WSL not available, skipping test");
            return;
        }

        let executor = WSLExecutor::new();
        let result = executor.execute("echo 'Hello WSL'").await.unwrap();
        assert!(result.success);
        assert!(result.stdout.contains("Hello WSL"));
    }

    #[tokio::test]
    #[cfg(target_os = "windows")]
    async fn test_check_tool_installed() {
        if !WSLExecutor::is_available() {
            println!("WSL not available, skipping test");
            return;
        }

        let executor = WSLExecutor::new();
        // bash should be installed in WSL
        let bash_installed = executor.check_tool_installed("bash").await;
        println!("bash installed: {}", bash_installed);
    }
}
