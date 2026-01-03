pub mod balance_commands;
pub mod config_commands;
pub mod dashboard_commands; // 仪表板状态管理命令
pub mod error; // 错误处理统一模块
pub mod log_commands;
pub mod onboarding;
pub mod profile_commands; // Profile 管理命令（v2.0）
pub mod provider_commands; // 供应商管理命令（v1.5.0）
pub mod proxy_commands;
pub mod session_commands;
pub mod startup_commands; // 开机自启动管理命令
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
pub use dashboard_commands::*; // 仪表板状态管理命令
pub use log_commands::*;
pub use onboarding::*;
pub use profile_commands::*; // Profile 管理命令（v2.0）
pub use provider_commands::*; // 供应商管理命令（v1.5.0）
pub use proxy_commands::*;
pub use session_commands::*;
pub use startup_commands::*; // 开机自启动管理命令
pub use stats_commands::*;
pub use tool_commands::*;
pub use tool_management::*;
pub use update_commands::*;
pub use watcher_commands::*;
pub use window_commands::*;
