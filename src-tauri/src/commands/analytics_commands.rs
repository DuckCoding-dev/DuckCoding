//! Token统计分析相关的Tauri命令

use anyhow::Result;
use duckcoding::services::token_stats::{
    CostGroupBy, CostSummaryQuery, TimeGranularity, TokenStatsAnalytics, TrendDataPoint, TrendQuery,
};
use duckcoding::utils::config_dir;
use serde::{Deserialize, Serialize};

/// 按模型分组的成本统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCostStat {
    /// 模型名称
    pub model: String,
    /// 总成本（USD）
    pub total_cost: f64,
    /// 请求数
    pub request_count: i64,
}

/// 按配置分组的成本统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigCostStat {
    /// 配置名称
    pub config_name: String,
    /// 总成本（USD）
    pub total_cost: f64,
    /// 请求数
    pub request_count: i64,
}

/// 成本汇总数据（前端期望的格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSummary {
    /// 总成本（USD）
    pub total_cost: f64,
    /// 总请求数
    pub total_requests: i64,
    /// 成功请求数
    pub successful_requests: i64,
    /// 失败请求数
    pub failed_requests: i64,
    /// 平均响应时间（毫秒）
    pub avg_response_time: Option<f64>,
    /// 按模型分组的成本
    pub cost_by_model: Vec<ModelCostStat>,
    /// 按配置分组的成本
    pub cost_by_config: Vec<ConfigCostStat>,
    /// 按天的成本趋势
    pub daily_costs: Vec<DailyCost>,
}

/// 按天的成本统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyCost {
    /// 日期（时间戳毫秒）
    pub date: i64,
    /// 总成本（USD）
    pub cost: f64,
}

/// 查询趋势数据
///
/// # 参数
/// - `query`: 趋势查询参数
///
/// # 返回
/// - `Ok(Vec<TrendDataPoint>)`: 按时间排序的趋势数据点列表
/// - `Err`: 查询失败
#[tauri::command]
pub async fn query_token_trends(query: TrendQuery) -> Result<Vec<TrendDataPoint>, String> {
    let db_path = config_dir()
        .map_err(|e| format!("Failed to get config dir: {}", e))?
        .join("token_stats.db");

    let analytics = TokenStatsAnalytics::new(db_path.clone());

    analytics
        .query_trends(&query)
        .map_err(|e| format!("Failed to query trends: {}", e))
}

