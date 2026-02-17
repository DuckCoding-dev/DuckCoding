// Checkin Service
//
// 供应商签到服务

use crate::models::provider::{CheckinConfig, Provider};
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckinResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<CheckinData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckinData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_awarded: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkin_date: Option<String>,
}

/// 执行签到
pub async fn perform_checkin(provider: &Provider) -> Result<CheckinResponse> {
    let config = provider
        .checkin_config
        .as_ref()
        .ok_or_else(|| anyhow!("签到配置不存在"))?;

    let base_url = provider
        .api_address
        .as_ref()
        .unwrap_or(&provider.website_url);
    let url = format!(
        "{}{}",
        base_url.trim_end_matches('/'),
        config.endpoint
    );

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", provider.access_token))
        .header("New-Api-User", &provider.user_id)
        .header("Content-Type", "application/json")
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow!("签到失败 ({}): {}", status, error_text));
    }

    let result: CheckinResponse = response.json().await?;
    Ok(result)
}

/// 检查是否需要签到
pub fn should_checkin(config: &CheckinConfig) -> bool {
    if !config.enabled {
        return false;
    }

    // 检查今天是否已签到
    if let Some(last_checkin) = config.last_checkin_at {
        let now = chrono::Local::now();
        let last = chrono::DateTime::<chrono::Local>::from(
            chrono::DateTime::<chrono::Utc>::from_timestamp(last_checkin, 0)
                .unwrap_or_default(),
        );

        // 如果是同一天,不需要签到
        if now.date_naive() == last.date_naive() {
            return false;
        }
    }

    // 检查当前时间是否到达签到时间
    let now = chrono::Local::now();
    let current_hour = now.hour() as u8;

    // 如果当前时间大于等于设定的签到时间,则需要签到
    current_hour >= config.checkin_hour
}
