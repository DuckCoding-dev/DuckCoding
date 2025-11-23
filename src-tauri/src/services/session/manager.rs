// SessionManager 单例 - 会话管理核心模块

use crate::services::session::db::SessionDatabase;
use crate::services::session::models::{ProxySession, SessionEvent, SessionListResponse};
use anyhow::Result;
use lazy_static::lazy_static;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

/// 会话管理器单例
pub struct SessionManager {
    db: Arc<SessionDatabase>,
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
        let db = Arc::new(SessionDatabase::new(db_path)?);

        // 创建事件队列
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        let manager = Self { db, event_sender };

        // 启动后台任务
        manager.start_background_tasks(event_receiver);

        Ok(manager)
    }

    /// 获取数据库路径
    fn get_db_path() -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Home directory not found")
        })?;
        Ok(home.join(".duckcoding").join("sessions.db"))
    }

    /// 启动后台任务
    fn start_background_tasks(&self, mut event_receiver: mpsc::UnboundedReceiver<SessionEvent>) {
        let db = Arc::clone(&self.db);

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
                            Self::flush_events(&db, &mut buffer);
                        }
                    }
                    // 每 100ms 刷新一次
                    _ = tick_interval.tick() => {
                        if !buffer.is_empty() {
                            Self::flush_events(&db, &mut buffer);
                        }
                    }
                }
            }
        });

        // 定期清理任务（每 1 小时）
        let db_clone = Arc::clone(&self.db);
        tokio::spawn(async move {
            let mut cleanup_interval = interval(Duration::from_secs(3600));

            loop {
                cleanup_interval.tick().await;

                // 清理三个工具的过期会话
                for tool_id in &["claude-code", "codex", "gemini-cli"] {
                    let _ = db_clone.cleanup_old_sessions(tool_id, 1000, 30);
                }
            }
        });
    }

    /// 批量写入事件到数据库
    fn flush_events(db: &SessionDatabase, buffer: &mut Vec<SessionEvent>) {
        for event in buffer.drain(..) {
            match event {
                SessionEvent::NewRequest {
                    session_id,
                    tool_id,
                    timestamp,
                } => {
                    // 提取 display_id
                    if let Some(display_id) = ProxySession::extract_display_id(&session_id) {
                        let _ = db.upsert_session(&session_id, &display_id, &tool_id, timestamp);
                    }
                }
            }
        }
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
        self.db.get_sessions(tool_id, page, page_size)
    }

    /// 删除单个会话（公共 API）
    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        self.db.delete_session(session_id)
    }

    /// 清空工具所有会话（公共 API）
    pub fn clear_sessions(&self, tool_id: &str) -> Result<()> {
        self.db.clear_sessions(tool_id)
    }

    /// 获取会话详情（公共 API）
    pub fn get_session(&self, session_id: &str) -> Result<Option<ProxySession>> {
        self.db.get_session(session_id)
    }

    /// 获取会话配置（公共 API，用于请求处理）
    /// 返回 (config_name, url, api_key)
    pub fn get_session_config(&self, session_id: &str) -> Result<Option<(String, String, String)>> {
        self.db.get_session_config(session_id)
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
        self.db
            .update_session_config(session_id, config_name, custom_profile_name, url, api_key)
    }

    /// 更新会话备注（公共 API）
    pub fn update_session_note(&self, session_id: &str, note: Option<&str>) -> Result<()> {
        self.db.update_session_note(session_id, note)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_manager_send_event() {
        let timestamp = chrono::Utc::now().timestamp();
        let event = SessionEvent::NewRequest {
            session_id: "test_user_session_abc-123".to_string(),
            tool_id: "claude-code".to_string(),
            timestamp,
        };

        // 发送事件
        SESSION_MANAGER.send_event(event).unwrap();

        // 等待批量写入
        tokio::time::sleep(Duration::from_millis(200)).await;

        // 查询验证
        let result = SESSION_MANAGER
            .get_session_list("claude-code", 1, 10)
            .unwrap();

        assert!(result.sessions.iter().any(|s| s.display_id == "abc-123"));
    }
}
