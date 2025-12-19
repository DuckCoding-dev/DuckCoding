pub mod balance_commands;
pub mod config_commands;
pub mod error; // 错误处理统一模块
pub mod log_commands;
pub mod onboarding;
pub mod profile_commands; // Profile 管理命令（v2.0）
pub mod proxy_commands;
pub mod session_commands;
pub mod stats_commands;
pub mod tool_commands;
pub mod tool_management;
pub mod types;
pub mod update_commands;
pub mod watcher_commands;
pub mod window_commands;

// 重新导出所有命令函数
pub use balance_commands::*;
pub use config_commands::*;
pub use log_commands::*;
pub use onboarding::*;
pub use profile_commands::*; // Profile 管理命令（v2.0）
pub use proxy_commands::*;
pub use session_commands::*;
pub use stats_commands::*;
pub use tool_commands::*;
pub use tool_management::*;
pub use update_commands::*;
pub use watcher_commands::*;
pub use window_commands::*;
