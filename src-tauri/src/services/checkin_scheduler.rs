// Checkin Scheduler
//
// 签到定时任务调度器

use crate::services::{checkin, provider_manager::ProviderManager};
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
            log::warn!("签到调度器已在运行");
            return;
        }
        *running = true;
        drop(running);

        let provider_manager = self.provider_manager.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            log::info!("签到调度器已启动");

            // 每小时检查一次
            let mut interval = time::interval(Duration::from_secs(3600));

            loop {
                interval.tick().await;

                let is_running = *running.read().await;
                if !is_running {
                    log::info!("签到调度器已停止");
                    break;
                }

                // 执行签到检查
                if let Err(e) = Self::check_and_checkin(&provider_manager).await {
                    log::error!("签到检查失败: {}", e);
                }
            }
        });
    }

    /// 停止定时任务
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        log::info!("签到调度器停止中...");
    }

    /// 检查并执行签到
    async fn check_and_checkin(
        provider_manager: &Arc<RwLock<ProviderManager>>,
    ) -> anyhow::Result<()> {
        // 先获取所有需要签到的供应商
        let providers_to_checkin = {
            let manager = provider_manager.read().await;
            let providers = manager.list_providers().await?;
            
            providers
                .into_iter()
                .filter(|p| {
                    p.checkin_config
                        .as_ref()
                        .map(|c| checkin::should_checkin(c))
                        .unwrap_or(false)
                })
                .collect::<Vec<_>>()
        };

        // 逐个执行签到
        for provider in providers_to_checkin {
            log::info!("开始为供应商 {} 执行自动签到", provider.name);

            match checkin::perform_checkin(&provider).await {
                Ok(response) => {
                    if response.success {
                        log::info!(
                            "供应商 {} 签到成功: {:?}",
                            provider.name,
                            response.message
                        );

                        // 更新签到配置
                        let mut updated_provider = provider.clone();
                        if let Some(config) = &mut updated_provider.checkin_config {
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

                        // 保存更新
                        let mut manager_write = provider_manager.write().await;
                        if let Err(e) = manager_write
                            .update_provider(&provider.id, updated_provider)
                            .await
                        {
                            log::error!("更新供应商签到配置失败: {}", e);
                        }
                    } else {
                        log::warn!(
                            "供应商 {} 签到失败: {:?}",
                            provider.name,
                            response.message
                        );
                    }
                }
                Err(e) => {
                    log::error!("供应商 {} 签到请求失败: {}", provider.name, e);
                }
            }
        }

        Ok(())
    }

    /// 立即执行一次签到检查（用于测试）
    pub async fn run_once(&self) -> anyhow::Result<()> {
        Self::check_and_checkin(&self.provider_manager).await
    }
}
