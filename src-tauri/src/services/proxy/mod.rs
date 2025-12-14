// 代理服务模块
//
// 包含代理配置、透明代理等功能

pub mod headers;
pub mod proxy_instance;
pub mod proxy_manager;
pub mod proxy_service;
pub mod transparent_proxy;

pub use headers::{create_request_processor, ProcessedRequest, RequestProcessor};
// 向后兼容的导出（已弃用）
#[allow(deprecated)]
pub use headers::create_headers_processor;
pub use proxy_instance::ProxyInstance;
pub use proxy_manager::ProxyManager;
pub use proxy_service::ProxyService;
pub use transparent_proxy::{ProxyConfig, TransparentProxyService};
