use crate::models::token_stats::{SessionStats, TokenLog, TokenLogsPage, TokenStatsQuery};
use crate::services::token_stats::db::TokenStatsDb;
use crate::utils::config_dir;
use anyhow::Result;
use once_cell::sync::OnceCell;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tokio_util::sync::CancellationToken;

/// 全局 TokenStatsManager 单例
static TOKEN_STATS_MANAGER: OnceCell<TokenStatsManager> = OnceCell::new();

/// 全局取消令牌，用于优雅关闭后台任务
static CANCELLATION_TOKEN: once_cell::sync::Lazy<CancellationToken> =
    once_cell::sync::Lazy::new(CancellationToken::new);

/// Token统计管理器（简化版）
///
/// 职责：仅负责将 TokenLog 写入数据库，不再负责提取 Token 信息和计算成本
pub struct TokenStatsManager {
    db: TokenStatsDb,
    event_sender: mpsc::UnboundedSender<TokenLog>,
}

impl TokenStatsManager {
    /// 获取全局单例实例
    pub fn get() -> &'static TokenStatsManager {
        TOKEN_STATS_MANAGER.get_or_init(|| {
            let db_path = Self::default_db_path();
            let db = TokenStatsDb::new(db_path);

            // 初始化数据库表
            if let Err(e) = db.init_table() {
                eprintln!("Failed to initialize token stats database: {}", e);
            }

            // 创建事件队列
            let (event_sender, event_receiver) = mpsc::unbounded_channel();

            let manager = TokenStatsManager { db, event_sender };

            // 启动后台任务
            manager.start_background_tasks(event_receiver);

            manager
        })
    }

    /// 获取默认数据库路径
    fn default_db_path() -> PathBuf {
        config_dir()
            .map(|dir| dir.join("token_stats.db"))
            .unwrap_or_else(|_| PathBuf::from("token_stats.db"))
    }

    /// 启动后台任务
    fn start_background_tasks(&self, mut event_receiver: mpsc::UnboundedReceiver<TokenLog>) {
        let db = self.db.clone();

        // 批量写入任务
        tokio::spawn(async move {
            let mut buffer: Vec<TokenLog> = Vec::new();
            let mut tick_interval = interval(Duration::from_millis(100));

            loop {
                tokio::select! {
                    _ = CANCELLATION_TOKEN.cancelled() => {
                        // 应用关闭，刷盘缓冲区
                        if !buffer.is_empty() {
                            Self::flush_logs(&db, &mut buffer, true);
                            tracing::info!("Token 日志已刷盘: {} 条", buffer.len());
                        }
                        tracing::info!("Token 批量写入任务已停止");
                        break;
                    }
                    // 接收日志事件
                    Some(log) = event_receiver.recv() => {
                        buffer.push(log);

                        // 如果缓冲区达到 10 条，立即写入
                        if buffer.len() >= 10 {
                            Self::flush_logs(&db, &mut buffer, false);
                        }
                    }
                    // 每 100ms 刷新一次
                    _ = tick_interval.tick() => {
                        if !buffer.is_empty() {
                            Self::flush_logs(&db, &mut buffer, false);
                        }
                    }
                }
            }
        });

        // 定期 TRUNCATE checkpoint 任务（每 5 分钟）
        let db_clone = self.db.clone();
        tokio::spawn(async move {
            let mut checkpoint_interval = interval(Duration::from_secs(300)); // 5分钟

            loop {
                tokio::select! {
                    _ = CANCELLATION_TOKEN.cancelled() => {
                        tracing::info!("Token Checkpoint 任务已停止");
                        break;
                    }
                    _ = checkpoint_interval.tick() => {
                        if let Err(e) = db_clone.force_checkpoint() {
                            tracing::error!("定期 Checkpoint 失败: {}", e);
                        } else {
                            tracing::debug!("Token 数据库 TRUNCATE checkpoint 完成");
                        }
                    }
                }
            }
        });
    }

    /// 批量写入日志到数据库
    ///
    /// # 参数
    /// - `db`: 数据库实例
    /// - `buffer`: 日志缓冲区
    /// - `use_truncate`: 是否使用 TRUNCATE checkpoint（应用关闭时使用）
    fn flush_logs(db: &TokenStatsDb, buffer: &mut Vec<TokenLog>, use_truncate: bool) {
        for log in buffer.drain(..) {
            if let Err(e) = db.insert_log_without_checkpoint(&log) {
                tracing::error!("插入 Token 日志失败: {}", e);
            }
        }

        // 批量写入后执行 checkpoint
        let checkpoint_result = if use_truncate {
            db.force_checkpoint() // TRUNCATE模式
        } else {
            db.passive_checkpoint() // PASSIVE模式
        };

        if let Err(e) = checkpoint_result {
            tracing::error!("Checkpoint 失败: {}", e);
        }
    }

    /// 写入日志（新架构）
    ///
    /// 直接写入已经构建好的 TokenLog 到队列
    ///
    /// # 参数
    /// - `log`: 已经构建好的 TokenLog 对象
    pub fn write_log(&self, log: TokenLog) {
        // 发送到批量写入队列（异步，不阻塞）
        if let Err(e) = self.event_sender.send(log) {
            tracing::error!("发送 Token 日志事件失败: {}", e);
        }
    }

    /// 查询会话实时统计
    pub fn get_session_stats(&self, tool_type: &str, session_id: &str) -> Result<SessionStats> {
        self.db.get_session_stats(tool_type, session_id)
    }

    /// 分页查询历史日志
    pub fn query_logs(&self, query: TokenStatsQuery) -> Result<TokenLogsPage> {
        self.db.query_logs(&query)
    }

    /// 根据配置清理旧数据
    pub fn cleanup_by_config(
        &self,
        retention_days: Option<u32>,
        max_count: Option<u32>,
    ) -> Result<usize> {
        self.db.cleanup_old_logs(retention_days, max_count)
    }

    /// 获取数据库统计摘要
    pub fn get_stats_summary(&self) -> Result<(i64, Option<i64>, Option<i64>)> {
        self.db.get_stats_summary()
    }

    /// 强制执行 WAL checkpoint
    ///
    /// 将所有 WAL 数据回写到主数据库文件，
    /// 用于手动清理过大的 WAL 文件
    pub fn force_checkpoint(&self) -> Result<()> {
        self.db.force_checkpoint()
    }
}

