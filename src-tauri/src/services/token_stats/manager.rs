use crate::models::token_stats::{SessionStats, TokenLog, TokenLogsPage, TokenStatsQuery};
use crate::services::pricing::PRICING_MANAGER;
use crate::services::token_stats::db::TokenStatsDb;
use crate::services::token_stats::extractor::{
    create_extractor, MessageDeltaData, MessageStartData, ResponseTokenInfo,
};
use crate::utils::config_dir;
use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use serde_json::Value;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tokio_util::sync::CancellationToken;

/// 全局 TokenStatsManager 单例
static TOKEN_STATS_MANAGER: OnceCell<TokenStatsManager> = OnceCell::new();

/// 全局取消令牌，用于优雅关闭后台任务
static CANCELLATION_TOKEN: once_cell::sync::Lazy<CancellationToken> =
    once_cell::sync::Lazy::new(CancellationToken::new);

/// 响应数据类型
pub enum ResponseData {
    /// SSE流式响应（收集的所有data块）
    Sse(Vec<String>),
    /// JSON响应
    Json(Value),
}

/// Token统计管理器
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

    /// 记录请求日志
    ///
    /// # 参数
    ///
    /// - `tool_type`: 工具类型（claude_code/codex/gemini_cli）
    /// - `session_id`: 会话ID
    /// - `config_name`: 使用的配置名称
    /// - `client_ip`: 客户端IP地址
    /// - `request_body`: 请求体（用于提取model）
    /// - `response_data`: 响应数据（SSE流或JSON）
    /// - `response_time_ms`: 响应时间（毫秒）
    /// - `pricing_template_id`: 价格模板ID（None则使用默认模板）
    #[allow(clippy::too_many_arguments)]
    pub async fn log_request(
        &self,
        tool_type: &str,
        session_id: &str,
        config_name: &str,
        client_ip: &str,
        request_body: &[u8],
        response_data: ResponseData,
        response_time_ms: Option<i64>,
        pricing_template_id: Option<String>,
    ) -> Result<()> {
        // 创建提取器
        let extractor = create_extractor(tool_type).context("Failed to create token extractor")?;

        // 提取请求中的模型名称
        let model = extractor
            .extract_model_from_request(request_body)
            .context("Failed to extract model from request")?;

        // 确定响应类型
        let response_type = match &response_data {
            ResponseData::Sse(_) => "sse",
            ResponseData::Json(_) => "json",
        };

        // 提取响应中的Token信息
        let token_info = match response_data {
            ResponseData::Sse(chunks) => self.parse_sse_chunks(&*extractor, chunks)?,
            ResponseData::Json(json) => extractor.extract_from_json(&json)?,
        };

        // 使用价格模板计算成本
        let template_id_ref = pricing_template_id.as_deref();

        let (
            final_input_price,
            final_output_price,
            final_cache_write_price,
            final_cache_read_price,
            final_total_cost,
            final_pricing_template_id,
        ) = match PRICING_MANAGER.calculate_cost(
            template_id_ref,
            &model,
            token_info.input_tokens,
            token_info.output_tokens,
            token_info.cache_creation_tokens,
            token_info.cache_read_tokens,
        ) {
            Ok(breakdown) => {
                tracing::debug!(
                    model = %model,
                    template_id = %breakdown.template_id,
                    total_cost = breakdown.total_cost,
                    input_tokens = token_info.input_tokens,
                    output_tokens = token_info.output_tokens,
                    "成本计算成功"
                );
                (
                    Some(breakdown.input_price),
                    Some(breakdown.output_price),
                    Some(breakdown.cache_write_price),
                    Some(breakdown.cache_read_price),
                    breakdown.total_cost,
                    Some(breakdown.template_id),
                )
            }
            Err(e) => {
                // 计算失败，使用 0
                tracing::warn!(
                    model = %model,
                    template_id = ?template_id_ref,
                    error = ?e,
                    "成本计算失败，使用默认值 0"
                );
                (None, None, None, None, 0.0, None)
            }
        };

        // 创建日志记录（成功）
        let timestamp = chrono::Utc::now().timestamp_millis();
        let log = TokenLog::new(
            tool_type.to_string(),
            timestamp,
            client_ip.to_string(),
            session_id.to_string(),
            config_name.to_string(),
            model,
            Some(token_info.message_id),
            token_info.input_tokens,
            token_info.output_tokens,
            token_info.cache_creation_tokens,
            token_info.cache_read_tokens,
            "success".to_string(),
            response_type.to_string(),
            None,
            None,
            response_time_ms,
            final_input_price,
            final_output_price,
            final_cache_write_price,
            final_cache_read_price,
            final_total_cost,
            final_pricing_template_id,
        );

        // 发送到批量写入队列（异步，不阻塞）
        if let Err(e) = self.event_sender.send(log) {
            tracing::error!("发送 Token 日志事件失败: {}", e);
        }

        Ok(())
    }

    /// 记录失败的请求
    ///
    /// # 参数
    ///
    /// - `tool_type`: 工具类型
    /// - `session_id`: 会话ID
    /// - `config_name`: 配置名称
    /// - `client_ip`: 客户端IP
    /// - `request_body`: 请求体（用于提取model，失败时可能为空）
    /// - `error_type`: 错误类型（parse_error/request_interrupted/upstream_error）
    /// - `error_detail`: 错误详情
    /// - `response_type`: 响应类型（sse/json/unknown）
    /// - `response_time_ms`: 响应时间（毫秒）
    #[allow(clippy::too_many_arguments)]
    pub async fn log_failed_request(
        &self,
        tool_type: &str,
        session_id: &str,
        config_name: &str,
        client_ip: &str,
        request_body: &[u8],
        error_type: &str,
        error_detail: &str,
        response_type: &str,
        response_time_ms: Option<i64>,
    ) -> Result<()> {
        // 尝试提取模型名称（失败时使用 "unknown"）
        let model = if !request_body.is_empty() {
            create_extractor(tool_type)
                .and_then(|extractor| extractor.extract_model_from_request(request_body))
                .unwrap_or_else(|_| "unknown".to_string())
        } else {
            "unknown".to_string()
        };

        // 创建日志记录（失败）
        let timestamp = chrono::Utc::now().timestamp_millis();
        let log = TokenLog::new(
            tool_type.to_string(),
            timestamp,
            client_ip.to_string(),
            session_id.to_string(),
            config_name.to_string(),
            model,
            None, // 失败时没有 message_id
            0,    // 失败时 token 数量为 0
            0,
            0,
            0,
            "failed".to_string(),
            response_type.to_string(),
            Some(error_type.to_string()),
            Some(error_detail.to_string()),
            response_time_ms,
            None, // 失败时没有价格信息
            None,
            None,
            None,
            0.0, // 失败时成本为 0
            None,
        );

        // 发送到批量写入队列
        if let Err(e) = self.event_sender.send(log) {
            tracing::error!("发送失败请求日志事件失败: {}", e);
        }

        Ok(())
    }

    /// 解析SSE流数据块
    fn parse_sse_chunks(
        &self,
        extractor: &dyn crate::services::token_stats::extractor::TokenExtractor,
        chunks: Vec<String>,
    ) -> Result<ResponseTokenInfo> {
        let mut message_start: Option<MessageStartData> = None;
        let mut message_delta: Option<MessageDeltaData> = None;

        for (i, chunk) in chunks.iter().enumerate() {
            match extractor.extract_from_sse_chunk(chunk) {
                Ok(Some(data)) => {
                    if let Some(start) = data.message_start {
                        tracing::debug!(chunk_index = i, "找到 message_start 事件");
                        message_start = Some(start);
                    }
                    if let Some(delta) = data.message_delta {
                        tracing::debug!(chunk_index = i, "找到 message_delta 事件");
                        message_delta = Some(delta);
                    }
                }
                Ok(None) => {
                    // 正常跳过非数据块（如 ping、空行等）
                }
                Err(e) => {
                    tracing::warn!(
                        chunk_index = i,
                        error = ?e,
                        chunk_preview = %chunk.chars().take(100).collect::<String>(),
                        "SSE chunk 解析失败"
                    );
                }
            }
        }

        if message_start.is_none() {
            tracing::error!(
                chunks_count = chunks.len(),
                "所有 SSE chunks 中未找到 message_start 事件"
            );
        }

        let start = message_start.context("Missing message_start in SSE stream")?;

        Ok(ResponseTokenInfo::from_sse_data(start, message_delta))
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
    use serde_json::json;

    #[tokio::test]
    async fn test_log_request_with_json() {
        let manager = TokenStatsManager::get();

        let request_body = json!({
            "model": "claude-3-5-sonnet-20241022",
            "messages": []
        })
        .to_string();

        let response_json = json!({
            "id": "msg_test_123",
            "model": "claude-3-5-sonnet-20241022",
            "usage": {
                "input_tokens": 100,
                "output_tokens": 50,
                "cache_creation_input_tokens": 10,
                "cache_read_input_tokens": 20
            }
        });

        let result = manager
            .log_request(
                "claude_code",
                "test_session",
                "default",
                "127.0.0.1",
                request_body.as_bytes(),
                ResponseData::Json(response_json),
                None, // response_time_ms
                None, // pricing_template_id
            )
            .await;

        assert!(result.is_ok());

        // 等待异步插入完成
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // 验证统计数据
        let stats = manager
            .get_session_stats("claude_code", "test_session")
            .unwrap();
        assert_eq!(stats.total_input, 100);
        assert_eq!(stats.total_output, 50);
    }

    #[test]
    fn test_parse_sse_chunks() {
        let manager = TokenStatsManager::get();
        let extractor = create_extractor("claude_code").unwrap();

        let chunks = vec![
            r#"data: {"type":"message_start","message":{"model":"claude-3","id":"msg_123","usage":{"input_tokens":1000,"output_tokens":1}}}"#.to_string(),
            r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"cache_creation_input_tokens":50,"cache_read_input_tokens":100,"output_tokens":200}}"#.to_string(),
        ];

        let info = manager.parse_sse_chunks(&*extractor, chunks).unwrap();
        assert_eq!(info.model, "claude-3");
        assert_eq!(info.message_id, "msg_123");
        assert_eq!(info.input_tokens, 1000);
        assert_eq!(info.output_tokens, 200);
        assert_eq!(info.cache_creation_tokens, 50);
        assert_eq!(info.cache_read_tokens, 100);
    }

    #[test]
    fn test_query_logs() {
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
            "success".to_string(),
            "json".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
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
