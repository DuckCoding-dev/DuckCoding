//! 数据库查询工具模块
//!
//! 提供 QueryRow ↔ ProxySession 转换逻辑，用于 SessionManager 与 DataManager 的适配层。

use crate::data::managers::sqlite::QueryRow;
use crate::services::session::models::ProxySession;
use anyhow::{anyhow, Context, Result};

/// 标准会话查询的 SQL 语句
///
/// **字段顺序（共 13 个）：**
/// 1. session_id
/// 2. display_id
/// 3. tool_id
/// 4. config_name
/// 5. custom_profile_name
/// 6. url
/// 7. api_key
/// 8. note
/// 9. first_seen_at
/// 10. last_seen_at
/// 11. request_count
/// 12. created_at
/// 13. updated_at
pub const SELECT_SESSION_FIELDS: &str = "session_id, display_id, tool_id, config_name, \
                                          custom_profile_name, url, api_key, note, \
                                          first_seen_at, last_seen_at, request_count, \
                                          created_at, updated_at";

/// 创建表的 SQL 语句
pub const CREATE_TABLE_SQL: &str = "
CREATE TABLE IF NOT EXISTS claude_proxy_sessions (
    session_id TEXT PRIMARY KEY,
    display_id TEXT NOT NULL,
    tool_id TEXT NOT NULL,
    config_name TEXT NOT NULL DEFAULT 'global',
    custom_profile_name TEXT,
    url TEXT NOT NULL,
    api_key TEXT NOT NULL,
    note TEXT,
    first_seen_at INTEGER NOT NULL,
    last_seen_at INTEGER NOT NULL,
    request_count INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tool_id ON claude_proxy_sessions(tool_id);
CREATE INDEX IF NOT EXISTS idx_display_id ON claude_proxy_sessions(display_id);
CREATE INDEX IF NOT EXISTS idx_last_seen_at ON claude_proxy_sessions(last_seen_at);
";

/// 兼容旧数据库的字段添加语句
pub const ALTER_TABLE_SQL: &str = "
ALTER TABLE claude_proxy_sessions ADD COLUMN custom_profile_name TEXT;
ALTER TABLE claude_proxy_sessions ADD COLUMN note TEXT;
";

/// 从 QueryRow 解析为 ProxySession
///
/// # 参数
///
/// - `row`: SqliteManager 返回的查询行
///
/// # 返回
///
/// 成功返回 ProxySession 实例，失败返回错误信息
///
/// # 字段映射
///
/// 依赖 `SELECT_SESSION_FIELDS` 定义的顺序：
/// - values[0..7]: 字符串字段
/// - values[7]: note (可为 NULL)
/// - values[8..12]: 整数字段
pub fn parse_proxy_session(row: &QueryRow) -> Result<ProxySession> {
    if row.values.len() != 13 {
        return Err(anyhow!(
            "Invalid row: expected 13 columns, got {}",
            row.values.len()
        ));
    }

    // 字段提取辅助函数
    let get_string = |idx: usize| -> Result<String> {
        row.values[idx]
            .as_str()
            .ok_or_else(|| anyhow!("Column {} is not a string", idx))
            .map(|s| s.to_string())
    };

    let get_optional_string =
        |idx: usize| -> Option<String> { row.values[idx].as_str().map(|s| s.to_string()) };

    let get_i64 = |idx: usize| -> Result<i64> {
        row.values[idx]
            .as_i64()
            .ok_or_else(|| anyhow!("Column {} is not an integer", idx))
    };

    let get_i32 = |idx: usize| -> Result<i32> {
        row.values[idx]
            .as_i64()
            .ok_or_else(|| anyhow!("Column {} is not an integer", idx))
            .map(|v| v as i32)
    };

    Ok(ProxySession {
        session_id: get_string(0).context("session_id")?,
        display_id: get_string(1).context("display_id")?,
        tool_id: get_string(2).context("tool_id")?,
        config_name: get_string(3).context("config_name")?,
        custom_profile_name: get_optional_string(4),
        url: get_string(5).context("url")?,
        api_key: get_string(6).context("api_key")?,
        note: get_optional_string(7),
        first_seen_at: get_i64(8).context("first_seen_at")?,
        last_seen_at: get_i64(9).context("last_seen_at")?,
        request_count: get_i32(10).context("request_count")?,
        created_at: get_i64(11).context("created_at")?,
        updated_at: get_i64(12).context("updated_at")?,
    })
}

/// 从 QueryRow 提取计数值
///
/// # 参数
///
/// - `row`: 包含单个整数列的查询结果
///
/// # 示例
///
/// ```rust
/// let rows = db.query("SELECT COUNT(*) FROM sessions WHERE tool_id = ?", &["claude-code"])?;
/// let count = parse_count(&rows[0])?;
/// ```
pub fn parse_count(row: &QueryRow) -> Result<usize> {
    if row.values.is_empty() {
        return Err(anyhow!("Count query returned empty row"));
    }

    row.values[0]
        .as_i64()
        .ok_or_else(|| anyhow!("Count value is not an integer"))
        .map(|v| v as usize)
}

/// 从 QueryRow 提取三元组配置 (config_name, url, api_key)
///
/// 用于 `get_session_config()` 方法的结果解析
pub fn parse_session_config(row: &QueryRow) -> Result<(String, String, String)> {
    if row.values.len() != 3 {
        return Err(anyhow!(
            "Invalid config row: expected 3 columns, got {}",
            row.values.len()
        ));
    }

    let config_name = row.values[0]
        .as_str()
        .ok_or_else(|| anyhow!("config_name is not a string"))?
        .to_string();

    let url = row.values[1]
        .as_str()
        .ok_or_else(|| anyhow!("url is not a string"))?
        .to_string();

    let api_key = row.values[2]
        .as_str()
        .ok_or_else(|| anyhow!("api_key is not a string"))?
        .to_string();

    Ok((config_name, url, api_key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_proxy_session_full() {
        let row = QueryRow {
            columns: vec![
                "session_id".to_string(),
                "display_id".to_string(),
                "tool_id".to_string(),
                "config_name".to_string(),
                "custom_profile_name".to_string(),
                "url".to_string(),
                "api_key".to_string(),
                "note".to_string(),
                "first_seen_at".to_string(),
                "last_seen_at".to_string(),
                "request_count".to_string(),
                "created_at".to_string(),
                "updated_at".to_string(),
            ],
            values: vec![
                json!("test_session_1"),
                json!("uuid-1"),
                json!("claude-code"),
                json!("custom"),
                json!("my-profile"),
                json!("https://api.example.com"),
                json!("sk-test"),
                json!("测试备注"),
                json!(1000),
                json!(2000),
                json!(5),
                json!(1000),
                json!(2000),
            ],
        };

        let session = parse_proxy_session(&row).unwrap();

        assert_eq!(session.session_id, "test_session_1");
        assert_eq!(session.display_id, "uuid-1");
        assert_eq!(session.tool_id, "claude-code");
        assert_eq!(session.config_name, "custom");
        assert_eq!(session.custom_profile_name, Some("my-profile".to_string()));
        assert_eq!(session.url, "https://api.example.com");
        assert_eq!(session.api_key, "sk-test");
        assert_eq!(session.note, Some("测试备注".to_string()));
        assert_eq!(session.first_seen_at, 1000);
        assert_eq!(session.last_seen_at, 2000);
        assert_eq!(session.request_count, 5);
        assert_eq!(session.created_at, 1000);
        assert_eq!(session.updated_at, 2000);
    }

    #[test]
    fn test_parse_proxy_session_with_nulls() {
        let row = QueryRow {
            columns: vec![
                "session_id".to_string(),
                "display_id".to_string(),
                "tool_id".to_string(),
                "config_name".to_string(),
                "custom_profile_name".to_string(),
                "url".to_string(),
                "api_key".to_string(),
                "note".to_string(),
                "first_seen_at".to_string(),
                "last_seen_at".to_string(),
                "request_count".to_string(),
                "created_at".to_string(),
                "updated_at".to_string(),
            ],
            values: vec![
                json!("test_session_2"),
                json!("uuid-2"),
                json!("codex"),
                json!("global"),
                json!(null), // custom_profile_name
                json!(""),
                json!(""),
                json!(null), // note
                json!(3000),
                json!(4000),
                json!(10),
                json!(3000),
                json!(4000),
            ],
        };

        let session = parse_proxy_session(&row).unwrap();

        assert_eq!(session.session_id, "test_session_2");
        assert_eq!(session.config_name, "global");
        assert_eq!(session.custom_profile_name, None);
        assert_eq!(session.note, None);
        assert_eq!(session.request_count, 10);
    }

    #[test]
    fn test_parse_count() {
        let row = QueryRow {
            columns: vec!["COUNT(*)".to_string()],
            values: vec![json!(42)],
        };

        let count = parse_count(&row).unwrap();
        assert_eq!(count, 42);
    }

    #[test]
    fn test_parse_session_config() {
        let row = QueryRow {
            columns: vec![
                "config_name".to_string(),
                "url".to_string(),
                "api_key".to_string(),
            ],
            values: vec![
                json!("custom"),
                json!("https://api.test.com"),
                json!("sk-xxx"),
            ],
        };

        let (config_name, url, api_key) = parse_session_config(&row).unwrap();

        assert_eq!(config_name, "custom");
        assert_eq!(url, "https://api.test.com");
        assert_eq!(api_key, "sk-xxx");
    }

    #[test]
    fn test_parse_proxy_session_invalid_column_count() {
        let row = QueryRow {
            columns: vec!["session_id".to_string()],
            values: vec![json!("test")],
        };

        let result = parse_proxy_session(&row);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expected 13 columns"));
    }
}
