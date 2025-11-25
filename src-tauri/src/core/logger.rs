use crate::models::config::{LogConfig, LogFormat, LogLevel, LogOutput};
use std::sync::OnceLock;
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    reload::{self, Handle},
    util::SubscriberInitExt,
    EnvFilter, Layer, Registry,
};

/// 全局日志级别 reload handle
static LOG_LEVEL_HANDLE: OnceLock<Handle<EnvFilter, Registry>> = OnceLock::new();

/// 初始化日志系统
///
/// 支持基于配置的日志输出，包括：
/// - 日志级别（trace/debug/info/warn/error）
/// - 输出格式（JSON/纯文本）
/// - 输出目标（控制台/文件/both）
/// - 文件路径（用于文件输出）
///
/// # 热重载支持
/// 日志级别可以通过 `update_log_level` 函数动态调整，无需重启应用。
/// 其他配置（格式、输出目标、文件路径）需要重启应用后生效。
///
/// # 示例
/// ```
/// use duckcoding::models::config::LogConfig;
/// use duckcoding::core::init_logger;
///
/// let config = LogConfig::default();
/// init_logger(&config).expect("初始化日志系统失败");
/// ```
pub fn init_logger(config: &LogConfig) -> anyhow::Result<()> {
    // 1. 创建可重载的过滤层
    let filter = create_env_filter(&config.level);
    let (filter_layer, reload_handle) = reload::Layer::new(filter);

    // 2. 保存 reload handle（用于后续动态调整级别）
    if LOG_LEVEL_HANDLE.set(reload_handle).is_err() {
        anyhow::bail!("日志系统已初始化，不能重复初始化");
    }

    // 3. 根据配置添加输出层并初始化
    match (&config.output, &config.format) {
        (LogOutput::Console, LogFormat::Text) => {
            Registry::default()
                .with(filter_layer)
                .with(create_console_text_layer())
                .init();
        }
        (LogOutput::Console, LogFormat::Json) => {
            Registry::default()
                .with(filter_layer)
                .with(create_console_json_layer())
                .init();
        }
        (LogOutput::File, LogFormat::Text) => {
            let file_layer = create_file_text_layer(config.file_path.as_deref())?;
            Registry::default()
                .with(filter_layer)
                .with(file_layer)
                .init();
        }
        (LogOutput::File, LogFormat::Json) => {
            let file_layer = create_file_json_layer(config.file_path.as_deref())?;
            Registry::default()
                .with(filter_layer)
                .with(file_layer)
                .init();
        }
        (LogOutput::Both, LogFormat::Text) => {
            let file_layer = create_file_text_layer(config.file_path.as_deref())?;
            Registry::default()
                .with(filter_layer)
                .with(create_console_text_layer())
                .with(file_layer)
                .init();
        }
        (LogOutput::Both, LogFormat::Json) => {
            let file_layer = create_file_json_layer(config.file_path.as_deref())?;
            Registry::default()
                .with(filter_layer)
                .with(create_console_json_layer())
                .with(file_layer)
                .init();
        }
    }

    tracing::info!(
        level = config.level.as_str(),
        format = ?config.format,
        output = ?config.output,
        file_path = ?config.file_path,
        "日志系统初始化完成"
    );

    Ok(())
}

/// 创建环境过滤器
fn create_env_filter(level: &LogLevel) -> EnvFilter {
    // 优先从环境变量读取（支持高级用户自定义）
    // 格式：RUST_LOG=debug 或 RUST_LOG=duckcoding=trace,reqwest=warn
    EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        // 默认配置：应用代码使用指定级别，第三方库使用 WARN
        EnvFilter::new(format!(
            "duckcoding={},hyper=warn,reqwest=warn,h2=warn,tokio=warn",
            level.as_str()
        ))
    })
}

/// 创建控制台文本格式输出层
fn create_console_text_layer<S>() -> Box<dyn Layer<S> + Send + Sync + 'static>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(cfg!(debug_assertions))
        .with_thread_ids(false)
        .with_ansi(true)
        .with_span_events(if cfg!(debug_assertions) {
            FmtSpan::CLOSE
        } else {
            FmtSpan::NONE
        })
        .boxed()
}

