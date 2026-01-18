// NEW API Client
//
// NEW API 客户端服务，用于与供应商的 API 交互

use crate::models::provider::Provider;
use crate::models::remote_token::{
    CreateRemoteTokenRequest, NewApiResponse, RemoteToken, RemoteTokenGroup, RemoteTokenGroupInfo,
    TokenListData, UpdateRemoteTokenRequest,
};
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;

/// NEW API 客户端
pub struct NewApiClient {
    provider: Provider,
    client: Client,
}

impl NewApiClient {
    /// 创建新的 NEW API 客户端
    pub fn new(provider: Provider) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| anyhow!("创建 HTTP 客户端失败: {}", e))?;

        Ok(Self { provider, client })
    }

    /// 获取基础 URL
    fn base_url(&self) -> String {
        self.provider.website_url.trim_end_matches('/').to_string()
    }

    /// 构建请求头
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", self.provider.access_token)
                .parse()
                .unwrap(),
        );
        headers.insert("New-Api-User", self.provider.user_id.parse().unwrap());
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers
    }

    /// 获取远程令牌列表（支持分页）
    pub async fn list_tokens(&self, page: i32, page_size: i32) -> Result<TokenListData> {
        let url = format!(
            "{}/api/token?p={}&page_size={}",
            self.base_url(),
            page,
            page_size
        );
        let response = self
            .client
            .get(&url)
            .headers(self.build_headers())
            .send()
            .await
            .map_err(|e| anyhow!("请求失败: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "API 请求失败，状态码: {}",
                response.status().as_u16()
            ));
        }

        let api_response: NewApiResponse<TokenListData> = response
            .json()
            .await
            .map_err(|e| anyhow!("解析响应失败: {}", e))?;

        if !api_response.success {
            return Err(anyhow!(
                "API 返回错误: {}",
                api_response
                    .message
                    .unwrap_or_else(|| "未知错误".to_string())
            ));
        }

        // 标准化 API Key，确保所有令牌都有 sk- 前缀
        let mut data = api_response.data.unwrap_or_else(|| TokenListData {
            page,
            page_size,
            total: 0,
            items: vec![],
        });
        for token in &mut data.items {
            if !token.key.starts_with("sk-") {
                token.key = format!("sk-{}", token.key);
            }
        }

        Ok(data)
    }

    /// 获取所有令牌分组
    pub async fn list_groups(&self) -> Result<Vec<RemoteTokenGroup>> {
        let url = format!("{}/api/user/self/groups", self.base_url());
        let response = self
            .client
            .get(&url)
            .headers(self.build_headers())
            .send()
            .await
            .map_err(|e| anyhow!("请求失败: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "API 请求失败，状态码: {}",
                response.status().as_u16()
            ));
        }

        let api_response: NewApiResponse<HashMap<String, RemoteTokenGroupInfo>> = response
            .json()
            .await
            .map_err(|e| anyhow!("解析响应失败: {}", e))?;

        if !api_response.success {
            return Err(anyhow!(
                "API 返回错误: {}",
                api_response
                    .message
                    .unwrap_or_else(|| "未知错误".to_string())
            ));
        }

        // 将 HashMap 转换为 Vec<RemoteTokenGroup>
        let groups = api_response
            .data
            .unwrap_or_default()
            .into_iter()
            .map(|(id, info)| RemoteTokenGroup {
                id,
                desc: info.desc,
                ratio: info.ratio,
            })
            .collect();

        Ok(groups)
    }

    /// 创建新的远程令牌（返回值仅包含成功状态，不返回令牌对象）
    pub async fn create_token(&self, request: CreateRemoteTokenRequest) -> Result<()> {
        let url = format!("{}/api/token", self.base_url());

        // 构建请求体（所有字段都是必需的）
        let body = json!({
            "name": request.name,
            "group": request.group,
            "remain_quota": request.remain_quota,
            "unlimited_quota": request.unlimited_quota,
            "expired_time": request.expired_time,
            "model_limits_enabled": request.model_limits_enabled,
            "model_limits": request.model_limits,
            "allow_ips": request.allow_ips,
        });

        let response = self
            .client
            .post(&url)
            .headers(self.build_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("请求失败: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "API 请求失败，状态码: {}",
                response.status().as_u16()
            ));
        }

        // API 只返回 { success: true, message: "" }，不返回令牌对象
        let api_response: NewApiResponse<()> = response
            .json()
            .await
            .map_err(|e| anyhow!("解析响应失败: {}", e))?;

        if !api_response.success {
            return Err(anyhow!(
                "API 返回错误: {}",
                api_response
                    .message
                    .unwrap_or_else(|| "未知错误".to_string())
            ));
        }

        Ok(())
    }

    /// 删除远程令牌
    pub async fn delete_token(&self, token_id: i64) -> Result<()> {
        let url = format!("{}/api/token/{}", self.base_url(), token_id);
        let response = self
            .client
            .delete(&url)
            .headers(self.build_headers())
            .send()
            .await
            .map_err(|e| anyhow!("请求失败: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "API 请求失败，状态码: {}",
                response.status().as_u16()
            ));
        }

        let api_response: NewApiResponse<()> = response
            .json()
            .await
            .map_err(|e| anyhow!("解析响应失败: {}", e))?;

        if !api_response.success {
            return Err(anyhow!(
                "API 返回错误: {}",
                api_response
                    .message
                    .unwrap_or_else(|| "未知错误".to_string())
            ));
        }

        Ok(())
    }

    /// 更新远程令牌信息（仅名称）
    pub async fn update_token(&self, token_id: i64, name: String) -> Result<RemoteToken> {
        let url = format!("{}/api/token/{}", self.base_url(), token_id);
        let body = json!({
            "name": name,
        });

        let response = self
            .client
            .patch(&url)
            .headers(self.build_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("请求失败: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "API 请求失败，状态码: {}",
                response.status().as_u16()
            ));
        }

        let api_response: NewApiResponse<RemoteToken> = response
            .json()
            .await
            .map_err(|e| anyhow!("解析响应失败: {}", e))?;

        if !api_response.success {
            return Err(anyhow!(
                "API 返回错误: {}",
                api_response
                    .message
                    .unwrap_or_else(|| "未知错误".to_string())
            ));
        }

        api_response
            .data
            .ok_or_else(|| anyhow!("API 未返回令牌数据"))
    }

    /// 更新远程令牌信息（完整版本，支持所有字段）
    pub async fn update_token_full(
        &self,
        token_id: i64,
        request: UpdateRemoteTokenRequest,
    ) -> Result<RemoteToken> {
        let url = format!("{}/api/token/{}", self.base_url(), token_id);

        // 构建请求体（所有字段）
        let body = json!({
            "name": request.name,
            "group": request.group,
            "remain_quota": request.remain_quota,
            "unlimited_quota": request.unlimited_quota,
            "expired_time": request.expired_time,
            "model_limits_enabled": request.model_limits_enabled,
            "model_limits": request.model_limits,
            "allow_ips": request.allow_ips,
        });

        let response = self
            .client
            .patch(&url)
            .headers(self.build_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("请求失败: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "API 请求失败，状态码: {}",
                response.status().as_u16()
            ));
        }

        let api_response: NewApiResponse<RemoteToken> = response
            .json()
            .await
            .map_err(|e| anyhow!("解析响应失败: {}", e))?;

        if !api_response.success {
            return Err(anyhow!(
                "API 返回错误: {}",
                api_response
                    .message
                    .unwrap_or_else(|| "未知错误".to_string())
            ));
        }

        api_response
            .data
            .ok_or_else(|| anyhow!("API 未返回令牌数据"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let provider = Provider {
            id: "test".to_string(),
            name: "Test Provider".to_string(),
            website_url: "https://test.com".to_string(),
            api_address: None,
            user_id: "123".to_string(),
            access_token: "token123".to_string(),
            username: None,
            is_default: false,
            created_at: 0,
            updated_at: 0,
        };

        let client = NewApiClient::new(provider);
        assert!(client.is_ok());
    }

    #[test]
    fn test_base_url() {
        let provider = Provider {
            id: "test".to_string(),
            name: "Test Provider".to_string(),
            website_url: "https://test.com/".to_string(),
            api_address: None,
            user_id: "123".to_string(),
            access_token: "token123".to_string(),
            username: None,
            is_default: false,
            created_at: 0,
            updated_at: 0,
        };

        let client = NewApiClient::new(provider).unwrap();
        assert_eq!(client.base_url(), "https://test.com");
    }
}
