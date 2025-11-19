use tracing::Level;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer, Registry,
};
use tracing_appender::{non_blocking, rolling};
use std::path::PathBuf;

/// 日志级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// 追踪级别（最详细）
    Trace,
    /// 调试级别
    Debug,
    /// 信息级别
    Info,
    /// 警告级别
    Warn,
    /// 错误级别
    Error,
}

impl LogLevel {
    /// 转换为 tracing::Level
    pub fn to_tracing_level(&self) -> Level {
        match self {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }

    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "trace" => Some(LogLevel::Trace),
            "debug" => Some(LogLevel::Debug),
            "info" => Some(LogLevel::Info),
            "warn" => Some(LogLevel::Warn),
            "error" => Some(LogLevel::Error),
            _ => None,
        }
    }
}

/// 日志配置
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// 日志级别
    pub level: LogLevel,

    /// 是否启用控制台输出
    pub console_enabled: bool,

    /// 是否启用文件输出
    pub file_enabled: bool,

    /// 日志文件目录
    pub log_dir: Option<PathBuf>,

    /// 日志文件名前缀
    pub log_file_prefix: String,

    /// 是否启用 JSON 格式（用于结构化日志）
    pub json_format: bool,

    /// 是否显示目标模块
    pub show_target: bool,

    /// 是否显示线程 ID
    pub show_thread_ids: bool,

    /// 是否显示时间戳
    pub show_time: bool,

    /// 是否启用 span 追踪
    pub enable_spans: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: if cfg!(debug_assertions) {
                LogLevel::Debug
            } else {
                LogLevel::Info
            },
            console_enabled: true,
            file_enabled: true,
            log_dir: None, // 默认使用应用数据目录
            log_file_prefix: "duckcoding".to_string(),
            json_format: false,
            show_target: cfg!(debug_assertions),
            show_thread_ids: false,
            show_time: true,
            enable_spans: cfg!(debug_assertions),
        }
    }
}

/// 初始化日志系统
///
/// # 示例
/// ```
/// use duckcoding::core::logger::{LogConfig, init_logger};
///
/// let config = LogConfig::default();
/// init_logger(config).expect("初始化日志系统失败");
/// ```
pub fn init_logger(config: LogConfig) -> anyhow::Result<()> {
    // 构建环境过滤器
    let env_filter = build_env_filter(&config);

    // 根据配置选择不同的初始化路径
    if config.file_enabled {
        // 同时启用控制台和文件输出
        // 确定日志目录
        let log_dir = match &config.log_dir {
            Some(dir) => dir.clone(),
            None => {
                // 使用应用数据目录
                let app_dir = dirs::data_local_dir()
                    .ok_or_else(|| anyhow::anyhow!("无法获取应用数据目录"))?
                    .join("DuckCoding")
                    .join("logs");

                std::fs::create_dir_all(&app_dir)?;
                app_dir
            }
        };

        // 创建滚动日志文件（每天一个文件）
        let file_appender = rolling::daily(log_dir, &config.log_file_prefix);
        let (non_blocking, guard) = non_blocking(file_appender);

        // 存储 guard 到全局静态变量（防止被 drop）
        Box::leak(Box::new(guard));

        // 构建文件输出层
        let file_layer = if config.json_format {
            // JSON 格式（用于日志分析工具）
            fmt::layer()
                .json()
                .with_writer(non_blocking.clone())
                .with_target(true)
                .with_thread_ids(true)
                .with_ansi(false) // 文件输出禁用颜色
                .boxed()
        } else {
            // 普通格式
            fmt::layer()
                .with_writer(non_blocking)
                .with_target(config.show_target)
                .with_thread_ids(config.show_thread_ids)
                .with_ansi(false)
                .boxed()
        };

        Registry::default()
            .with(env_filter)
            .with(fmt::layer()
                .with_target(config.show_target)
                .with_thread_ids(config.show_thread_ids)
                .with_ansi(true)
                .with_span_events(if config.enable_spans {
                    FmtSpan::CLOSE
                } else {
                    FmtSpan::NONE
                }))
            .with(file_layer)
            .init();
    } else if config.console_enabled {
        // 仅控制台输出
        Registry::default()
            .with(env_filter)
            .with(fmt::layer()
                .with_target(config.show_target)
                .with_thread_ids(config.show_thread_ids)
                .with_ansi(true)
                .with_span_events(if config.enable_spans {
                    FmtSpan::CLOSE
                } else {
                    FmtSpan::NONE
                }))
            .init();
    } else {
        // 都不启用（极少情况）
        Registry::default()
            .with(env_filter)
            .init();
    }

    tracing::info!(
        console_enabled = config.console_enabled,
        file_enabled = config.file_enabled,
        level = ?config.level,
        "日志系统初始化成功"
    );

    Ok(())
}

/// 构建环境过滤器
fn build_env_filter(config: &LogConfig) -> EnvFilter {
    // 优先从环境变量读取（支持运行时调整）
    // 格式：RUST_LOG=debug 或 RUST_LOG=duckcoding=trace,reqwest=warn
    EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            // 默认配置：应用代码使用指定级别，第三方库使用 WARN
            EnvFilter::new(format!(
                "duckcoding={},hyper=warn,reqwest=warn,h2=warn,tokio=warn",
                config.level.to_tracing_level()
            ))
        })
}

/// 运行时调整日志级别
///
/// # 示例
/// ```
/// use duckcoding::core::logger::{LogLevel, set_log_level};
///
/// set_log_level(LogLevel::Debug);
/// ```
pub fn set_log_level(level: LogLevel) {
    // 注意：这需要重新初始化订阅者，或者使用 reload layer
    // 这里提供一个简化版本，通过环境变量实现
    std::env::set_var("RUST_LOG", format!("duckcoding={}", level.to_tracing_level()));
    tracing::warn!("日志级别已调整为 {:?}，需要重启应用生效", level);
}
