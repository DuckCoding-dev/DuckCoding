// Checkin Scheduler State
//
// 签到调度器全局状态

use crate::services::CheckinScheduler;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct CheckinSchedulerState {
    pub scheduler: Arc<RwLock<CheckinScheduler>>,
}

impl CheckinSchedulerState {
    pub fn new(scheduler: CheckinScheduler) -> Self {
        Self {
            scheduler: Arc::new(RwLock::new(scheduler)),
        }
    }
}
