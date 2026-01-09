use crate::data::DataManager;
use crate::models::token_stats::{SessionStats, TokenLog, TokenLogsPage, TokenStatsQuery};
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Token统计数据库操作层
pub struct TokenStatsDb {
    db_path: PathBuf,
}

impl TokenStatsDb {
    /// 创建新的数据库操作实例
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    /// 初始化数据库表
    pub fn init_table(&self) -> Result<()> {
        let manager = DataManager::global()
            .sqlite(&self.db_path)
            .context("Failed to get SQLite manager")?;

        // 启用 WAL 模式（提升并发性能）
        manager
            .execute_raw("PRAGMA journal_mode=WAL")
            .context("Failed to enable WAL mode")?;

        // 创建表
        manager
            .execute_raw(
                "CREATE TABLE IF NOT EXISTS token_logs (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    tool_type TEXT NOT NULL,
                    timestamp INTEGER NOT NULL,
                    client_ip TEXT NOT NULL,
                    session_id TEXT NOT NULL,
                    config_name TEXT NOT NULL,
                    model TEXT NOT NULL,
                    message_id TEXT,
                    input_tokens INTEGER NOT NULL DEFAULT 0,
                    output_tokens INTEGER NOT NULL DEFAULT 0,
                    cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
                    cache_read_tokens INTEGER NOT NULL DEFAULT 0,
                    request_status TEXT NOT NULL DEFAULT 'success',
                    response_type TEXT NOT NULL DEFAULT 'unknown',
                    error_type TEXT,
                    error_detail TEXT,
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                )",
            )
            .context("Failed to create token_logs table")?;

        // 如果表已存在但缺少新字段，添加它们（ALTER TABLE）
        let _ = manager.execute_raw(
            "ALTER TABLE token_logs ADD COLUMN request_status TEXT NOT NULL DEFAULT 'success'",
        );
        let _ = manager.execute_raw(
            "ALTER TABLE token_logs ADD COLUMN response_type TEXT NOT NULL DEFAULT 'unknown'",
        );
        let _ = manager.execute_raw("ALTER TABLE token_logs ADD COLUMN error_type TEXT");
        let _ = manager.execute_raw("ALTER TABLE token_logs ADD COLUMN error_detail TEXT");

        // 创建索引
        manager
            .execute_raw(
                "CREATE INDEX IF NOT EXISTS idx_session_timestamp
                 ON token_logs(session_id, timestamp)",
            )
            .context("Failed to create session_timestamp index")?;

        manager
            .execute_raw(
                "CREATE INDEX IF NOT EXISTS idx_timestamp
                 ON token_logs(timestamp)",
            )
            .context("Failed to create timestamp index")?;

        manager
            .execute_raw(
                "CREATE INDEX IF NOT EXISTS idx_tool_type
                 ON token_logs(tool_type)",
            )
            .context("Failed to create tool_type index")?;

