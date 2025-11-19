pub mod window;
pub mod tray;
pub mod events;

// 导出窗口管理函数
pub use window::{focus_main_window, restore_window_state, hide_window_to_tray};

// 导出托盘管理函数
pub use tray::create_tray_menu;

// 导出事件常量和函数
pub use events::{
    CLOSE_CONFIRM_EVENT,
    SINGLE_INSTANCE_EVENT,
    SingleInstancePayload,
    emit_close_confirm,
    emit_single_instance,
};
