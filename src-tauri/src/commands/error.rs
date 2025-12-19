//! Commands 层错误处理统一模块
//!
//! ## 使用指南
//!
//! ### 当前状态
//!
//! - Commands 层使用 `Result<T, String>` 作为返回类型（Tauri 要求）
//! - Services 层使用 `anyhow::Result<T>` 作为返回类型
//! - Core 层提供完善的 `AppError` 枚举和 `AppResult<T>` 类型别名
//!
//! ### 最佳实践（渐进式迁移）
//!
//! #### 方案 A：保持现状（推荐新手）
//!
//! ```rust
//! #[tauri::command]
//! pub async fn my_command() -> Result<Data, String> {
//!     service::do_something().map_err(|e| e.to_string())
//! }
//! ```
//!
//! **优点**：简单直接，无需修改现有代码
//! **缺点**：错误信息丢失上下文
//!
//! #### 方案 B：使用 AppError（推荐专家）
//!
//! ```rust
//! use super::error::AppResult;
//!
//! #[tauri::command]
//! pub async fn my_command() -> AppResult<Data> {
//!     let data = service::do_something()?;  // anyhow::Result 自动转换
//!     Ok(data)
//! }
//! ```
//!
//! **优点**：保留完整错误链，结构化错误信息
//! **缺点**：需要确保所有错误类型实现 `Into<AppError>`
//!
//! #### 方案 C：混合使用（推荐过渡期）
//!
//! ```rust
//! use super::error::AppError;
//!
//! #[tauri::command]
//! pub async fn my_command(id: String) -> Result<Data, String> {
//!     let tool = Tool::by_id(&id)
//!         .ok_or_else(|| AppError::ToolNotFound { tool: id.clone() })?;
//!
//!     service::process(&tool)
//!         .map_err(|e| e.to_string())
//! }
//! ```
//!
//! **优点**：关键错误使用结构化类型，其他保持简单
//! **缺点**：代码风格不统一
//!
//! ### 错误类型速查
//!
//! - `AppError::ToolNotFound { tool }`  - 工具未找到
//! - `AppError::ConfigNotFound { path }` - 配置文件未找到
//! - `AppError::ProfileNotFound { profile }` - Profile 未找到
//! - `AppError::ValidationError { field, reason }` - 验证失败
//! - `AppError::Custom(String)` - 自定义错误
//!
//! ### 迁移计划
//!
//! 1. ✅ 创建 error.rs 模块（本文件）
//! 2. ⏳ 迁移高频命令（config/profile/tool）
//! 3. ⏳ 迁移中频命令（proxy/session）
//! 4. ⏳ 迁移低频命令（其他）
//!
//! ## 导出
//!
//! 重导出 core::error 中的类型供 commands 层使用

pub use ::duckcoding::core::error::{AppError, AppResult, ErrorContext};

/// 便捷函数：将 anyhow::Error 转换为 String（用于兼容旧代码）
pub fn anyhow_to_string(err: anyhow::Error) -> String {
    err.to_string()
}

/// 便捷函数：将 AppError 转换为 String（用于兼容旧代码）
pub fn app_error_to_string(err: AppError) -> String {
    err.to_string()
}
