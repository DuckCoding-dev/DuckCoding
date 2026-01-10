//! Token统计服务模块
//!
//! 提供透明代理的Token数据统计和请求记录功能。

// TODO: analytics 模块尚未实现，暂时注释
// pub mod analytics;
pub mod db;
pub mod extractor;
pub mod manager;

#[cfg(test)]
mod cost_calculation_test;

// TODO: analytics 导出暂时注释
// pub use analytics::{
//     CostGroupBy, CostSummary, CostSummaryQuery, TimeGranularity, TokenStatsAnalytics,
//     TrendDataPoint, TrendQuery,
// };
pub use db::TokenStatsDb;
pub use extractor::{
    create_extractor, ClaudeTokenExtractor, MessageDeltaData, MessageStartData, ResponseTokenInfo,
    SseTokenData, TokenExtractor,
};
pub use manager::{shutdown_token_stats_manager, TokenStatsManager};
