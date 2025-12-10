// Balance 监控数据模型
//
// 余额监控配置的持久化存储结构

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 余额监控配置项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceConfig {
    /// 配置 ID
    pub id: String,
    /// 配置名称
    pub name: String,
    /// API 端点 URL
    pub endpoint: String,
    /// HTTP 方法（GET | POST）
    pub method: String,
    /// 静态请求头（持久化）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static_headers: Option<HashMap<String, String>>,
    /// 提取器 JavaScript 代码
    pub extractor_script: String,
    /// 自动刷新间隔（秒），0 或 None 表示不自动刷新
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval_sec: Option<u32>,
    /// 请求超时（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    /// 是否保存 API Key 到文件
    #[serde(default)]
    pub save_api_key: bool,
    /// API Key（可选，明文存储）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// 创建时间（Unix 时间戳，毫秒）
    pub created_at: i64,
    /// 更新时间（Unix 时间戳，毫秒）
    pub updated_at: i64,
}

/// 余额监控存储结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceStore {
    /// 存储格式版本
    pub version: u32,
    /// 所有配置列表
    pub configs: Vec<BalanceConfig>,
}

impl Default for BalanceStore {
    fn default() -> Self {
        Self {
            version: 1,
            configs: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balance_config_serialization() {
        let config = BalanceConfig {
            id: "test-id".to_string(),
            name: "Test Config".to_string(),
            endpoint: "https://api.example.com/balance".to_string(),
            method: "GET".to_string(),
            static_headers: Some(HashMap::from([(
                "Authorization".to_string(),
                "Bearer token".to_string(),
            )])),
            extractor_script: "return response.balance;".to_string(),
            interval_sec: Some(300),
            timeout_ms: Some(5000),
            save_api_key: false,
            api_key: None,
            created_at: 1234567890000,
            updated_at: 1234567890000,
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: BalanceConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.id, deserialized.id);
        assert_eq!(config.name, deserialized.name);
        assert_eq!(config.save_api_key, deserialized.save_api_key);
    }

    #[test]
    fn test_balance_store_default() {
        let store = BalanceStore::default();
        assert_eq!(store.version, 1);
        assert_eq!(store.configs.len(), 0);
    }
}