/// 关闭 TokenStatsManager 后台任务
///
/// 在应用关闭时调用，优雅地停止所有后台任务并刷盘缓冲区数据
pub fn shutdown_token_stats_manager() {
    tracing::info!("TokenStatsManager 关闭信号已发送");
    CANCELLATION_TOKEN.cancel();

    // 等待一小段时间让任务完成刷盘
    std::thread::sleep(std::time::Duration::from_millis(300));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_write_log() {
        let manager = TokenStatsManager::get();

        // 创建测试日志
        let log = TokenLog::new(
            "claude_code".to_string(),
            chrono::Utc::now().timestamp_millis(),
            "127.0.0.1".to_string(),
            "test_write_session".to_string(),
            "default".to_string(),
            "claude-3".to_string(),
            Some("msg_write_test".to_string()),
            100,
            50,
            10,
            20,
            0, // reasoning_tokens
            "success".to_string(),
            "json".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None, // reasoning_price
            0.0,
            None,
        );

        // 写入日志
        manager.write_log(log);

        // 等待异步插入完成
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    #[tokio::test]
    async fn test_query_logs() {
        let manager = TokenStatsManager::get();

        // 插入测试数据
        let log = TokenLog::new(
            "claude_code".to_string(),
            chrono::Utc::now().timestamp_millis(),
            "127.0.0.1".to_string(),
            "test_query_session".to_string(),
            "default".to_string(),
            "claude-3".to_string(),
            Some("msg_query_test".to_string()),
            100,
            50,
            10,
            20,
            0, // reasoning_tokens
            "success".to_string(),
            "json".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None, // reasoning_price
            0.0,
            None,
        );
        manager.db.insert_log(&log).unwrap();

        // 查询日志
        let query = TokenStatsQuery {
            session_id: Some("test_query_session".to_string()),
            ..Default::default()
        };
        let page = manager.query_logs(query).unwrap();
        assert!(page.total >= 1);
    }
}
