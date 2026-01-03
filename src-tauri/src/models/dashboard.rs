// Dashboard Configuration Models
//
// 仪表板状态数据模型

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 仪表板配置存储
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStore {
    /// 数据版本
    pub version: u32,
    /// 工具实例选择记录（key: tool_id, value: instance_id）
    /// 例如：{"claude-code": "claude-code-local", "codex": "codex-wsl-Ubuntu"}
    pub tool_instance_selections: HashMap<String, String>,
    /// 最后选中的供应商 ID
    pub selected_provider_id: Option<String>,
    /// 最后更新时间（Unix 时间戳）
    pub updated_at: i64,
}

impl Default for DashboardStore {
    fn default() -> Self {
        Self {
            version: 1,
            tool_instance_selections: HashMap::new(),
            selected_provider_id: None,
            updated_at: chrono::Utc::now().timestamp(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_dashboard_store() {
        let store = DashboardStore::default();
        assert_eq!(store.version, 1);
        assert!(store.tool_instance_selections.is_empty());
        assert!(store.selected_provider_id.is_none());
        assert!(store.updated_at > 0);
    }

    #[test]
    fn test_dashboard_store_serialization() {
        let mut selections = HashMap::new();
        selections.insert("claude-code".to_string(), "claude-code-local".to_string());
        selections.insert("codex".to_string(), "codex-wsl-Ubuntu".to_string());

        let store = DashboardStore {
            version: 1,
            tool_instance_selections: selections,
            selected_provider_id: Some("duckcoding".to_string()),
            updated_at: 1234567890,
        };

        let json = serde_json::to_string(&store).unwrap();
        let deserialized: DashboardStore = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.version, 1);
        assert_eq!(deserialized.tool_instance_selections.len(), 2);
        assert_eq!(
            deserialized.tool_instance_selections.get("claude-code"),
            Some(&"claude-code-local".to_string())
        );
        assert_eq!(
            deserialized.selected_provider_id,
            Some("duckcoding".to_string())
        );
        assert_eq!(deserialized.updated_at, 1234567890);
    }
}
