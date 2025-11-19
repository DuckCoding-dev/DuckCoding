// 服务层模块
//
// 重组后的目录结构：
// - config: 配置管理（待拆分优化）
// - tool: 工具安装、版本检查、下载
// - proxy: 代理配置和透明代理
// - update: 应用自身更新

pub mod config;
pub mod tool;
pub mod proxy;
pub mod update;

// 重新导出服务
pub use config::*;
pub use tool::*;
pub use proxy::*;
pub use update::*;