        Ok(())
    }

    /// 插入单条日志记录
    pub fn insert_log(&self, log: &TokenLog) -> Result<i64> {
        let manager = DataManager::global()
            .sqlite(&self.db_path)
            .context("Failed to get SQLite manager")?;

        let params = vec![
            log.tool_type.clone(),
            log.timestamp.to_string(),
            log.client_ip.clone(),
            log.session_id.clone(),
            log.config_name.clone(),
            log.model.clone(),
            log.message_id.clone().unwrap_or_default(),
            log.input_tokens.to_string(),
            log.output_tokens.to_string(),
            log.cache_creation_tokens.to_string(),
            log.cache_read_tokens.to_string(),
            log.request_status.clone(),
            log.response_type.clone(),
            log.error_type.clone().unwrap_or_default(),
            log.error_detail.clone().unwrap_or_default(),
        ];

        let params_refs: Vec<&str> = params.iter().map(|s| s.as_str()).collect();

        manager
            .execute(
                "INSERT INTO token_logs (
                    tool_type, timestamp, client_ip, session_id, config_name,
                    model, message_id, input_tokens, output_tokens,
                    cache_creation_tokens, cache_read_tokens,
                    request_status, response_type, error_type, error_detail
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                &params_refs,
            )
            .context("Failed to insert token log")?;

        // 执行 WAL checkpoint（TRUNCATE 模式，立即回写主文件）
        // 注意：这会稍微影响写入性能，但确保数据及时持久化
        let _ = manager.execute_raw("PRAGMA wal_checkpoint(TRUNCATE)");

        // 获取最后插入的ID（通过查询max(id)）
        let rows = manager
            .query("SELECT max(id) as last_id FROM token_logs", &[])
            .context("Failed to query last insert id")?;

        let id = rows
            .first()
            .and_then(|row| row.values.first())
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        Ok(id)
    }

    /// 插入单条日志记录（不执行 checkpoint）
    ///
    /// 用于批量写入场景，在批量写入后统一执行 checkpoint
    pub fn insert_log_without_checkpoint(&self, log: &TokenLog) -> Result<i64> {
        let manager = DataManager::global()
            .sqlite(&self.db_path)
            .context("Failed to get SQLite manager")?;

        let params = vec![
            log.tool_type.clone(),
            log.timestamp.to_string(),
            log.client_ip.clone(),
            log.session_id.clone(),
            log.config_name.clone(),
            log.model.clone(),
            log.message_id.clone().unwrap_or_default(),
            log.input_tokens.to_string(),
            log.output_tokens.to_string(),
            log.cache_creation_tokens.to_string(),
            log.cache_read_tokens.to_string(),
            log.request_status.clone(),
            log.response_type.clone(),
            log.error_type.clone().unwrap_or_default(),
            log.error_detail.clone().unwrap_or_default(),
        ];

        let params_refs: Vec<&str> = params.iter().map(|s| s.as_str()).collect();

        manager
            .execute(
                "INSERT INTO token_logs (
                    tool_type, timestamp, client_ip, session_id, config_name,
                    model, message_id, input_tokens, output_tokens,
                    cache_creation_tokens, cache_read_tokens,
                    request_status, response_type, error_type, error_detail
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                &params_refs,
            )
            .context("Failed to insert token log")?;

        // 获取最后插入的ID
        let rows = manager
            .query("SELECT max(id) as last_id FROM token_logs", &[])
            .context("Failed to query last insert id")?;

        let id = rows
            .first()
            .and_then(|row| row.values.first())
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        Ok(id)
    }

    /// 查询会话统计数据
    pub fn get_session_stats(&self, tool_type: &str, session_id: &str) -> Result<SessionStats> {
        let manager = DataManager::global()
            .sqlite(&self.db_path)
            .context("Failed to get SQLite manager")?;

        let rows = manager
            .query(
                "SELECT
                    COALESCE(SUM(input_tokens), 0) as total_input,
                    COALESCE(SUM(output_tokens), 0) as total_output,
                    COALESCE(SUM(cache_creation_tokens), 0) as total_cache_creation,
                    COALESCE(SUM(cache_read_tokens), 0) as total_cache_read,
                    COUNT(*) as request_count
                FROM token_logs
                WHERE session_id = ?1 AND tool_type = ?2",
                &[session_id, tool_type],
            )
            .context("Failed to query session stats")?;

        let row = rows.first().context("No stats row returned")?;

        Ok(SessionStats {
            total_input: row.values.first().and_then(|v| v.as_i64()).unwrap_or(0),
            total_output: row.values.get(1).and_then(|v| v.as_i64()).unwrap_or(0),
            total_cache_creation: row.values.get(2).and_then(|v| v.as_i64()).unwrap_or(0),
            total_cache_read: row.values.get(3).and_then(|v| v.as_i64()).unwrap_or(0),
            request_count: row.values.get(4).and_then(|v| v.as_i64()).unwrap_or(0),
        })
    }

    /// 分页查询日志记录
    pub fn query_logs(&self, query: &TokenStatsQuery) -> Result<TokenLogsPage> {
        let manager = DataManager::global()
            .sqlite(&self.db_path)
            .context("Failed to get SQLite manager")?;

        // 构建查询条件
        let mut where_clauses = Vec::new();
        let mut params = Vec::new();

        if let Some(ref tool_type) = query.tool_type {
            where_clauses.push("tool_type = ?");
            params.push(tool_type.clone());
        }

        if let Some(ref session_id) = query.session_id {
            where_clauses.push("session_id = ?");
            params.push(session_id.clone());
        }

        if let Some(ref config_name) = query.config_name {
            where_clauses.push("config_name = ?");
            params.push(config_name.clone());
        }

        if let Some(start_time) = query.start_time {
            where_clauses.push("timestamp >= ?");
            params.push(start_time.to_string());
        }

        if let Some(end_time) = query.end_time {
            where_clauses.push("timestamp <= ?");
            params.push(end_time.to_string());
        }

        let where_clause = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        // 查询总数
        let count_sql = format!("SELECT COUNT(*) FROM token_logs {}", where_clause);
        let params_refs: Vec<&str> = params.iter().map(|s| s.as_str()).collect();

        let count_rows = manager
            .query(&count_sql, &params_refs)
            .context("Failed to query total count")?;

        let total: i64 = count_rows
            .first()
            .and_then(|row| row.values.first())
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        // 查询日志列表
        let offset = query.page * query.page_size;
        let list_sql = format!(
            "SELECT id, tool_type, timestamp, client_ip, session_id, config_name,
                    model, message_id, input_tokens, output_tokens,
                    cache_creation_tokens, cache_read_tokens,
                    request_status, response_type, error_type, error_detail
             FROM token_logs {}
             ORDER BY timestamp DESC
             LIMIT ? OFFSET ?",
            where_clause
        );

        let mut list_params = params.clone();
        list_params.push(query.page_size.to_string());
        list_params.push(offset.to_string());

        let list_params_refs: Vec<&str> = list_params.iter().map(|s| s.as_str()).collect();

        let list_rows = manager
            .query(&list_sql, &list_params_refs)
            .context("Failed to query logs")?;

        let logs = list_rows
            .iter()
            .map(|row| {
                Ok(TokenLog {
                    id: row.values.first().and_then(|v| v.as_i64()),
                    tool_type: row
                        .values
                        .get(1)
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    timestamp: row.values.get(2).and_then(|v| v.as_i64()).unwrap_or(0),
                    client_ip: row
                        .values
                        .get(3)
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    session_id: row
                        .values
                        .get(4)
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    config_name: row
                        .values
                        .get(5)
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    model: row
                        .values
                        .get(6)
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    message_id: row.values.get(7).and_then(|v| v.as_str()).map(String::from),
                    input_tokens: row.values.get(8).and_then(|v| v.as_i64()).unwrap_or(0),
                    output_tokens: row.values.get(9).and_then(|v| v.as_i64()).unwrap_or(0),
                    cache_creation_tokens: row.values.get(10).and_then(|v| v.as_i64()).unwrap_or(0),
                    cache_read_tokens: row.values.get(11).and_then(|v| v.as_i64()).unwrap_or(0),
                    request_status: row
                        .values
                        .get(12)
                        .and_then(|v| v.as_str())
                        .unwrap_or("success")
                        .to_string(),
                    response_type: row
                        .values
                        .get(13)
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    error_type: row
                        .values
                        .get(14)
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    error_detail: row
                        .values
                        .get(15)
                        .and_then(|v| v.as_str())
                        .map(String::from),
                })
            })
            .collect::<Result<Vec<TokenLog>>>()?;

        Ok(TokenLogsPage {
            logs,
            total,
            page: query.page,
            page_size: query.page_size,
        })
    }

    /// 清理旧数据
    pub fn cleanup_old_logs(
        &self,
        retention_days: Option<u32>,
        max_count: Option<u32>,
    ) -> Result<usize> {
        let manager = DataManager::global()
            .sqlite(&self.db_path)
            .context("Failed to get SQLite manager")?;

        let mut deleted_count = 0;

        // 按时间清理
        if let Some(days) = retention_days {
            let cutoff_timestamp =
                chrono::Utc::now().timestamp_millis() - (days as i64 * 86400 * 1000);
            let count = manager
                .execute(
                    "DELETE FROM token_logs WHERE timestamp < ?1",
                    &[&cutoff_timestamp.to_string()],
                )
                .context("Failed to delete old logs by time")?;
            deleted_count += count;
        }

        // 按条数清理
        if let Some(max) = max_count {
            let count = manager
                .execute(
                    "DELETE FROM token_logs
                     WHERE id NOT IN (
                         SELECT id FROM token_logs
                         ORDER BY timestamp DESC
                         LIMIT ?1
                     )",
                    &[&max.to_string()],
                )
                .context("Failed to delete old logs by count")?;
            deleted_count += count;
        }

        // 执行 WAL checkpoint 回写主文件
        if deleted_count > 0 {
            manager
                .execute_raw("PRAGMA wal_checkpoint(TRUNCATE)")
                .context("Failed to checkpoint WAL")?;
        }

        Ok(deleted_count)
    }

    /// 获取数据库统计信息
    pub fn get_stats_summary(&self) -> Result<(i64, Option<i64>, Option<i64>)> {
        let manager = DataManager::global()
            .sqlite(&self.db_path)
            .context("Failed to get SQLite manager")?;

        let rows = manager
            .query(
                "SELECT
                    COUNT(*) as total,
                    MIN(timestamp) as oldest,
                    MAX(timestamp) as newest
                FROM token_logs",
                &[],
            )
            .context("Failed to query stats summary")?;

        let row = rows.first().context("No summary row returned")?;

        let total = row.values.first().and_then(|v| v.as_i64()).unwrap_or(0);
        let oldest = row.values.get(1).and_then(|v| v.as_i64());
        let newest = row.values.get(2).and_then(|v| v.as_i64());

        Ok((total, oldest, newest))
    }

    /// 强制执行 WAL checkpoint（手动触发）
    ///
    /// 将 WAL 文件中的所有数据回写到主数据库文件，
    /// 用于清理过大的 WAL 文件
    pub fn force_checkpoint(&self) -> Result<()> {
        let manager = DataManager::global()
            .sqlite(&self.db_path)
            .context("Failed to get SQLite manager")?;

        manager
            .execute_raw("PRAGMA wal_checkpoint(TRUNCATE)")
            .context("Failed to execute WAL checkpoint")?;

        Ok(())
    }

    /// 执行 PASSIVE checkpoint
    ///
    /// 尽可能多地将 WAL 数据回写到主文件，但不阻塞其他操作。
    /// 适合在批量写入后执行，性能影响最小。
    pub fn passive_checkpoint(&self) -> Result<()> {
        let manager = DataManager::global()
            .sqlite(&self.db_path)
            .context("Failed to get SQLite manager")?;

        manager
            .execute_raw("PRAGMA wal_checkpoint(PASSIVE)")
            .context("Failed to execute PASSIVE checkpoint")?;

        Ok(())
    }
}

