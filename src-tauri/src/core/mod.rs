pub mod error;
pub mod http;
pub mod logger;
pub mod log_utils;

// 导出核心类型
pub use error::{AppError, AppResult, ErrorContext};
pub use http::{build_http_client, get_global_client};
pub use logger::{LogConfig, LogLevel, init_logger, set_log_level};
pub use log_utils::{Timer, LogContext};

// 重新导出 tracing 核心功能
pub use tracing::{trace, debug, info, warn, error, instrument, span, Level};
