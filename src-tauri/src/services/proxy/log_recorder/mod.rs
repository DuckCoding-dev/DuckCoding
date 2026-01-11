// 日志记录模块 - 统一的请求日志记录架构
//
// 职责：
// - 提取请求上下文
// - 解析响应数据（SSE/JSON）
// - 提取 Token 统计
// - 计算成本
// - 记录到数据库

mod context;
mod parser;
mod recorder;

pub use context::RequestLogContext;
pub use parser::{ParsedResponse, ResponseParser};
pub use recorder::LogRecorder;
