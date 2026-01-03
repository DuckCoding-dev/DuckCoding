// Provider Configuration Models
//
// 供应商配置数据模型

use serde::{Deserialize, Serialize};

/// 供应商配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    /// 唯一标识
    pub id: String,
    /// 供应商名称（如 DuckCoding）
    pub name: String,
    /// 官网地址
    pub website_url: String,
    /// 用户ID
    pub user_id: String,
    /// 系统访问令牌
    pub access_token: String,
    /// 用户名（可选，用于确认）
    pub username: Option<String>,
    /// 是否为默认供应商
    pub is_default: bool,
    /// 创建时间
    pub created_at: i64,
    /// 更新时间
    pub updated_at: i64,
}

/// 供应商存储结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStore {
    /// 数据版本
    pub version: u32,
    /// 供应商列表
    pub providers: Vec<Provider>,
    /// 最后更新时间
    pub updated_at: i64,
}

impl Default for ProviderStore {
    fn default() -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            version: 1,
            providers: vec![Provider {
                id: "duckcoding".to_string(),
                name: "DuckCoding".to_string(),
                website_url: "https://duckcoding.com".to_string(),
                user_id: String::new(),
                access_token: String::new(),
                username: None,
                is_default: true,
                created_at: now,
                updated_at: now,
            }],
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_provider_store() {
        let store = ProviderStore::default();
        assert_eq!(store.version, 1);
        assert_eq!(store.providers.len(), 1);
        assert_eq!(store.providers[0].id, "duckcoding");
        assert_eq!(store.providers[0].name, "DuckCoding");
        assert!(store.providers[0].is_default);
    }

    #[test]
    fn test_provider_serialization() {
        let provider = Provider {
            id: "test".to_string(),
            name: "Test Provider".to_string(),
            website_url: "https://test.com".to_string(),
            user_id: "12345".to_string(),
            access_token: "token123".to_string(),
            username: Some("testuser".to_string()),
            is_default: false,
            created_at: 1234567890,
            updated_at: 1234567890,
        };

        let json = serde_json::to_string(&provider).unwrap();
        let deserialized: Provider = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, provider.id);
        assert_eq!(deserialized.name, provider.name);
        assert_eq!(deserialized.username, provider.username);
    }
}
