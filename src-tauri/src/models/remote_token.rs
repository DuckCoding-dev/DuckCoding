// Remote Token Models
//
// NEW API 远程令牌数据模型

use serde::{Deserialize, Serialize};

/// 远程令牌（从 NEW API 拉取，不本地持久化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteToken {
    /// 令牌 ID
    pub id: i64,
    /// 用户 ID
    #[serde(default)]
    pub user_id: i64,
    /// 令牌名称
    pub name: String,
    /// 令牌密钥
    pub key: String,
    /// 所属分组
    pub group: String,
    /// 剩余额度
    pub remain_quota: i64,
    /// 已使用额度
    #[serde(default)]
    pub used_quota: i64,
    /// 过期时间（Unix 时间戳，-1 表示永不过期）
    pub expired_time: i64,
    /// 状态（1=启用，2=禁用）
    pub status: i32,
    /// 是否无限额度
    pub unlimited_quota: bool,
    /// 是否启用模型限制
    #[serde(default)]
    pub model_limits_enabled: bool,
    /// 模型限制（逗号分隔的模型列表）
    #[serde(default)]
    pub model_limits: String,
    /// 允许的 IP 地址（逗号分隔）
    #[serde(default)]
    pub allow_ips: String,
    /// 是否支持跨分组重试
    #[serde(default)]
    pub cross_group_retry: bool,
    /// 创建时间（Unix 时间戳）
    pub created_time: i64,
    /// 最后访问时间（Unix 时间戳）
    #[serde(default)]
    pub accessed_time: i64,
}

/// 远程令牌分组（API 返回的分组信息）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteTokenGroupInfo {
    /// 分组描述
    pub desc: String,
    /// 倍率
    pub ratio: f64,
}

/// 远程令牌分组（前端展示用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteTokenGroup {
    /// 分组 ID（即分组名称）
    pub id: String,
    /// 分组描述
    pub desc: String,
    /// 倍率
    pub ratio: f64,
}

/// 创建远程令牌请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRemoteTokenRequest {
    /// 令牌名称
    pub name: String,
    /// 分组名称
    pub group: String,
    /// 初始额度（token，500000 = 基准值）
    pub remain_quota: i64,
    /// 是否无限额度
    pub unlimited_quota: bool,
    /// 过期时间（Unix 时间戳，-1 表示永不过期）
    pub expired_time: i64,
    /// 是否启用模型限制
    pub model_limits_enabled: bool,
    /// 模型限制（逗号分隔的模型列表）
    pub model_limits: String,
    /// 允许的 IP 地址（逗号分隔）
    pub allow_ips: String,
}

/// 更新远程令牌请求（支持完整字段更新）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRemoteTokenRequest {
    /// 令牌名称
    pub name: String,
    /// 分组名称
    pub group: String,
    /// 剩余额度（token，500000 = 基准值）
    pub remain_quota: i64,
    /// 是否无限额度
    pub unlimited_quota: bool,
    /// 过期时间（Unix 时间戳，-1 表示永不过期）
    pub expired_time: i64,
    /// 是否启用模型限制
    pub model_limits_enabled: bool,
    /// 模型限制（逗号分隔的模型列表）
    pub model_limits: String,
    /// 允许的 IP 地址（换行符分隔，支持 CIDR 表达式）
    pub allow_ips: String,
}

/// NEW API 通用响应结构
#[derive(Debug, Deserialize)]
pub struct NewApiResponse<T> {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<T>,
}

/// NEW API 令牌列表响应的 data 部分
#[derive(Debug, Deserialize)]
pub struct TokenListData {
    pub page: i32,
    pub page_size: i32,
    pub total: i32,
    pub items: Vec<RemoteToken>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_token_serialization() {
        let token = RemoteToken {
            id: 123,
            user_id: 2703,
            name: "Test Token".to_string(),
            key: "sk-test123".to_string(),
            group: "default".to_string(),
            remain_quota: 100,
            used_quota: 50,
            expired_time: 1735200000,
            status: 1,
            unlimited_quota: false,
            model_limits_enabled: false,
            model_limits: String::new(),
            allow_ips: String::new(),
            cross_group_retry: false,
            created_time: 1704067200,
            accessed_time: 1704067200,
        };

        let json = serde_json::to_string(&token).unwrap();
        let deserialized: RemoteToken = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, token.id);
        assert_eq!(deserialized.name, token.name);
        assert_eq!(deserialized.key, token.key);
    }

    #[test]
    fn test_create_request_serialization() {
        let request = CreateRemoteTokenRequest {
            name: "New Token".to_string(),
            group: "group1".to_string(),
            remain_quota: 500000,
            unlimited_quota: false,
            expired_time: 1735200000,
            model_limits_enabled: false,
            model_limits: String::new(),
            allow_ips: String::new(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"name\":\"New Token\""));
        assert!(json.contains("\"unlimited_quota\":false"));
        assert!(json.contains("\"remain_quota\":500000"));
        assert!(json.contains("\"group\":\"group1\""));
    }
}
