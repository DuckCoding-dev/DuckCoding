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
            .map_err(|e| format!("Failed to create config directory: {e}"))?;
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
        fs::read_to_string(&config_path).map_err(|e| format!("Failed to read config: {e}"))?;
    let mut config: GlobalConfig =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse config: {e}"))?;

    // 自动迁移旧的透明代理配置到新结构
    migrate_proxy_config(&mut config)?;

    // 自动迁移全局会话配置到工具级
    migrate_session_config(&mut config)?;

    Ok(Some(config))
}

/// 迁移旧的透明代理配置到新的多工具架构
///
/// 将旧的 `transparent_proxy_*` 字段迁移到 `proxy_configs["claude-code"]`
/// 迁移完成后清除旧字段并保存配置到磁盘
fn migrate_proxy_config(config: &mut GlobalConfig) -> Result<(), String> {
    // 检查是否需要迁移（旧字段存在且新结构中 claude-code 配置为空）
    if config.transparent_proxy_enabled
        || config.transparent_proxy_api_key.is_some()
        || config.transparent_proxy_real_api_key.is_some()
    {
        // 获取或创建 claude-code 的配置
        let claude_config = config
            .proxy_configs
            .entry("claude-code".to_string())
            .or_default();

        // 只有当新配置还是默认值时才迁移
        if !claude_config.enabled && claude_config.real_api_key.is_none() {
            println!("🔄 检测到旧的透明代理配置，正在迁移到新架构...");

            claude_config.enabled = config.transparent_proxy_enabled;
            claude_config.port = config.transparent_proxy_port;
            claude_config.local_api_key = config.transparent_proxy_api_key.clone();
            claude_config.real_api_key = config.transparent_proxy_real_api_key.clone();
            claude_config.real_base_url = config.transparent_proxy_real_base_url.clone();
            claude_config.allow_public = config.transparent_proxy_allow_public;

            println!("✅ 配置迁移完成，Claude Code 代理配置已更新");
        }

        // 清除旧字段以防止重复迁移
        config.transparent_proxy_enabled = false;
        config.transparent_proxy_api_key = None;
        config.transparent_proxy_real_api_key = None;
        config.transparent_proxy_real_base_url = None;

        // 保存迁移后的配置到磁盘
        let config_path = global_config_path()?;
        let json = serde_json::to_string_pretty(config)
            .map_err(|e| format!("Failed to serialize config: {e}"))?;
        fs::write(&config_path, json).map_err(|e| format!("Failed to write config: {e}"))?;

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

        println!("✅ 迁移配置已保存到磁盘");
    }

    Ok(())
}

/// 迁移全局 session_endpoint_config_enabled 到各工具的配置中
///
/// 如果全局开关已启用，则将其值迁移到每个工具的 session_endpoint_config_enabled 字段
fn migrate_session_config(config: &mut GlobalConfig) -> Result<(), String> {
    // 仅在全局开关为 true 时进行迁移
    if config.session_endpoint_config_enabled {
        let mut migrated = false;

        for tool_config in config.proxy_configs.values_mut() {
            // 仅迁移尚未设置的工具
            if !tool_config.session_endpoint_config_enabled {
                tool_config.session_endpoint_config_enabled = true;
                migrated = true;
            }
        }

        if migrated {
            println!("🔄 正在迁移全局会话端点配置到工具级...");

            // 保存迁移后的配置到磁盘
            let config_path = global_config_path()?;
            let json = serde_json::to_string_pretty(config)
                .map_err(|e| format!("Failed to serialize config: {e}"))?;
            fs::write(&config_path, json).map_err(|e| format!("Failed to write config: {e}"))?;

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

            println!("✅ 会话端点配置迁移完成");
        }
    }

    Ok(())
}

/// 写入全局配置，同时设置权限并更新当前进程代理
pub fn write_global_config(config: &GlobalConfig) -> Result<(), String> {
    let config_path = global_config_path()?;
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {e}"))?;

    fs::write(&config_path, json).map_err(|e| format!("Failed to write config: {e}"))?;

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
