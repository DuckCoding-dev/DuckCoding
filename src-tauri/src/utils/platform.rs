use std::env;

/// 平台信息
#[derive(Debug, Clone)]
pub struct PlatformInfo {
    pub os: String,
    pub arch: String,
    pub is_windows: bool,
    pub is_macos: bool,
    pub is_linux: bool,
}

impl PlatformInfo {
    /// 获取当前平台信息
    pub fn current() -> Self {
        let os = env::consts::OS.to_string();
        let arch = env::consts::ARCH.to_string();

        PlatformInfo {
            is_windows: os == "windows",
            is_macos: os == "macos",
            is_linux: os == "linux",
            os,
            arch,
        }
    }

    /// 获取平台标识符（用于下载）
    pub fn platform_id(&self) -> String {
        match (self.os.as_str(), self.arch.as_str()) {
            ("macos", "aarch64") => "darwin-arm64".to_string(),
            ("macos", "x86_64") => "darwin-x64".to_string(),
            ("linux", "x86_64") => "linux-x64".to_string(),
            ("linux", "aarch64") => "linux-arm64".to_string(),
            ("windows", "x86_64") => "win32-x64".to_string(),
            ("windows", "aarch64") => "win32-arm64".to_string(),
            _ => format!("{}-{}", self.os, self.arch),
        }
    }

    /// 获取 PATH 分隔符
    pub fn path_separator(&self) -> &str {
        if self.is_windows {
            ";"
        } else {
            ":"
        }
    }

    /// 构建增强的 PATH 环境变量
    pub fn build_enhanced_path(&self) -> String {
        let separator = self.path_separator();
        let current_path = env::var("PATH").unwrap_or_default();

        let system_paths = if self.is_windows {
            self.windows_system_paths()
        } else {
            self.unix_system_paths()
        };

        format!("{}{}{}", system_paths.join(separator), separator, current_path)
    }

    /// Windows 系统路径
    fn windows_system_paths(&self) -> Vec<String> {
        let mut paths = vec![
            "C:\\Program Files\\nodejs".to_string(),
            "C:\\Program Files (x86)\\nodejs".to_string(),
        ];

        if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
            paths.push(format!("{}\\Programs\\claude\\bin", local_app_data));
        }

        if let Ok(user_profile) = env::var("USERPROFILE") {
            paths.push(format!("{}\\.claude\\bin", user_profile));
            paths.push(format!("{}\\.local\\bin", user_profile));
        }

        paths
    }

    /// Unix 系统路径
    fn unix_system_paths(&self) -> Vec<String> {
        let mut paths = vec![
            "/opt/homebrew/bin".to_string(),
            "/usr/local/bin".to_string(),
            "/usr/bin".to_string(),
            "/bin".to_string(),
            "/usr/sbin".to_string(),
            "/sbin".to_string(),
        ];

        if let Some(home_dir) = dirs::home_dir() {
            let home_str = home_dir.to_string_lossy();
            paths.insert(0, format!("{}/.local/bin", home_str));
            paths.insert(0, format!("{}/.claude/bin", home_str));
            paths.insert(0, format!("{}/.claude/local", home_str));

            // NVM 支持 - 优先使用当前激活的版本
            if let Ok(nvm_dir) = std::env::var("NVM_DIR") {
                // 检查 nvm current symlink
                let nvm_current = format!("{}/current/bin", nvm_dir);
                if std::path::Path::new(&nvm_current).exists() {
                    paths.insert(0, nvm_current);
                } else {
                    // 如果没有 current symlink，尝试使用 default
                    let nvm_default = format!("{}/.nvm/versions/node/default/bin", home_str);
                    if std::path::Path::new(&nvm_default).exists() {
                        paths.insert(0, nvm_default);
                    }
                }
            } else {
                // 如果 NVM_DIR 未设置，尝试默认路径
                let nvm_current = format!("{}/.nvm/current/bin", home_str);
                if std::path::Path::new(&nvm_current).exists() {
                    paths.insert(0, nvm_current);
                }
            }

            // npm global bin 支持 - 检查自定义 npm prefix
            if let Ok(npm_prefix) = std::env::var("NPM_CONFIG_PREFIX") {
                paths.insert(0, format!("{}/bin", npm_prefix));
            } else {
                // 默认 npm global bin 路径
                paths.push(format!("{}/.npm-global/bin", home_str));
            }
        }

        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = PlatformInfo::current();
        assert!(!platform.os.is_empty());
        assert!(!platform.arch.is_empty());
    }

    #[test]
    fn test_path_separator() {
        let platform = PlatformInfo::current();
        if cfg!(windows) {
            assert_eq!(platform.path_separator(), ";");
        } else {
            assert_eq!(platform.path_separator(), ":");
        }
    }

    #[test]
    fn test_platform_id() {
        let platform = PlatformInfo::current();
        let id = platform.platform_id();
        assert!(id.contains("-"));
    }
}
