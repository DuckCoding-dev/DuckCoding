//! Token统计分析相关的Tauri命令

use duckcoding::services::token_stats::{
    CostSummary, CostSummaryQuery, TokenStatsAnalytics, TrendDataPoint, TrendQuery,
};
use duckcoding::utils::config_dir;
use anyhow::Result;

/// 查询趋势数据
///
/// # 参数
/// - `query`: 趋势查询参数
///
/// # 返回
/// - `Ok(Vec<TrendDataPoint>)`: 按时间排序的趋势数据点列表
/// - `Err`: 查询失败
#[tauri::command]
pub async fn query_trends(query: TrendQuery) -> Result<Vec<TrendDataPoint>, String> {
    let db_path = config_dir()
        .map_err(|e| format!("Failed to get config dir: {}", e))?
        .join("token_stats.db");

    let analytics = TokenStatsAnalytics::new(db_path);

    analytics
        .query_trends(&query)
        .map_err(|e| format!("Failed to query trends: {}", e))
}

/// 查询成本摘要数据
///
/// # 参数
/// - `query`: 成本摘要查询参数
///
/// # 返回
/// - `Ok(Vec<CostSummary>)`: 按指定字段排序的成本摘要列表
/// - `Err`: 查询失败
#[tauri::command]
pub async fn query_cost_summary(query: CostSummaryQuery) -> Result<Vec<CostSummary>, String> {
    let db_path = config_dir()
        .map_err(|e| format!("Failed to get config dir: {}", e))?
        .join("token_stats.db");

    let analytics = TokenStatsAnalytics::new(db_path);

    analytics
        .query_cost_summary(&query)
        .map_err(|e| format!("Failed to query cost summary: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use duckcoding::models::token_stats::TokenLog;
    use duckcoding::services::token_stats::db::TokenStatsDb;
    use duckcoding::services::token_stats::{CostGroupBy, TimeGranularity};
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_query_trends_command() {
        // 创建临时数据库
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_trends.db");
        let db = TokenStatsDb::new(db_path.clone());
        db.init_table().unwrap();

        // 插入测试数据（使用固定时间避免跨日期边界）
        let base_time = chrono::Utc
            .with_ymd_and_hms(2026, 1, 10, 12, 0, 0)
            .unwrap()
            .timestamp_millis();

        for i in 0..10 {
            let log = TokenLog::new(
                "claude_code".to_string(),
                base_time - (i * 3600 * 1000), // 每小时一条
                "127.0.0.1".to_string(),
                "test_session".to_string(),
                "default".to_string(),
                "claude-3-5-sonnet-20241022".to_string(),
                Some(format!("msg_{}", i)),
                100,
                50,
                10,
                20,
                "success".to_string(),
                "json".to_string(),
                None,
                None,
                Some(100),
                Some(0.001),
                Some(0.002),
                Some(0.0001),
                Some(0.0002),
                0.0033,
                Some("test_template".to_string()),
            );
            db.insert_log(&log).unwrap();
        }

        // 创建查询
        let query = TrendQuery {
            tool_type: Some("claude_code".to_string()),
            granularity: TimeGranularity::Hour,
            ..Default::default()
        };

        // 执行查询（通过直接调用analytics而不是tauri命令）
        let analytics = TokenStatsAnalytics::new(db_path);
        let trends = analytics.query_trends(&query).unwrap();

        // 验证结果
        assert_eq!(trends.len(), 10);
        assert!(trends[0].input_tokens > 0);
        assert!(trends[0].total_cost > 0.0);
    }

    #[tokio::test]
    async fn test_query_cost_summary_command() {
        // 创建临时数据库
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_cost_summary.db");
        let db = TokenStatsDb::new(db_path.clone());
        db.init_table().unwrap();

        // 插入测试数据（多个会话，使用固定时间）
        let base_time = chrono::Utc
            .with_ymd_and_hms(2026, 1, 10, 12, 0, 0)
            .unwrap()
            .timestamp_millis();

        for session_idx in 0..3 {
            for i in 0..5 {
                let log = TokenLog::new(
                    "claude_code".to_string(),
                    base_time - (i * 1000),
                    "127.0.0.1".to_string(),
                    format!("session_{}", session_idx),
                    "default".to_string(),
                    "claude-3-5-sonnet-20241022".to_string(),
                    Some(format!("msg_{}_{}", session_idx, i)),
                    100,
                    50,
                    10,
                    20,
                    "success".to_string(),
                    "json".to_string(),
                    None,
                    None,
                    Some(100),
                    Some(0.001),
                    Some(0.002),
                    Some(0.0001),
                    Some(0.0002),
                    0.0033,
                    Some("test_template".to_string()),
                );
                db.insert_log(&log).unwrap();
            }
        }

        // 创建查询
        let query = CostSummaryQuery {
            tool_type: Some("claude_code".to_string()),
            group_by: CostGroupBy::Session,
            ..Default::default()
        };

        // 执行查询
        let analytics = TokenStatsAnalytics::new(db_path);
        let summaries = analytics.query_cost_summary(&query).unwrap();

        // 验证结果
        assert_eq!(summaries.len(), 3); // 3个会话
        for summary in &summaries {
            assert_eq!(summary.request_count, 5); // 每个会话5条记录
            assert!(summary.total_cost > 0.0);
        }
    }
}
