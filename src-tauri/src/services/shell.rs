use std::env;
use std::fs;
use std::process::{Command, Output};

#[cfg(target_os = "windows")]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

use crate::error::{AppError, AppResult};

pub struct CommandRunner;

impl CommandRunner {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&self, cmd: &str) -> AppResult<Output> {
        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .env("PATH", extended_path())
                .arg("/C")
                .arg(cmd)
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(AppError::from)
        }

        #[cfg(not(target_os = "windows"))]
        {
            Command::new("sh")
                .env("PATH", extended_path())
                .arg("-c")
                .arg(cmd)
                .output()
                .map_err(AppError::from)
        }
    }
}

pub fn extended_path() -> String {
    #[cfg(target_os = "windows")]
    {
        let user_profile =
            env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".to_string());

        let system_paths = vec![
            format!("{}\\AppData\\Local\\Programs\\claude-code", user_profile),
            format!("{}\\AppData\\Roaming\\npm", user_profile),
            format!(
                "{}\\AppData\\Local\\Programs\\Python\\Python312",
                user_profile
            ),
            format!(
                "{}\\AppData\\Local\\Programs\\Python\\Python312\\Scripts",
                user_profile
            ),
            "C:\\Program Files\\nodejs".to_string(),
            "C:\\Program Files\\Git\\cmd".to_string(),
            "C:\\Windows\\System32".to_string(),
            "C:\\Windows".to_string(),
        ];

        let current_path = env::var("PATH").unwrap_or_default();
        format!("{};{}", system_paths.join(";"), current_path)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home_dir = env::var("HOME").unwrap_or_else(|_| "/Users/default".to_string());

        let mut system_paths = vec![
            format!("{}/.local/bin", home_dir),
            format!("{}/.claude/bin", home_dir),
            format!("{}/.claude/local", home_dir),
            "/opt/homebrew/bin".to_string(),
            "/usr/local/bin".to_string(),
            "/usr/bin".to_string(),
            "/bin".to_string(),
            "/usr/sbin".to_string(),
            "/sbin".to_string(),
        ];

        let nvm_dir = format!("{}/.nvm/versions/node", home_dir);
        if let Ok(entries) = fs::read_dir(&nvm_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let bin_path = entry.path().join("bin");
                        if bin_path.exists() {
                            system_paths.push(bin_path.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        format!(
            "{}:{}",
            system_paths.join(":"),
            env::var("PATH").unwrap_or_default()
        )
    }
}
