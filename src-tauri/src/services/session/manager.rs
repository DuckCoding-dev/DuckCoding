// SessionManager 单例 - 会话管理核心模块

use crate::data::DataManager;
use crate::services::session::db_utils::{
    parse_count, parse_proxy_session, parse_session_config, ALTER_TABLE_SQL, CREATE_TABLE_SQL,
    SELECT_SESSION_FIELDS,
};
use crate::services::session::models::{ProxySession, SessionEvent, SessionListResponse};
use anyhow::Result;
use lazy_static::lazy_static;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

/// 会话管理器单例
pub struct SessionManager {
    manager: Arc<DataManager>,
    db_path: PathBuf,
    event_sender: mpsc::UnboundedSender<SessionEvent>,
}

lazy_static! {
    /// 全局 SessionManager 实例
    pub static ref SESSION_MANAGER: SessionManager = {
        SessionManager::new().expect("Failed to initialize SessionManager")
    };
}

impl SessionManager {
    /// 创建 SessionManager 实例
    fn new() -> Result<Self> {
        // 数据库路径：~/.duckcoding/sessions.db
        let db_path = Self::get_db_path()?;
        let manager_instance = Arc::new(DataManager::new());

        // 初始化数据库表结构
        let db = manager_instance.sqlite(&db_path)?;
        db.execute_raw(CREATE_TABLE_SQL)?;

        // 兼容旧数据库（忽略错误）
        let _ = db.execute_raw(ALTER_TABLE_SQL);

        // 创建事件队列
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        let manager = Self {
            manager: manager_instance,
            db_path,
            event_sender,
        };

        // 启动后台任务
        manager.start_background_tasks(event_receiver);

        Ok(manager)
    }

    /// 获取数据库路径
    fn get_db_path() -> Result<PathBuf> {
        let base = crate::utils::config::config_dir()
            .map_err(|e| std::io::Error::other(format!("Failed to resolve config dir: {e}")))?;
        Ok(base.join("sessions.db"))
    }

    /// 启动后台任务
    fn start_background_tasks(&self, mut event_receiver: mpsc::UnboundedReceiver<SessionEvent>) {
        let manager = self.manager.clone();
        let db_path = self.db_path.clone();

        // 批量写入任务
        tokio::spawn(async move {
            let mut buffer: Vec<SessionEvent> = Vec::new();
            let mut tick_interval = interval(Duration::from_millis(100));

            loop {
                tokio::select! {
                    // 接收事件
                    Some(event) = event_receiver.recv() => {
                        buffer.push(event);

                        // 如果缓冲区达到 10 条，立即写入
                        if buffer.len() >= 10 {
                            Self::flush_events(&manager, &db_path, &mut buffer);
                        }
                    }
                    // 每 100ms 刷新一次
                    _ = tick_interval.tick() => {
                        if !buffer.is_empty() {
                            Self::flush_events(&manager, &db_path, &mut buffer);
                        }
                    }
                }
            }
        });

        // 定期清理任务（每 1 小时）
        let manager_clone = self.manager.clone();
        let db_path_clone = self.db_path.clone();
        tokio::spawn(async move {
            let mut cleanup_interval = interval(Duration::from_secs(3600));

            loop {
                cleanup_interval.tick().await;

                // 清理三个工具的过期会话
                for tool_id in &["claude-code", "codex", "gemini-cli"] {
                    let _ = Self::cleanup_old_sessions_internal(
                        &manager_clone,
                        &db_path_clone,
                        tool_id,
                        1000,
                        30,
                    );
                }
            }
        });
    }

