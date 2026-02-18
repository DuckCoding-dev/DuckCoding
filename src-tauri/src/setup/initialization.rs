use duckcoding::core::init_logger;
use duckcoding::services::profile_manager::ProfileManager;
use duckcoding::services::proxy_config_manager::ProxyConfigManager;
use duckcoding::utils::config::read_global_config;
use duckcoding::{ProxyManager, ToolRegistry};
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

/// 启动初始化上下文
///
/// 包含应用启动所需的核心服务实例
pub struct InitializationContext {
    pub proxy_manager: Arc<ProxyManager>,
    pub tool_registry: Arc<TokioMutex<ToolRegistry>>,
    pub profile_manager: Arc<tokio::sync::RwLock<ProfileManager>>,
}

/// 初始化日志系统
///
/// 从全局配置读取日志配置，失败则使用默认配置
fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
    let log_config = read_global_config()
        .ok()
        .flatten()
        .map(|cfg| cfg.log_config)
        .unwrap_or_default();

    if let Err(e) = init_logger(&log_config) {
        // 日志系统初始化失败时使用 eprintln!（因为 tracing 还不可用）
        eprintln!("WARNING: Failed to initialize logging system: {}", e);
        // 继续运行，但日志功能将不可用
    }

    tracing::info!("DuckCoding 应用启动");
    Ok(())
}

/// 初始化内置 Profile（用于透明代理配置切换）
///
/// 为每个启用且配置完整的代理工具创建内置 Profile
fn initialize_proxy_profiles() -> Result<(), Box<dyn std::error::Error>> {
    let proxy_mgr = ProxyConfigManager::new()?;
    let profile_mgr = ProfileManager::new()?;

    for tool_id in &["claude-code", "codex", "gemini-cli"] {
        if let Ok(Some(config)) = proxy_mgr.get_config(tool_id) {
            // 检查配置完整性并解构
            if let (true, Some(proxy_key), Some(_real_key), Some(_real_url)) = (
                config.enabled,
                &config.local_api_key,
                &config.real_api_key,
                &config.real_base_url,
            ) {
                let proxy_profile_name = format!("dc_proxy_{}", tool_id.replace("-", "_"));
                let proxy_endpoint = format!("http://127.0.0.1:{}", config.port);

                let result = match *tool_id {
                    "claude-code" => profile_mgr.save_claude_profile_internal(
                        &proxy_profile_name,
                        proxy_key.clone(),
                        proxy_endpoint,
                    ),
                    "codex" => profile_mgr.save_codex_profile_internal(
                        &proxy_profile_name,
                        proxy_key.clone(),
                        proxy_endpoint,
                        Some("responses".to_string()),
                    ),
                    "gemini-cli" => profile_mgr.save_gemini_profile_internal(
                        &proxy_profile_name,
                        proxy_key.clone(),
                        proxy_endpoint,
                        None, // 不设置 model，保留用户原有配置
                    ),
                    _ => continue,
                };

                if let Err(e) = result {
                    tracing::warn!(
                        tool_id = tool_id,
                        error = ?e,
                        "初始化内置 Profile 失败"
                    );
                } else {
                    tracing::debug!(
                        tool_id = tool_id,
                        profile = %proxy_profile_name,
                        "已初始化内置 Profile"
                    );
                }
            }
        }
    }

    Ok(())
}

/// 执行数据迁移（版本驱动）
async fn run_migrations() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("执行数据迁移检查");
    let migration_manager = duckcoding::create_migration_manager();
    match migration_manager.run_all().await {
        Ok(results) => {
            if !results.is_empty() {
                tracing::info!("迁移执行完成：{} 个迁移", results.len());
                for result in results {
                    if result.success {
                        tracing::info!("✅ {}: {}", result.migration_id, result.message);
                    } else {
                        tracing::error!("❌ {}: {}", result.migration_id, result.message);
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!("迁移执行失败: {}", e);
            return Err(e.into());
        }
    }
    Ok(())
}

/// 标记未处理的配置变更日志为已过期
fn mark_expired_change_logs() -> Result<(), Box<dyn std::error::Error>> {
    use duckcoding::data::changelogs::ChangeLogStore;

    match ChangeLogStore::load() {
        Ok(mut store) => {
            store.mark_pending_as_expired();
            if let Err(e) = store.save() {
                tracing::warn!(error = ?e, "保存过期日志失败");
            } else {
                tracing::debug!("已标记未处理的配置变更日志为已过期");
            }
        }
        Err(e) => {
            tracing::warn!(error = ?e, "加载配置变更日志失败");
        }
    }

    Ok(())
}

/// 自动启动配置的代理
async fn auto_start_proxies(
    proxy_manager: &Arc<ProxyManager>,
    _profile_manager: &Arc<tokio::sync::RwLock<ProfileManager>>,
) {
    duckcoding::auto_start_proxies(proxy_manager).await;
}

/// 执行所有启动初始化任务
///
/// 按顺序执行：日志 → Profile → 迁移 → 标记过期日志 → 工具注册表 → 代理管理器
pub async fn initialize_app() -> Result<InitializationContext, Box<dyn std::error::Error>> {
    // 1. 初始化日志
    init_logging()?;

    // 2. 初始化内置 Profile
    if let Err(e) = initialize_proxy_profiles() {
        tracing::warn!(error = ?e, "初始化内置 Profile 失败");
    }

    // 3. 执行数据迁移
    run_migrations().await?;

    // 4. 标记未处理的配置变更日志为已过期
    if let Err(e) = mark_expired_change_logs() {
        tracing::warn!(error = ?e, "标记过期日志失败");
    }

    // 5. 创建工具注册表
    let tool_registry = ToolRegistry::new().await.expect("无法创建工具注册表");

    // 6. 创建 ProfileManager 单例
    let profile_manager = Arc::new(tokio::sync::RwLock::new(
        ProfileManager::new().expect("初始化 ProfileManager 失败"),
    ));

    // 7. 创建代理管理器并异步启动自启动代理
    let proxy_manager = Arc::new(ProxyManager::new());
    let proxy_manager_for_auto_start = proxy_manager.clone();
    let profile_manager_for_auto_start = profile_manager.clone();
    tauri::async_runtime::spawn(async move {
        auto_start_proxies(
            &proxy_manager_for_auto_start,
            &profile_manager_for_auto_start,
        )
        .await;
    });

    // 8. 启动远程价格同步调度器
    tauri::async_runtime::spawn(async {
        duckcoding::services::pricing::remote_sync::start_sync_scheduler().await;
    });

    Ok(InitializationContext {
        proxy_manager,
        tool_registry: Arc::new(TokioMutex::new(tool_registry)),
        profile_manager,
    })
}
