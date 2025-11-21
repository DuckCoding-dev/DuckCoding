//! 应用事件常量定义
//!
//! 用于统一管理应用内部事件名称，避免拼写错误

use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};

/// 关闭确认事件
///
/// 当用户尝试关闭窗口时，发送此事件到前端，
/// 前端显示对话框让用户选择"最小化到托盘"或"直接退出"
pub const CLOSE_CONFIRM_EVENT: &str = "duckcoding://request-close-action";

/// 单实例事件
///
/// 当用户尝试第二次启动应用时，发送此事件到已存在的实例，
/// 携带新的启动参数和工作目录
pub const SINGLE_INSTANCE_EVENT: &str = "single-instance";

/// 单实例事件负载
///
/// 包含第二次启动时的参数信息
#[derive(Clone, Serialize)]
pub struct SingleInstancePayload {
    /// 命令行参数
    pub args: Vec<String>,
    /// 工作目录
    pub cwd: String,
}

/// 发送关闭确认事件到前端
///
/// # 参数
/// - `app`: Tauri 应用句柄
///
/// # 返回
/// - 成功或错误
///
/// # 示例
/// ```rust,no_run
/// use duckcoding::emit_close_confirm;
/// # use tauri::{AppHandle, Runtime};
/// # fn demo<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
/// emit_close_confirm(app)?;
/// # Ok(())
/// # }
/// ```
pub fn emit_close_confirm<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    tracing::debug!("发送关闭确认事件到前端");
    app.emit(CLOSE_CONFIRM_EVENT, ())
}

/// 发送单实例事件
///
/// # 参数
/// - `app`: Tauri 应用句柄
/// - `payload`: 单实例事件负载
///
/// # 返回
/// - 成功或错误
pub fn emit_single_instance<R: Runtime>(
    app: &AppHandle<R>,
    payload: SingleInstancePayload,
) -> tauri::Result<()> {
    tracing::info!(
        args = ?payload.args,
        cwd = %payload.cwd,
        "发送单实例事件"
    );
    app.emit(SINGLE_INSTANCE_EVENT, payload)
}
