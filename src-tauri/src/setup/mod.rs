// 托盘菜单和窗口管理
pub mod tray;

// 启动初始化逻辑
pub mod initialization;

// macOS 应用菜单栏
#[cfg(target_os = "macos")]
pub mod menu;

// 重新导出常用函数供 main.rs 使用
pub use initialization::initialize_app;
pub use tray::focus_main_window;