/// 查询成本汇总数据
///
/// # 参数
/// - `start_time`: 开始时间戳（毫秒）
/// - `end_time`: 结束时间戳（毫秒）
/// - `tool_type`: 工具类型过滤（可选）
/// - `session_id`: 会话 ID 过滤（可选）
///
/// # 返回
/// - `Ok(CostSummary)`: 成本汇总数据
/// - `Err`: 查询失败
#[tauri::command]
pub async fn query_cost_summary(
    start_time: i64,
    end_time: i64,
    tool_type: Option<String>,
    session_id: Option<String>,
) -> Result<CostSummary, String> {
    let db_path = config_dir()
        .map_err(|e| format!("Failed to get config dir: {}", e))?
        .join("token_stats.db");

    let analytics = TokenStatsAnalytics::new(db_path.clone());

    // 构建基础查询参数
    let base_query = CostSummaryQuery {
        start_time: Some(start_time),
        end_time: Some(end_time),
        tool_type: tool_type.clone(),
        session_id: session_id.clone(),
        group_by: CostGroupBy::Model, // 默认分组，实际查询时会覆盖
    };

    // 1. 查询按模型分组的成本
    let model_query = CostSummaryQuery {
        group_by: CostGroupBy::Model,
        ..base_query.clone()
    };
    let model_summaries = analytics
        .query_cost_summary(&model_query)
        .map_err(|e| format!("Failed to query cost by model: {}", e))?;

    // 2. 查询按配置分组的成本
    let config_query = CostSummaryQuery {
        group_by: CostGroupBy::Config,
        ..base_query.clone()
    };
    let config_summaries = analytics
        .query_cost_summary(&config_query)
        .map_err(|e| format!("Failed to query cost by config: {}", e))?;

    // 3. 查询按天的成本趋势
    let trend_query = TrendQuery {
        start_time: Some(start_time),
        end_time: Some(end_time),
        tool_type: tool_type.clone(),
        session_id: session_id.clone(),
        granularity: TimeGranularity::Day,
        ..Default::default()
    };
    let daily_trends = analytics
        .query_trends(&trend_query)
        .map_err(|e| format!("Failed to query daily trends: {}", e))?;

    // 4. 计算总计指标（通过聚合所有数据）
    let total_cost: f64 = model_summaries.iter().map(|s| s.total_cost).sum();
    let total_requests: i64 = model_summaries.iter().map(|s| s.request_count).sum();

    // 5. 查询成功和失败请求数（需要额外查询）
    use duckcoding::data::DataManager;
    let manager = DataManager::global()
        .sqlite(&db_path)
        .map_err(|e| format!("Failed to get SQLite manager: {}", e))?;

    // 构建 WHERE 子句
    let mut where_clauses = vec!["timestamp >= ?1", "timestamp <= ?2"];
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![
        Box::new(start_time) as Box<dyn rusqlite::ToSql>,
        Box::new(end_time),
    ];

    if let Some(ref tt) = tool_type {
        where_clauses.push("tool_type = ?");
        params.push(Box::new(tt.clone()));
    }

    if let Some(ref sid) = session_id {
        where_clauses.push("session_id = ?");
        params.push(Box::new(sid.clone()));
    }

    let where_clause = where_clauses.join(" AND ");

    let sql = format!(
        "SELECT
            COUNT(*) as total,
            COALESCE(SUM(CASE WHEN request_status = 'success' THEN 1 ELSE 0 END), 0) as successful,
            COALESCE(SUM(CASE WHEN request_status = 'failed' THEN 1 ELSE 0 END), 0) as failed,
            AVG(response_time_ms) as avg_response_time
        FROM token_logs
        WHERE {}",
        where_clause
    );

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let (successful_requests, failed_requests, avg_response_time) = manager
        .transaction(|tx| {
            let mut stmt = tx
                .prepare(&sql)
                .map_err(duckcoding::data::DataError::Database)?;

            let result = stmt
                .query_row(param_refs.as_slice(), |row| {
                    Ok((
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, Option<f64>>(3)?,
                    ))
                })
                .map_err(duckcoding::data::DataError::Database)?;

            Ok(result)
        })
        .map_err(|e| format!("Failed to query request stats: {}", e))?;

    // 6. 构建返回结果
    Ok(CostSummary {
        total_cost,
        total_requests,
        successful_requests,
        failed_requests,
        avg_response_time,
        cost_by_model: model_summaries
            .into_iter()
            .map(|s| ModelCostStat {
                model: s.group_name,
                total_cost: s.total_cost,
                request_count: s.request_count,
            })
            .collect(),
        cost_by_config: config_summaries
            .into_iter()
            .map(|s| ConfigCostStat {
                config_name: s.group_name,
                total_cost: s.total_cost,
                request_count: s.request_count,
            })
            .collect(),
        daily_costs: daily_trends
            .into_iter()
            .map(|d| DailyCost {
                date: d.timestamp,
                cost: d.total_cost,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use duckcoding::models::token_stats::TokenLog;
    use duckcoding::services::token_stats::db::TokenStatsDb;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_query_token_trends_command() {
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
                "claude-code".to_string(),
                base_time - (i * 3600 * 1000), // 每小时一条
                "127.0.0.1".to_string(),
                "test_session".to_string(),
                "default".to_string(),
                "claude-sonnet-4-5-20250929".to_string(),
                Some(format!("msg_{}", i)),
                100,
                50,
                10,
                0, // cache_creation_1h_tokens
                20,
                0, // reasoning_tokens
                "success".to_string(),
                "json".to_string(),
                None,
                None,
                Some(100),
                Some(0.001),
                Some(0.002),
                Some(0.0001),
                Some(0.0002),
                None, // reasoning_price
                0.0033,
                Some("test_template".to_string()),
            );
            db.insert_log(&log).unwrap();
        }

        // 创建查询
        let query = TrendQuery {
            tool_type: Some("claude-code".to_string()),
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

        // 插入测试数据（多个模型和配置，使用固定时间）
        let base_time = chrono::Utc
            .with_ymd_and_hms(2026, 1, 10, 12, 0, 0)
            .unwrap()
            .timestamp_millis();

        let models = ["claude-sonnet-4-5-20250929", "claude-3-opus-20240229"];
        let configs = ["default", "custom"];

        for (i, model) in models.iter().enumerate() {
            for (j, config) in configs.iter().enumerate() {
                for k in 0..3 {
                    let log = TokenLog::new(
                        "claude-code".to_string(),
                        base_time - (k * 1000),
                        "127.0.0.1".to_string(),
                        format!("session_{}_{}", i, j),
                        config.to_string(),
                        model.to_string(),
                        Some(format!("msg_{}_{}_{}", i, j, k)),
                        100,
                        50,
                        10,
                        0, // cache_creation_1h_tokens
                        20,
                        0, // reasoning_tokens
                        "success".to_string(),
                        "json".to_string(),
                        None,
                        None,
                        Some(100),
                        Some(0.001),
                        Some(0.002),
                        Some(0.0001),
                        Some(0.0002),
                        None, // reasoning_price
                        0.0033,
                        Some("test_template".to_string()),
                    );
                    db.insert_log(&log).unwrap();
                }
            }
        }

        // 执行查询
        let analytics = TokenStatsAnalytics::new(db_path);

        // 按模型分组
        let model_query = CostSummaryQuery {
            tool_type: Some("claude-code".to_string()),
            group_by: CostGroupBy::Model,
            ..Default::default()
        };
        let model_summaries = analytics.query_cost_summary(&model_query).unwrap();

        // 验证结果
        assert_eq!(model_summaries.len(), 2); // 2个模型
        for summary in &model_summaries {
            assert_eq!(summary.request_count, 6); // 每个模型6条记录（2个配置 × 3条）
            assert!(summary.total_cost > 0.0);
        }

        // 按配置分组
        let config_query = CostSummaryQuery {
            tool_type: Some("claude-code".to_string()),
            group_by: CostGroupBy::Config,
            ..Default::default()
        };
        let config_summaries = analytics.query_cost_summary(&config_query).unwrap();

        // 验证结果
        assert_eq!(config_summaries.len(), 2); // 2个配置
        for summary in &config_summaries {
            assert_eq!(summary.request_count, 6); // 每个配置6条记录（2个模型 × 3条）
            assert!(summary.total_cost > 0.0);
        }
    }
}
