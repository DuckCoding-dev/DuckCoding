use duckcoding::models::token_stats::{SessionStats, TokenLogsPage, TokenStatsQuery};
use duckcoding::services::token_stats::TokenStatsManager;

/// 查询会话实时统计
#[tauri::command]
pub async fn get_session_stats(
    tool_type: String,
    session_id: String,
) -> Result<SessionStats, String> {
    TokenStatsManager::get()
        .get_session_stats(&tool_type, &session_id)
        .map_err(|e| e.to_string())
}

/// 分页查询Token日志
#[tauri::command]
pub async fn query_token_logs(query_params: TokenStatsQuery) -> Result<TokenLogsPage, String> {
    TokenStatsManager::get()
        .query_logs(query_params)
        .map_err(|e| e.to_string())
}

/// 手动清理旧日志
#[tauri::command]
pub async fn cleanup_token_logs(
    retention_days: Option<u32>,
    max_count: Option<u32>,
) -> Result<usize, String> {
    TokenStatsManager::get()
        .cleanup_by_config(retention_days, max_count)
        .map_err(|e| e.to_string())
}

/// 获取数据库统计摘要
#[tauri::command]
pub async fn get_token_stats_summary() -> Result<(i64, Option<i64>, Option<i64>), String> {
    TokenStatsManager::get()
        .get_stats_summary()
        .map_err(|e| e.to_string())
}

/// 强制执行 WAL checkpoint
///
/// 将 WAL 文件中的所有数据回写到主数据库文件，
/// 用于手动清理过大的 WAL 文件
#[tauri::command]
pub async fn force_token_stats_checkpoint() -> Result<(), String> {
    TokenStatsManager::get()
        .force_checkpoint()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_session_stats() {
        let result = get_session_stats("claude_code".to_string(), "test_session".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_query_token_logs() {
        let query = TokenStatsQuery::default();
        let result = query_token_logs(query).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cleanup_token_logs() {
        let result = cleanup_token_logs(Some(30), Some(10000)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_token_stats_summary() {
        let result = get_token_stats_summary().await;
        assert!(result.is_ok());
    }
}
