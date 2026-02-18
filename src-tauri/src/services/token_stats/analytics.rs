//! Token 统计分析模块
//!
//! 提供趋势分析和成本汇总查询功能

use crate::data::DataManager;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 时间粒度
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimeGranularity {
    /// 15分钟粒度
    FifteenMinutes,
    /// 30分钟粒度
    ThirtyMinutes,
    /// 小时粒度
    Hour,
    /// 12小时粒度
    TwelveHours,
    /// 天粒度
    #[default]
    Day,
}

/// 趋势查询参数
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrendQuery {
    /// 开始时间戳（毫秒）
    pub start_time: Option<i64>,
    /// 结束时间戳（毫秒）
    pub end_time: Option<i64>,
    /// 工具类型过滤
    pub tool_type: Option<String>,
    /// 模型过滤
    pub model: Option<String>,
    /// 配置名称过滤
    pub config_name: Option<String>,
    /// 会话 ID 过滤
    pub session_id: Option<String>,
    /// 时间粒度
    pub granularity: TimeGranularity,
}

/// 趋势数据点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendDataPoint {
    /// 时间戳（毫秒）
    pub timestamp: i64,
    /// 输入 Token 总数
    pub input_tokens: i64,
    /// 输出 Token 总数
    pub output_tokens: i64,
    /// 缓存写入 Token 总数
    pub cache_creation_tokens: i64,
    /// 缓存读取 Token 总数
    pub cache_read_tokens: i64,
    /// 总成本（USD）
    pub total_cost: f64,
    /// 输入部分成本（USD）
    pub input_price: f64,
    /// 输出部分成本（USD）
    pub output_price: f64,
    /// 缓存写入部分成本（USD）
    pub cache_write_price: f64,
    /// 缓存读取部分成本（USD）
    pub cache_read_price: f64,
    /// 请求总数
    pub request_count: i64,
    /// 错误请求数
    pub error_count: i64,
    /// 平均响应时间（毫秒）
    pub avg_response_time: Option<f64>,
}

/// 成本汇总分组方式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CostGroupBy {
    /// 按模型分组
    #[default]
    Model,
    /// 按配置分组
    Config,
    /// 按会话分组
    Session,
}

/// 成本汇总查询参数
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CostSummaryQuery {
    /// 开始时间戳（毫秒）
    pub start_time: Option<i64>,
    /// 结束时间戳（毫秒）
    pub end_time: Option<i64>,
    /// 工具类型过滤
    pub tool_type: Option<String>,
    /// 会话 ID 过滤
    pub session_id: Option<String>,
    /// 分组方式
    pub group_by: CostGroupBy,
}

/// 成本汇总数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSummary {
    /// 分组字段名称（model/config_name/session_id）
    pub group_name: String,
    /// 总成本（USD）
    pub total_cost: f64,
    /// 请求总数
    pub request_count: i64,
    /// 输入 Token 总数
    pub input_tokens: i64,
    /// 输出 Token 总数
    pub output_tokens: i64,
    /// 平均响应时间（毫秒）
    pub avg_response_time: Option<f64>,
}

/// Token 统计分析服务
pub struct TokenStatsAnalytics {
    db_path: PathBuf,
}

