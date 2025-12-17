//! 代理工具模块
//!
//! 包含通用的工具函数和类型定义

pub mod body;
pub mod error_responses;
pub mod loop_detector;

// 重新导出常用类型
pub use body::{box_body, BoxBody};