impl Clone for TokenStatsDb {
    fn clone(&self) -> Self {
        Self::new(self.db_path.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_db() -> (TokenStatsDb, PathBuf) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_token_stats.db");
        let db = TokenStatsDb::new(db_path.clone());
        db.init_table().unwrap();
        (db, db_path)
    }

    #[test]
    fn test_init_table() {
        let (db, _) = create_test_db();
        // 重复初始化不应报错
        assert!(db.init_table().is_ok());
    }

    #[test]
    fn test_insert_and_query() {
        let (db, _) = create_test_db();

        let log = TokenLog::new(
            "claude_code".to_string(),
            chrono::Utc::now().timestamp_millis(),
            "127.0.0.1".to_string(),
            "session_123".to_string(),
            "default".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
            Some("msg_123".to_string()),
            1000,
            500,
            100,
            200,
            "success".to_string(),
            "json".to_string(),
            None,
            None,
        );

        let id = db.insert_log(&log).unwrap();
        assert!(id > 0);

        // 查询会话统计
        let stats = db.get_session_stats("claude_code", "session_123").unwrap();
        assert_eq!(stats.total_input, 1000);
        assert_eq!(stats.total_output, 500);
        assert_eq!(stats.request_count, 1);
    }

    #[test]
    fn test_query_logs_pagination() {
        let (db, _) = create_test_db();

        // 插入多条记录
        for i in 0..25 {
            let log = TokenLog::new(
                "claude_code".to_string(),
                chrono::Utc::now().timestamp_millis() + i,
                "127.0.0.1".to_string(),
                "session_123".to_string(),
                "default".to_string(),
                "claude-3-5-sonnet-20241022".to_string(),
                Some(format!("msg_{}", i)),
                100,
                50,
                10,
                20,
                "success".to_string(),
                "sse".to_string(),
                None,
                None,
            );
            db.insert_log(&log).unwrap();
        }

        // 查询第一页
        let query = TokenStatsQuery {
            page: 0,
            page_size: 10,
            ..Default::default()
        };
        let page = db.query_logs(&query).unwrap();
        assert_eq!(page.logs.len(), 10);
        assert_eq!(page.total, 25);

        // 查询第三页
        let query = TokenStatsQuery {
            page: 2,
            page_size: 10,
            ..Default::default()
        };
        let page = db.query_logs(&query).unwrap();
        assert_eq!(page.logs.len(), 5);
    }

    #[test]
    fn test_cleanup() {
        let (db, _) = create_test_db();

        // 插入旧数据和新数据
        let old_timestamp = chrono::Utc::now().timestamp_millis() - (40 * 86400 * 1000); // 40天前
        let old_log = TokenLog::new(
            "claude_code".to_string(),
            old_timestamp,
            "127.0.0.1".to_string(),
            "session_old".to_string(),
            "default".to_string(),
            "claude-3".to_string(),
            None,
            100,
            50,
            0,
            0,
            "success".to_string(),
            "json".to_string(),
            None,
            None,
        );
        db.insert_log(&old_log).unwrap();

        let new_log = TokenLog::new(
            "claude_code".to_string(),
            chrono::Utc::now().timestamp_millis(),
            "127.0.0.1".to_string(),
            "session_new".to_string(),
            "default".to_string(),
            "claude-3".to_string(),
            None,
            200,
            100,
            0,
            0,
            "success".to_string(),
            "json".to_string(),
            None,
            None,
        );
        db.insert_log(&new_log).unwrap();

        // 清理30天前的数据
        let deleted = db.cleanup_old_logs(Some(30), None).unwrap();
        assert_eq!(deleted, 1);

        // 验证新数据仍在
        let stats = db.get_session_stats("claude_code", "session_new").unwrap();
        assert_eq!(stats.request_count, 1);
    }
}
