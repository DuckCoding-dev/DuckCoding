// Checkin Scheduler
//
// 签到定时任务调度器：每分钟检查，随机时间签到，失败自动重试

use crate::models::provider::Provider;
use crate::services::{checkin, provider_manager::ProviderManager};
use chrono::Local;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;

pub struct CheckinScheduler {
    provider_manager: Arc<RwLock<ProviderManager>>,
    running: Arc<RwLock<bool>>,
}

impl CheckinScheduler {
    pub fn new(provider_manager: Arc<RwLock<ProviderManager>>) -> Self {
        Self {
            provider_manager,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// 启动定时任务
    pub async fn start(&self) {
        let mut running = self.running.write().await;
        if *running {
            tracing::warn!("签到调度器已在运行");
            return;
        }
        *running = true;
        drop(running);

        let provider_manager = self.provider_manager.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            tracing::info!("签到调度器已启动（60秒间隔）");

            // 每分钟检查一次，支持分钟级随机时间
            let mut interval = time::interval(Duration::from_secs(60));

            loop {
                interval.tick().await;

                let is_running = *running.read().await;
                if !is_running {
                    tracing::info!("签到调度器已停止");
                    break;
                }

                // 执行签到检查
                if let Err(e) = Self::check_and_checkin(&provider_manager).await {
                    tracing::error!("签到检查失败: {}", e);
                }
            }
        });
    }

    /// 停止定时任务
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        tracing::info!("签到调度器停止中...");
    }

    /// 两阶段签到检查：调度 + 执行
    async fn check_and_checkin(
        provider_manager: &Arc<RwLock<ProviderManager>>,
    ) -> anyhow::Result<()> {
        let providers = {
            let manager = provider_manager.read().await;
            let all: Vec<Provider> = manager.list_providers()?;
            all.into_iter()
                .filter(|p| p.checkin_config.as_ref().is_some_and(|c| c.enabled))
                .collect::<Vec<_>>()
        };

        if providers.is_empty() {
            return Ok(());
        }

        // 阶段 1：为缺少计划时间的供应商生成随机签到时间
        for provider in &providers {
            if let Some(config) = &provider.checkin_config {
                if checkin::needs_schedule(config) {
                    let today = Local::now().date_naive();
                    let scheduled_time = checkin::generate_checkin_time(config, today);
                    let now = chrono::Utc::now().timestamp();

                    // 如果生成的时间已过，直接设为当前时间（立即执行）
                    let final_time = if scheduled_time < now {
                        now
                    } else {
                        scheduled_time
                    };

                    let mut updated = provider.clone();
                    if let Some(c) = &mut updated.checkin_config {
                        c.next_checkin_at = Some(final_time);
                    }

                    let manager = provider_manager.write().await;
                    if let Err(e) = manager.update_provider(&provider.id, updated) {
                        tracing::error!("保存签到计划失败 [{}]: {}", provider.name, e);
                    } else {
                        let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(final_time, 0)
                            .unwrap_or_default()
                            .with_timezone(&Local);
                        tracing::info!(
                            "已为供应商 {} 生成签到计划: {}",
                            provider.name,
                            dt.format("%H:%M")
                        );
                    }
                }
            }
        }

        // 重新加载供应商（阶段 1 可能已更新 next_checkin_at）
        let providers_to_checkin = {
            let manager = provider_manager.read().await;
            let all: Vec<Provider> = manager.list_providers()?;
            all.into_iter()
                .filter(|p| {
                    p.checkin_config
                        .as_ref()
                        .map(checkin::should_checkin)
                        .unwrap_or(false)
                })
                .collect::<Vec<_>>()
        };

        // 阶段 2：执行到期的签到
        for provider in providers_to_checkin {
            tracing::info!("开始为供应商 {} 执行自动签到", provider.name);

            match checkin::perform_checkin(&provider).await {
                Ok(response) => {
                    if response.success {
                        tracing::info!("供应商 {} 签到成功: {:?}", provider.name, response.message);

                        // 更新签到统计，清除 next_checkin_at
                        let mut updated = provider.clone();
                        if let Some(config) = &mut updated.checkin_config {
                            config.next_checkin_at = None;
                            config.last_checkin_at = Some(chrono::Utc::now().timestamp());
                            config.last_checkin_status = Some("success".to_string());
                            config.last_checkin_message = response.message.clone();
                            config.total_checkins += 1;
                            if let Some(data) = response.data {
                                if let Some(quota) = data.quota_awarded {
                                    config.total_quota += quota;
                                }
                            }
                        }

                        let manager = provider_manager.write().await;
                        if let Err(e) = manager.update_provider(&provider.id, updated) {
                            tracing::error!("更新签到统计失败 [{}]: {}", provider.name, e);
                        }
                    } else {
                        // API 返回失败，安排重试
                        tracing::warn!(
                            "供应商 {} 签到返回失败: {:?}，安排重试",
                            provider.name,
                            response.message
                        );
                        Self::schedule_retry(provider_manager, &provider).await;
                    }
                }
                Err(e) => {
                    // 请求异常，安排重试
                    tracing::error!("供应商 {} 签到请求失败: {}，安排重试", provider.name, e);
                    Self::schedule_retry(provider_manager, &provider).await;
                }
            }
        }

        Ok(())
    }

    /// 安排重试：在剩余范围内生成新的随机时间
    async fn schedule_retry(provider_manager: &Arc<RwLock<ProviderManager>>, provider: &Provider) {
        let mut updated = provider.clone();
        if let Some(config) = &mut updated.checkin_config {
            match checkin::generate_retry_time(config) {
                Some(retry_time) => {
                    config.next_checkin_at = Some(retry_time);
                    config.last_checkin_status = Some("failed".to_string());

                    let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(retry_time, 0)
                        .unwrap_or_default()
                        .with_timezone(&Local);
                    tracing::info!(
                        "供应商 {} 将在 {} 重试签到",
                        provider.name,
                        dt.format("%H:%M")
                    );
                }
                None => {
                    // 今天范围已过，清除计划，明天再来
                    config.next_checkin_at = None;
                    config.last_checkin_status = Some("failed".to_string());
                    tracing::info!("供应商 {} 今日签到范围已过，明天重试", provider.name);
                }
            }
        }

        let manager = provider_manager.write().await;
        if let Err(e) = manager.update_provider(&provider.id, updated) {
            tracing::error!("保存重试计划失败 [{}]: {}", provider.name, e);
        }
    }

    /// 立即执行一次签到检查（用于测试）
    pub async fn run_once(&self) -> anyhow::Result<()> {
        Self::check_and_checkin(&self.provider_manager).await
    }
}