impl TokenStatsAnalytics {
    /// 创建新的分析服务实例
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    /// 查询趋势数据
    pub fn query_trends(&self, query: &TrendQuery) -> Result<Vec<TrendDataPoint>> {
        let manager = DataManager::global()
            .sqlite(&self.db_path)
            .context("Failed to get SQLite manager")?;

        // 构建时间分组表达式
        let time_expr = match query.granularity {
            TimeGranularity::FifteenMinutes => {
                // 按15分钟分组：向下取整到最近的15分钟
                "CAST((timestamp / 900000) * 900000 AS INTEGER)"
            }
            TimeGranularity::ThirtyMinutes => {
                // 按30分钟分组：向下取整到最近的30分钟
                "CAST((timestamp / 1800000) * 1800000 AS INTEGER)"
            }
            TimeGranularity::Hour => {
                // 按小时分组
                "CAST((timestamp / 3600000) * 3600000 AS INTEGER)"
            }
            TimeGranularity::TwelveHours => {
                // 按12小时分组
                "CAST((timestamp / 43200000) * 43200000 AS INTEGER)"
            }
            TimeGranularity::Day => {
                // 按天分组
                "CAST((timestamp / 86400000) * 86400000 AS INTEGER)"
            }
        };

        // 构建 WHERE 子句
        let mut where_clauses = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(start_time) = query.start_time {
            where_clauses.push("timestamp >= ?");
            params.push(Box::new(start_time));
        }

        if let Some(end_time) = query.end_time {
            where_clauses.push("timestamp <= ?");
            params.push(Box::new(end_time));
        }

        if let Some(ref tool_type) = query.tool_type {
            where_clauses.push("tool_type = ?");
            params.push(Box::new(tool_type.clone()));
        }

        if let Some(ref model) = query.model {
            where_clauses.push("model = ?");
            params.push(Box::new(model.clone()));
        }

        if let Some(ref config_name) = query.config_name {
            where_clauses.push("config_name = ?");
            params.push(Box::new(config_name.clone()));
        }

        if let Some(ref session_id) = query.session_id {
            where_clauses.push("session_id = ?");
            params.push(Box::new(session_id.clone()));
        }

        let where_clause = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        // 构建完整 SQL
        let sql = format!(
            "SELECT
                {} as timestamp,
                SUM(input_tokens) as input_tokens,
                SUM(output_tokens) as output_tokens,
                SUM(cache_creation_tokens) as cache_creation_tokens,
                SUM(cache_read_tokens) as cache_read_tokens,
                SUM(total_cost) as total_cost,
                SUM(COALESCE(input_price, 0.0)) as input_price,
                SUM(COALESCE(output_price, 0.0)) as output_price,
                SUM(COALESCE(cache_write_price, 0.0)) as cache_write_price,
                SUM(COALESCE(cache_read_price, 0.0)) as cache_read_price,
                COUNT(*) as request_count,
                SUM(CASE WHEN request_status = 'error' THEN 1 ELSE 0 END) as error_count,
                AVG(response_time_ms) as avg_response_time
            FROM token_logs
            {}
            GROUP BY {}
            ORDER BY timestamp",
            time_expr, where_clause, time_expr
        );

        // 执行查询
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let db_trends = manager.transaction(|tx| {
            let mut stmt = tx.prepare(&sql)?;
            let trends = stmt
                .query_map(param_refs.as_slice(), |row| {
                    Ok(TrendDataPoint {
                        timestamp: row.get(0)?,
                        input_tokens: row.get(1)?,
                        output_tokens: row.get(2)?,
                        cache_creation_tokens: row.get(3)?,
                        cache_read_tokens: row.get(4)?,
                        total_cost: row.get(5)?,
                        input_price: row.get(6)?,
                        output_price: row.get(7)?,
                        cache_write_price: row.get(8)?,
                        cache_read_price: row.get(9)?,
                        request_count: row.get(10)?,
                        error_count: row.get(11)?,
                        avg_response_time: row.get(12)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(crate::data::DataError::Database)?;
            Ok(trends)
        })?;

        // 如果没有指定时间范围，直接返回查询结果
        if query.start_time.is_none() || query.end_time.is_none() {
            return Ok(db_trends);
        }

        // 填充缺失的时间点
        let filled_trends = self.fill_missing_time_points(
            db_trends,
            query.start_time.unwrap(),
            query.end_time.unwrap(),
            query.granularity,
        );

        Ok(filled_trends)
    }

    /// 填充缺失的时间点，确保所有时间段都有数据（即使为0）
    fn fill_missing_time_points(
        &self,
        db_trends: Vec<TrendDataPoint>,
        start_time: i64,
        end_time: i64,
        granularity: TimeGranularity,
    ) -> Vec<TrendDataPoint> {
        use std::collections::HashMap;

        // 计算时间间隔（毫秒）
        let interval_ms = match granularity {
            TimeGranularity::FifteenMinutes => 15 * 60 * 1000,
            TimeGranularity::ThirtyMinutes => 30 * 60 * 1000,
            TimeGranularity::Hour => 60 * 60 * 1000,
            TimeGranularity::TwelveHours => 12 * 60 * 60 * 1000,
            TimeGranularity::Day => 24 * 60 * 60 * 1000,
        };

        // 将数据库结果转换为 HashMap 以便快速查找
        let mut data_map: HashMap<i64, TrendDataPoint> = HashMap::new();
        for point in db_trends {
            data_map.insert(point.timestamp, point);
        }

        // 生成完整的时间序列
        let mut result = Vec::new();
        let mut current_time = (start_time / interval_ms) * interval_ms; // 向下取整到粒度边界

        while current_time <= end_time {
            let point = if let Some(existing) = data_map.get(&current_time) {
                // 如果有数据，使用数据库的值
                existing.clone()
            } else {
                // 如果没有数据，创建零值数据点
                TrendDataPoint {
                    timestamp: current_time,
                    input_tokens: 0,
                    output_tokens: 0,
                    cache_creation_tokens: 0,
                    cache_read_tokens: 0,
                    total_cost: 0.0,
                    input_price: 0.0,
                    output_price: 0.0,
                    cache_write_price: 0.0,
                    cache_read_price: 0.0,
                    request_count: 0,
                    error_count: 0,
                    avg_response_time: None,
                }
            };
            result.push(point);
            current_time += interval_ms;
        }

        result
    }

    /// 查询成本汇总数据
    pub fn query_cost_summary(&self, query: &CostSummaryQuery) -> Result<Vec<CostSummary>> {
        let manager = DataManager::global()
            .sqlite(&self.db_path)
            .context("Failed to get SQLite manager")?;

        // 确定分组字段
        let group_field = match query.group_by {
            CostGroupBy::Model => "model",
            CostGroupBy::Config => "config_name",
            CostGroupBy::Session => "session_id",
        };

        // 构建 WHERE 子句
        let mut where_clauses = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(start_time) = query.start_time {
            where_clauses.push("timestamp >= ?");
            params.push(Box::new(start_time));
        }

        if let Some(end_time) = query.end_time {
            where_clauses.push("timestamp <= ?");
            params.push(Box::new(end_time));
        }

        if let Some(ref tool_type) = query.tool_type {
            where_clauses.push("tool_type = ?");
            params.push(Box::new(tool_type.clone()));
        }

        if let Some(ref session_id) = query.session_id {
            where_clauses.push("session_id = ?");
            params.push(Box::new(session_id.clone()));
        }

        let where_clause = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        // 构建完整 SQL
        let sql = format!(
            "SELECT
                {} as group_name,
                SUM(total_cost) as total_cost,
                COUNT(*) as request_count,
                SUM(input_tokens) as input_tokens,
                SUM(output_tokens) as output_tokens,
                AVG(response_time_ms) as avg_response_time
            FROM token_logs
            {}
            GROUP BY {}
            ORDER BY total_cost DESC",
            group_field, where_clause, group_field
        );

        // 执行查询
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        Ok(manager.transaction(|tx| {
            let mut stmt = tx.prepare(&sql)?;
            let summaries = stmt
                .query_map(param_refs.as_slice(), |row| {
                    Ok(CostSummary {
                        group_name: row.get(0)?,
                        total_cost: row.get(1)?,
                        request_count: row.get(2)?,
                        input_tokens: row.get(3)?,
                        output_tokens: row.get(4)?,
                        avg_response_time: row.get(5)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(crate::data::DataError::Database)?;
            Ok(summaries)
        })?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::token_stats::TokenLog;
    use crate::services::token_stats::db::TokenStatsDb;
    use chrono::TimeZone;
    use tempfile::tempdir;

    #[test]
    fn test_query_trends() {
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

        // 查询趋势数据
        let analytics = TokenStatsAnalytics::new(db_path);
        let query = TrendQuery {
            tool_type: Some("claude_code".to_string()),
            granularity: TimeGranularity::Hour,
            ..Default::default()
        };

        let trends = analytics.query_trends(&query).unwrap();

        // 验证结果
        assert_eq!(trends.len(), 10);
        assert_eq!(trends[0].input_tokens, 100);
        assert_eq!(trends[0].output_tokens, 50);
        assert!((trends[0].total_cost - 0.0033).abs() < 0.0001);
        assert_eq!(trends[0].request_count, 1);
        assert_eq!(trends[0].error_count, 0);
    }

    #[test]
    fn test_query_cost_summary() {
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
                    "claude-sonnet-4-5-20250929".to_string(),
                    Some(format!("msg_{}_{}", session_idx, i)),
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

        // 查询成本汇总
        let analytics = TokenStatsAnalytics::new(db_path);
        let query = CostSummaryQuery {
            tool_type: Some("claude_code".to_string()),
            group_by: CostGroupBy::Session,
            ..Default::default()
        };

        let summaries = analytics.query_cost_summary(&query).unwrap();

        // 验证结果
        assert_eq!(summaries.len(), 3); // 3个会话
        for summary in &summaries {
            assert_eq!(summary.request_count, 5); // 每个会话5条记录
            assert!((summary.total_cost - 0.0165).abs() < 0.001); // 0.0033 * 5
        }
    }
}