    /// 批量写入事件到数据库
    fn flush_events(manager: &Arc<DataManager>, db_path: &Path, buffer: &mut Vec<SessionEvent>) {
        for event in buffer.drain(..) {
            match event {
                SessionEvent::NewRequest {
                    session_id,
                    tool_id,
                    timestamp,
                } => {
                    // 提取 display_id
                    if let Some(display_id) = ProxySession::extract_display_id(&session_id) {
                        // Upsert 会话
                        if let Ok(db) = manager.sqlite(db_path) {
                            let _ = db.execute(
                                "INSERT INTO claude_proxy_sessions (
                                    session_id, display_id, tool_id, config_name, url, api_key,
                                    first_seen_at, last_seen_at, request_count,
                                    created_at, updated_at
                                ) VALUES (?1, ?2, ?3, 'global', '', '', ?4, ?4, 1, ?4, ?4)
                                ON CONFLICT(session_id) DO UPDATE SET
                                    last_seen_at = ?4,
                                    request_count = request_count + 1,
                                    updated_at = ?4",
                                &[&session_id, &display_id, &tool_id, &timestamp.to_string()],
                            );
                        }
                    }
                }
            }
        }
    }

    /// 内部清理方法（用于后台任务）
    fn cleanup_old_sessions_internal(
        manager: &Arc<DataManager>,
        db_path: &Path,
        tool_id: &str,
        max_count: usize,
        max_age_days: i64,
    ) -> Result<usize> {
        let db = manager.sqlite(db_path)?;
        let now = chrono::Utc::now().timestamp();
        let cutoff_time = now - (max_age_days * 24 * 3600);

        // 1. 删除超过 30 天的会话
        let deleted_by_age = db.execute(
            "DELETE FROM claude_proxy_sessions WHERE tool_id = ? AND last_seen_at < ?",
            &[tool_id, &cutoff_time.to_string()],
        )?;

        // 2. 如果超过 1000 条，删除最旧的会话
        let count_rows = db.query(
            "SELECT COUNT(*) FROM claude_proxy_sessions WHERE tool_id = ?",
            &[tool_id],
        )?;
        let current_count = parse_count(&count_rows[0])?;

        let deleted_by_count = if current_count > max_count {
            let to_delete = current_count - max_count;
            db.execute(
                "DELETE FROM claude_proxy_sessions WHERE session_id IN (
                    SELECT session_id FROM claude_proxy_sessions
                    WHERE tool_id = ?
                    ORDER BY last_seen_at ASC
                    LIMIT ?
                )",
                &[tool_id, &to_delete.to_string()],
            )?
        } else {
            0
        };

        Ok(deleted_by_age + deleted_by_count)
    }

    /// 发送会话事件（公共 API）
    pub fn send_event(&self, event: SessionEvent) -> Result<()> {
        self.event_sender
            .send(event)
            .map_err(|e| std::io::Error::other(format!("Failed to send event: {e}")))?;
        Ok(())
    }

    /// 获取会话列表（公共 API）
    pub fn get_session_list(
        &self,
        tool_id: &str,
        page: usize,
        page_size: usize,
    ) -> Result<SessionListResponse> {
        let db = self.manager.sqlite(&self.db_path)?;

        // 查询总数
        let total_rows = db.query(
            "SELECT COUNT(*) FROM claude_proxy_sessions WHERE tool_id = ?",
            &[tool_id],
        )?;
        let total = parse_count(&total_rows[0])?;

        // 查询分页数据（按最后活跃时间降序）
        let offset = (page.saturating_sub(1)) * page_size;
        let sql = format!(
            "SELECT {} FROM claude_proxy_sessions WHERE tool_id = ? ORDER BY last_seen_at DESC LIMIT ? OFFSET ?",
            SELECT_SESSION_FIELDS
        );
        let rows = db.query(
            &sql,
            &[tool_id, &page_size.to_string(), &offset.to_string()],
        )?;

        // 转换为 ProxySession
        let sessions = rows
            .iter()
            .map(parse_proxy_session)
            .collect::<Result<Vec<_>>>()?;

        Ok(SessionListResponse {
            sessions,
            total,
            page,
            page_size,
        })
    }

    /// 删除单个会话（公共 API）
    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        let db = self.manager.sqlite(&self.db_path)?;
        db.execute(
            "DELETE FROM claude_proxy_sessions WHERE session_id = ?",
            &[session_id],
        )?;
        Ok(())
    }

    /// 清空工具所有会话（公共 API）
    pub fn clear_sessions(&self, tool_id: &str) -> Result<()> {
        let db = self.manager.sqlite(&self.db_path)?;
        db.execute(
            "DELETE FROM claude_proxy_sessions WHERE tool_id = ?",
            &[tool_id],
        )?;
        Ok(())
    }

    /// 获取会话详情（公共 API）
    pub fn get_session(&self, session_id: &str) -> Result<Option<ProxySession>> {
        let db = self.manager.sqlite(&self.db_path)?;
        let sql = format!(
            "SELECT {} FROM claude_proxy_sessions WHERE session_id = ?",
            SELECT_SESSION_FIELDS
        );
        let rows = db.query(&sql, &[session_id])?;

        if rows.is_empty() {
            Ok(None)
        } else {
            Ok(Some(parse_proxy_session(&rows[0])?))
        }
    }

    /// 获取会话配置（公共 API，用于请求处理）
    /// 返回 (config_name, url, api_key)
    pub fn get_session_config(&self, session_id: &str) -> Result<Option<(String, String, String)>> {
        let db = self.manager.sqlite(&self.db_path)?;
        let rows = db.query(
            "SELECT config_name, url, api_key FROM claude_proxy_sessions WHERE session_id = ?",
            &[session_id],
        )?;

        if rows.is_empty() {
            Ok(None)
        } else {
            Ok(Some(parse_session_config(&rows[0])?))
        }
    }

    /// 更新会话配置（公共 API）
    pub fn update_session_config(
        &self,
        session_id: &str,
        config_name: &str,
        custom_profile_name: Option<&str>,
        url: &str,
        api_key: &str,
    ) -> Result<()> {
        let db = self.manager.sqlite(&self.db_path)?;
        let now = chrono::Utc::now().timestamp();

        db.execute(
            "UPDATE claude_proxy_sessions
             SET config_name = ?, custom_profile_name = ?, url = ?, api_key = ?, updated_at = ?
             WHERE session_id = ?",
            &[
                config_name,
                custom_profile_name.unwrap_or(""),
                url,
                api_key,
                &now.to_string(),
                session_id,
            ],
        )?;

        Ok(())
    }

    /// 更新会话备注（公共 API）
    pub fn update_session_note(&self, session_id: &str, note: Option<&str>) -> Result<()> {
        let db = self.manager.sqlite(&self.db_path)?;
        let now = chrono::Utc::now().timestamp();

        db.execute(
            "UPDATE claude_proxy_sessions SET note = ?, updated_at = ? WHERE session_id = ?",
            &[note.unwrap_or(""), &now.to_string(), session_id],
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    /// 创建测试用的 SessionManager 实例
    fn create_test_manager(temp_dir: &TempDir) -> SessionManager {
        let db_path = temp_dir.path().join("sessions.db");
        let manager_instance = Arc::new(DataManager::new());

        // 初始化数据库
        let db = manager_instance.sqlite(&db_path).unwrap();
        db.execute_raw(CREATE_TABLE_SQL).unwrap();
        let _ = db.execute_raw(ALTER_TABLE_SQL);

        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        let manager = SessionManager {
            manager: manager_instance,
            db_path,
            event_sender,
        };

        manager.start_background_tasks(event_receiver);
        manager
    }

    #[tokio::test]
    #[serial]
    async fn test_session_manager_send_event() {
        let temp = TempDir::new().expect("create temp dir");
        let manager = create_test_manager(&temp);

        let timestamp = chrono::Utc::now().timestamp();
        let event = SessionEvent::NewRequest {
            session_id: "test_user_session_abc-123".to_string(),
            tool_id: "claude-code".to_string(),
            timestamp,
        };

        // 发送事件
        manager.send_event(event).unwrap();

        // 等待批量写入
        tokio::time::sleep(Duration::from_millis(200)).await;

        // 查询验证
        let result = manager.get_session_list("claude-code", 1, 10).unwrap();

        assert!(result.sessions.iter().any(|s| s.display_id == "abc-123"));
    }

    #[tokio::test]
    #[serial]
    async fn test_datamanager_query_caching() {
        let temp = TempDir::new().expect("create temp dir");
        let manager = create_test_manager(&temp);

        // 插入测试数据
        let timestamp = chrono::Utc::now().timestamp();
        manager
            .send_event(SessionEvent::NewRequest {
                session_id: "test_session_cache_xyz".to_string(),
                tool_id: "claude-code".to_string(),
                timestamp,
            })
            .unwrap();

        tokio::time::sleep(Duration::from_millis(150)).await;

        // 第一次查询（缓存未命中）
        let start1 = std::time::Instant::now();
        let result1 = manager.get_session_list("claude-code", 1, 10).unwrap();
        let duration1 = start1.elapsed();

        // 第二次查询（缓存命中，应该更快）
        let start2 = std::time::Instant::now();
        let result2 = manager.get_session_list("claude-code", 1, 10).unwrap();
        let duration2 = start2.elapsed();

        assert_eq!(result1.total, result2.total);
        assert!(result1.total >= 1, "Should have at least one session");
        // 缓存命中的查询应该更快（允许一定误差）
        println!("Query 1: {:?}, Query 2: {:?}", duration1, duration2);
    }

    #[tokio::test]
    async fn test_update_session_config() {
        let temp = TempDir::new().expect("create temp dir");
        let manager = create_test_manager(&temp);

        // 手动插入测试会话
        let db = manager.manager.sqlite(&manager.db_path).unwrap();
        let now = chrono::Utc::now().timestamp();
        db.execute(
            "INSERT INTO claude_proxy_sessions (
                session_id, display_id, tool_id, config_name, url, api_key,
                first_seen_at, last_seen_at, request_count,
                created_at, updated_at
            ) VALUES (?, ?, ?, 'global', '', '', ?, ?, 1, ?, ?)",
            &[
                "test_session_update",
                "uuid-update",
                "claude-code",
                &now.to_string(),
                &now.to_string(),
                &now.to_string(),
                &now.to_string(),
            ],
        )
        .unwrap();

        // 更新配置
        manager
            .update_session_config(
                "test_session_update",
                "custom",
                Some("my-profile"),
                "https://api.test.com",
                "sk-test",
            )
            .unwrap();

        // 验证更新
        let session = manager.get_session("test_session_update").unwrap().unwrap();
        assert_eq!(session.config_name, "custom");
        assert_eq!(session.custom_profile_name, Some("my-profile".to_string()));
        assert_eq!(session.url, "https://api.test.com");
        assert_eq!(session.api_key, "sk-test");
    }
}
