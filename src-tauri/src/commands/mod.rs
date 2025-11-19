pub mod types;
pub mod tool_commands;
pub mod config_commands;
pub mod stats_commands;
pub mod proxy_commands;
pub mod update_commands;
pub mod window_commands;


// 重新导出所有命令函数
pub use tool_commands::*;
pub use proxy_commands::*;
pub use config_commands::*;
pub use stats_commands::*;
pub use update_commands::*;
pub use window_commands::*;
