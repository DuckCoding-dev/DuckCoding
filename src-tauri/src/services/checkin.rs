// Checkin Service
//
// 供应商签到服务：执行签到、随机时间调度、重试逻辑

use crate::models::provider::{CheckinConfig, Provider};
use anyhow::{anyhow, Result};
use chrono::{Local, NaiveDate, NaiveTime, TimeZone, Timelike};
use rand::Rng;
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
    let url = format!("{}{}", base_url.trim_end_matches('/'), config.endpoint);

    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

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

/// 检查是否需要签到（基于 next_checkin_at 时间戳）
pub fn should_checkin(config: &CheckinConfig) -> bool {
    if !config.enabled {
        return false;
    }

    // 今天已签到则跳过
    if checked_in_today(config) {
        return false;
    }

    // 检查是否到达计划签到时间
    if let Some(next_at) = config.next_checkin_at {
        let now = chrono::Utc::now().timestamp();
        return now >= next_at;
    }

    // 无计划时间，由调度器生成
    false
}

/// 检查是否需要为今天生成签到计划
pub fn needs_schedule(config: &CheckinConfig) -> bool {
    config.enabled && !checked_in_today(config) && config.next_checkin_at.is_none()
}

/// 检查今天是否已签到
fn checked_in_today(config: &CheckinConfig) -> bool {
    if let Some(last_checkin) = config.last_checkin_at {
        let last = chrono::DateTime::<chrono::Utc>::from_timestamp(last_checkin, 0)
            .unwrap_or_default()
            .with_timezone(&Local);
        let today = Local::now().date_naive();
        return last.date_naive() == today;
    }
    false
}

/// 在配置的时间范围内为指定日期生成随机签到时间戳
pub fn generate_checkin_time(config: &CheckinConfig, date: NaiveDate) -> i64 {
    let (start_hour, end_hour) = config.effective_range();
    let mut rng = rand::thread_rng();

    let hour = rng.gen_range(start_hour as u32..=end_hour as u32);
    let minute = rng.gen_range(0..60u32);

    let time = NaiveTime::from_hms_opt(hour, minute, 0).unwrap_or_default();
    let datetime = date.and_time(time);

    Local
        .from_local_datetime(&datetime)
        .single()
        .map(|dt| dt.timestamp())
        .unwrap_or_else(|| {
            // 时区转换失败时使用 UTC
            datetime.and_utc().timestamp()
        })
}

/// 在当天剩余范围内生成重试时间（距当前至少 10 分钟）
/// 范围不足时返回 None（今天不再重试，明天再来）
pub fn generate_retry_time(config: &CheckinConfig) -> Option<i64> {
    let now = Local::now();
    let (_, end_hour) = config.effective_range();

    // 最早重试时间：当前时间 + 10 分钟
    let min_retry = now + chrono::Duration::minutes(10);
    let min_hour = min_retry.hour();
    let min_minute = min_retry.minute();

    // 范围结束时间为 end_hour:59
    // 如果最早重试时间已超过范围结束，返回 None
    if min_hour > end_hour as u32 || (min_hour == end_hour as u32 && min_minute > 59) {
        return None;
    }

    let mut rng = rand::thread_rng();

    // 在 min_retry 到 end_hour:59 之间随机选取
    let start_minutes = min_hour * 60 + min_minute;
    let end_minutes = end_hour as u32 * 60 + 59;

    if start_minutes >= end_minutes {
        return None;
    }

    let random_minutes = rng.gen_range(start_minutes..=end_minutes);
    let retry_hour = random_minutes / 60;
    let retry_minute = random_minutes % 60;

    let date = now.date_naive();
    let time = NaiveTime::from_hms_opt(retry_hour, retry_minute, 0)?;
    let datetime = date.and_time(time);

    Local
        .from_local_datetime(&datetime)
        .single()
        .map(|dt| dt.timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(enabled: bool, start: u8, end: u8) -> CheckinConfig {
        CheckinConfig {
            enabled,
            endpoint: "/api/user/checkin".to_string(),
            checkin_hour_start: start,
            checkin_hour_end: end,
            next_checkin_at: None,
            last_checkin_at: None,
            last_checkin_status: None,
            last_checkin_message: None,
            total_checkins: 0,
            total_quota: 0,
        }
    }

    #[test]
    fn test_should_checkin_disabled() {
        let config = make_config(false, 0, 0);
        assert!(!should_checkin(&config));
    }

    #[test]
    fn test_should_checkin_no_schedule() {
        let config = make_config(true, 0, 0);
        // 无 next_checkin_at，应返回 false
        assert!(!should_checkin(&config));
    }

    #[test]
    fn test_should_checkin_with_past_schedule() {
        let mut config = make_config(true, 0, 0);
        // 设置过去的时间
        config.next_checkin_at = Some(chrono::Utc::now().timestamp() - 100);
        assert!(should_checkin(&config));
    }

    #[test]
    fn test_should_checkin_with_future_schedule() {
        let mut config = make_config(true, 0, 0);
        // 设置未来的时间
        config.next_checkin_at = Some(chrono::Utc::now().timestamp() + 3600);
        assert!(!should_checkin(&config));
    }

    #[test]
    fn test_needs_schedule() {
        let config = make_config(true, 0, 0);
        assert!(needs_schedule(&config));

        let disabled = make_config(false, 0, 0);
        assert!(!needs_schedule(&disabled));

        let mut scheduled = make_config(true, 0, 0);
        scheduled.next_checkin_at = Some(12345);
        assert!(!needs_schedule(&scheduled));
    }

    #[test]
    fn test_generate_checkin_time_in_range() {
        let config = make_config(true, 9, 12);
        let date = Local::now().date_naive();

        // 生成 100 次，确保都在范围内
        for _ in 0..100 {
            let ts = generate_checkin_time(&config, date);
            let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0)
                .unwrap()
                .with_timezone(&Local);
            let hour = dt.hour();
            assert!((9..=12).contains(&hour), "hour {} not in range 9-12", hour);
        }
    }

    #[test]
    fn test_generate_checkin_time_full_day() {
        let config = make_config(true, 0, 0); // start == end → 全天
        let date = Local::now().date_naive();
        let ts = generate_checkin_time(&config, date);
        let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0)
            .unwrap()
            .with_timezone(&Local);
        assert!((0..=23).contains(&dt.hour()));
    }

    #[test]
    fn test_effective_range() {
        let config = make_config(true, 9, 12);
        assert_eq!(config.effective_range(), (9, 12));

        let full_day = make_config(true, 0, 0);
        assert_eq!(full_day.effective_range(), (0, 23));

        let reversed = make_config(true, 12, 9);
        assert_eq!(reversed.effective_range(), (0, 23));
    }
}
