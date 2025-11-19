use crate::services::proxy::ProxyService;
use crate::GlobalConfig;
use std::fs;
use std::path::PathBuf;

/// DuckCoding 配置目录 (~/.duckcoding)，若不存在则创建
pub fn config_dir() -> Result<PathBuf, String> {
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;
    let config_dir = home_dir.join(".duckcoding");
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    Ok(config_dir)
}

/// 全局配置文件路径
pub fn global_config_path() -> Result<PathBuf, String> {
    Ok(config_dir()?.join("config.json"))
}

/// 读取全局配置（若文件不存在返回 Ok(None)）
pub fn read_global_config() -> Result<Option<GlobalConfig>, String> {
    let config_path = global_config_path()?;
    if !config_path.exists() {
        return Ok(None);
    }

    let content =
        fs::read_to_string(&config_path).map_err(|e| format!("Failed to read config: {}", e))?;
    let config: GlobalConfig =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse config: {}", e))?;

    Ok(Some(config))
}

/// 写入全局配置，同时设置权限并更新当前进程代理
pub fn write_global_config(config: &GlobalConfig) -> Result<(), String> {
    let config_path = global_config_path()?;
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&config_path, json).map_err(|e| format!("Failed to write config: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(&config_path)
            .map_err(|e| format!("Failed to get file metadata: {}", e))?;
        let mut perms = metadata.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&config_path, perms)
            .map_err(|e| format!("Failed to set file permissions: {}", e))?;
    }

    ProxyService::apply_proxy_from_config(config);
    Ok(())
}

/// 如配置存在代理设置，则立即应用到环境变量
pub fn apply_proxy_if_configured() {
    if let Ok(Some(config)) = read_global_config() {
        ProxyService::apply_proxy_from_config(&config);
    }
}
