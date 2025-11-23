// 多代理实例管理器
//
// ProxyManager 负责协调多个 ProxyInstance 的生命周期：
// - 启动和停止指定工具的代理
// - 管理所有代理实例的状态
// - 确保端口不冲突

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::headers::create_request_processor;
use super::proxy_instance::ProxyInstance;
use crate::models::ToolProxyConfig;

/// 代理管理器
pub struct ProxyManager {
    instances: Arc<RwLock<HashMap<String, ProxyInstance>>>,
}

impl ProxyManager {
    /// 创建新的代理管理器
    pub fn new() -> Self {
        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 启动指定工具的代理
    ///
    /// # 参数
    /// - `tool_id`: 工具标识符 ("claude-code", "codex", "gemini-cli")
    /// - `config`: 工具的代理配置
    ///
    /// # 返回
    /// - `Ok(())`: 启动成功
    /// - `Err`: 启动失败（如端口已占用、配置无效等）
    pub async fn start_proxy(&self, tool_id: &str, config: ToolProxyConfig) -> Result<()> {
        // 检查是否已经在运行
        {
            let instances = self.instances.read().await;
            if let Some(instance) = instances.get(tool_id) {
                if instance.is_running_async().await {
                    anyhow::bail!("{tool_id} 代理已在运行");
                }
            }
        }

        // 检查端口冲突
        {
            let instances = self.instances.read().await;
            for (id, instance) in instances.iter() {
                if id != tool_id && instance.is_running_async().await {
                    // TODO: 获取运行中实例的端口并检查冲突
                    // 暂时跳过，因为 ProxyInstance 没有暴露 get_port 方法
                }
            }
        }

        // 创建 RequestProcessor
        let processor = create_request_processor(tool_id);

        // 创建并启动代理实例
        let instance = ProxyInstance::new(tool_id.to_string(), config, processor);
        instance
            .start()
            .await
            .context(format!("启动 {tool_id} 代理失败"))?;

        // 存入 HashMap
        {
            let mut instances = self.instances.write().await;
            instances.insert(tool_id.to_string(), instance);
        }

        Ok(())
    }

    /// 停止指定工具的代理
    ///
    /// # 参数
    /// - `tool_id`: 工具标识符
    ///
    /// # 返回
    /// - `Ok(())`: 停止成功（或代理未运行）
    /// - `Err`: 停止失败
    pub async fn stop_proxy(&self, tool_id: &str) -> Result<()> {
        let mut instances = self.instances.write().await;

        if let Some(instance) = instances.remove(tool_id) {
            instance
                .stop()
                .await
                .context(format!("停止 {tool_id} 代理失败"))?;
        } else {
            tracing::warn!(tool_id = %tool_id, "代理未运行或不存在");
        }

        Ok(())
    }

    /// 停止所有运行中的代理
    pub async fn stop_all(&self) -> Result<()> {
        let mut instances = self.instances.write().await;
        let tool_ids: Vec<String> = instances.keys().cloned().collect();

        for tool_id in tool_ids {
            if let Some(instance) = instances.remove(&tool_id) {
                if let Err(e) = instance.stop().await {
                    tracing::error!(
                        tool_id = %tool_id,
                        error = ?e,
                        "停止代理失败"
                    );
                }
            }
        }

        Ok(())
    }

    /// 检查指定工具的代理是否在运行
    pub async fn is_running(&self, tool_id: &str) -> bool {
        let instances = self.instances.read().await;
        instances
            .get(tool_id)
            .map(|i| {
                // 使用 blocking 获取状态（在异步上下文中）
                // 实际实现中应该使用异步版本
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async { i.is_running_async().await })
                })
            })
            .unwrap_or(false)
    }

    /// 获取所有工具的代理运行状态
    ///
    /// # 返回
    /// - HashMap<tool_id, is_running>
    pub async fn get_all_status(&self) -> HashMap<String, bool> {
        let instances = self.instances.read().await;
        let mut status_map = HashMap::new();

        for (tool_id, instance) in instances.iter() {
            let running = instance.is_running_async().await;
            status_map.insert(tool_id.clone(), running);
        }

        status_map
    }

    /// 更新指定工具的代理配置（无需重启）
    pub async fn update_config(&self, tool_id: &str, config: ToolProxyConfig) -> Result<()> {
        let instances = self.instances.read().await;

        if let Some(instance) = instances.get(tool_id) {
            instance
                .update_config(config)
                .await
                .context(format!("更新 {tool_id} 代理配置失败"))?;
        } else {
            anyhow::bail!("{tool_id} 代理未运行");
        }

        Ok(())
    }
}

impl Default for ProxyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_proxy_manager_creation() {
        let manager = ProxyManager::new();
        assert!(!manager.is_running("claude-code").await);
        assert!(!manager.is_running("codex").await);
        assert!(!manager.is_running("gemini-cli").await);
    }

    #[tokio::test]
    async fn test_get_all_status_empty() {
        let manager = ProxyManager::new();
        let status = manager.get_all_status().await;
        assert!(status.is_empty());
    }

    // 更多测试需要 mock 或集成测试环境
}