/// 创建控制台 JSON 格式输出层
fn create_console_json_layer<S>() -> Box<dyn Layer<S> + Send + Sync + 'static>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fmt::layer()
        .json()
        .with_writer(std::io::stdout)
        .with_target(cfg!(debug_assertions))
        .with_thread_ids(false)
        .with_ansi(true)
        .boxed()
}

/// 创建文件文本格式输出层
fn create_file_text_layer<S>(
    file_path: Option<&str>,
) -> anyhow::Result<Box<dyn Layer<S> + Send + Sync + 'static>>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    let log_dir = get_log_dir(file_path)?;
    let file_appender = rolling::daily(log_dir, "duckcoding");
    let (non_blocking, guard) = non_blocking(file_appender);

    // 存储 guard 到全局静态变量（防止被 drop）
    Box::leak(Box::new(guard));

    Ok(fmt::layer()
        .with_writer(non_blocking)
        .with_target(cfg!(debug_assertions))
        .with_thread_ids(false)
        .with_ansi(false)
        .boxed())
}

/// 创建文件 JSON 格式输出层
fn create_file_json_layer<S>(
    file_path: Option<&str>,
) -> anyhow::Result<Box<dyn Layer<S> + Send + Sync + 'static>>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    let log_dir = get_log_dir(file_path)?;
    let file_appender = rolling::daily(log_dir, "duckcoding");
    let (non_blocking, guard) = non_blocking(file_appender);

    // 存储 guard 到全局静态变量（防止被 drop）
    Box::leak(Box::new(guard));

    Ok(fmt::layer()
        .json()
        .with_writer(non_blocking)
        .with_target(true)
        .with_thread_ids(true)
        .with_ansi(false)
        .boxed())
}

/// 获取日志目录
fn get_log_dir(file_path: Option<&str>) -> anyhow::Result<std::path::PathBuf> {
    match file_path {
        Some(path) => Ok(std::path::PathBuf::from(path)),
        None => {
            // 使用用户主目录下的 .duckcoding/logs
            let app_dir = dirs::home_dir()
                .ok_or_else(|| anyhow::anyhow!("无法获取用户主目录"))?
                .join(".duckcoding")
                .join("logs");

            std::fs::create_dir_all(&app_dir)?;
            Ok(app_dir)
        }
    }
}

/// 动态更新日志级别（热重载）
///
/// 此函数支持在应用运行时动态调整日志级别，无需重启。
/// 仅限调整日志级别，格式和输出目标的变更仍需要重启应用。
///
/// # 示例
/// ```
/// use duckcoding::models::config::LogLevel;
/// use duckcoding::core::update_log_level;
///
/// update_log_level(LogLevel::Debug).expect("更新日志级别失败");
/// ```
pub fn update_log_level(new_level: LogLevel) -> anyhow::Result<()> {
    let handle = LOG_LEVEL_HANDLE
        .get()
        .ok_or_else(|| anyhow::anyhow!("日志系统未初始化"))?;

    let new_filter = create_env_filter(&new_level);
    handle
        .reload(new_filter)
        .map_err(|e| anyhow::anyhow!("重载日志级别失败: {}", e))?;

    tracing::info!(new_level = new_level.as_str(), "日志级别已动态更新");
    Ok(())
}

/// 运行时调整日志级别（兼容旧接口）
///
/// # 弃用提示
/// 此函数已弃用，请使用 `update_log_level` 代替。
/// 旧版本通过环境变量调整，需要重启应用生效。
/// 新版本使用 reload 机制，支持动态热重载。
#[deprecated(
    since = "1.3.0",
    note = "请使用 update_log_level 代替，支持动态热重载"
)]
#[allow(dead_code)]
pub fn set_log_level(level: LogLevel) {
    if let Err(e) = update_log_level(level) {
        tracing::error!(error = ?e, "更新日志级别失败");
    }
}
