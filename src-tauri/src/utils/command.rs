use super::platform::PlatformInfo;
use std::io;
use std::process::{Command, Output};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// 命令执行结果
#[derive(Debug)]
pub struct CommandResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

impl CommandResult {
    pub fn from_output(output: Output) -> Self {
        CommandResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            exit_code: output.status.code(),
        }
    }

    pub fn from_error(error: io::Error) -> Self {
        CommandResult {
            success: false,
            stdout: String::new(),
            stderr: error.to_string(),
            exit_code: None,
        }
    }
}

/// 命令执行器
#[derive(Clone)]
pub struct CommandExecutor {
    platform: PlatformInfo,
}

impl CommandExecutor {
    pub fn new() -> Self {
        CommandExecutor {
            platform: PlatformInfo::current(),
        }
    }

    /// 执行命令（使用增强的 PATH）
    pub fn execute(&self, command_str: &str) -> CommandResult {
        let enhanced_path = self.platform.build_enhanced_path();

        let output = if self.platform.is_windows {
            #[cfg(target_os = "windows")]
            {
                Command::new("cmd")
                    .args(["/C", command_str])
                    .creation_flags(0x08000000) // CREATE_NO_WINDOW
                    .env("PATH", enhanced_path)
                    .output()
            }
            #[cfg(not(target_os = "windows"))]
            {
                Command::new("cmd")
                    .args(["/C", command_str])
                    .env("PATH", enhanced_path)
                    .output()
            }
        } else {
            Command::new("sh")
                .args(["-c", command_str])
                .env("PATH", enhanced_path)
                .output()
        };

        match output {
            Ok(output) => CommandResult::from_output(output),
            Err(e) => CommandResult::from_error(e),
        }
    }

    /// 执行命令（异步）
    pub async fn execute_async(&self, command_str: &str) -> CommandResult {
        let command_str = command_str.to_string();
        let platform = self.platform.clone();

        tokio::task::spawn_blocking(move || {
            let executor = CommandExecutor { platform };
            executor.execute(&command_str)
        })
        .await
        .unwrap_or_else(|e| CommandResult {
            success: false,
            stdout: String::new(),
            stderr: format!("任务执行失败: {e}"),
            exit_code: None,
        })
    }

    /// 检查命令是否存在
    pub fn command_exists(&self, command: &str) -> bool {
        // 从命令字符串中提取命令名（第一个词）
        // 例如: "claude --version" -> "claude"
        let cmd_name = command.split_whitespace().next().unwrap_or(command);

        let check_cmd = if self.platform.is_windows {
            format!("where {cmd_name}")
        } else {
            format!("which {cmd_name}")
        };

        self.execute(&check_cmd).success
    }

    /// 检查命令是否存在（异步）
    pub async fn command_exists_async(&self, command: &str) -> bool {
        // 从命令字符串中提取命令名（第一个词）
        // 例如: "claude --version" -> "claude"
        let cmd_name = command.split_whitespace().next().unwrap_or(command);

        let check_cmd = if self.platform.is_windows {
            format!("where {cmd_name}")
        } else {
            format!("which {cmd_name}")
        };

        tracing::info!(
            "检查命令是否存在: command={}, cmd_name={}, check_cmd={}",
            command,
            cmd_name,
            check_cmd
        );

        let result = self.execute_async(&check_cmd).await;

        tracing::info!(
            "命令检查结果: command={}, cmd_name={}, success={}, stdout={:?}, stderr={:?}",
            command,
            cmd_name,
            result.success,
            result.stdout.trim(),
            result.stderr.trim()
        );

        result.success
    }
}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_executor() {
        let executor = CommandExecutor::new();

        // Test a command that should exist on all platforms
        let result = executor.execute("echo test");

        assert!(result.success);
        assert!(result.stdout.contains("test"));
    }

    #[test]
    fn test_command_exists() {
        let executor = CommandExecutor::new();

        // Test that echo/cmd exists
        if cfg!(windows) {
            assert!(executor.command_exists("cmd"));
        } else {
            assert!(executor.command_exists("sh"));
        }
    }

    #[tokio::test]
    async fn test_async_execution() {
        let executor = CommandExecutor::new();
        let result = executor.execute_async("echo async_test").await;

        assert!(result.success);
    }
}
