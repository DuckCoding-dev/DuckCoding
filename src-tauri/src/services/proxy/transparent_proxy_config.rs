// 透明代理配置管理服务
use crate::models::{GlobalConfig, Tool};
use anyhow::{Context, Result};
use serde_json::{Map, Value};
use std::fs;

pub struct TransparentProxyConfigService;

impl TransparentProxyConfigService {
    /// 启用透明代理 - 保存真实配置并修改 ClaudeCode 配置指向本地代理
    pub fn enable_transparent_proxy(
        tool: &Tool,
        global_config: &mut GlobalConfig,
        local_proxy_port: u16,
        local_proxy_key: &str,
    ) -> Result<()> {
        if tool.id != "claude-code" {
            anyhow::bail!("透明代理目前仅支持 ClaudeCode");
        }

        let config_path = tool.config_dir.join(&tool.config_file);

        // 1. 读取当前 ClaudeCode 配置，保存真实的 API Key 和 Base URL
        let mut settings = if config_path.exists() {
            let content = fs::read_to_string(&config_path).context("读取配置文件失败")?;
            serde_json::from_str::<Value>(&content).unwrap_or(Value::Object(Map::new()))
        } else {
            anyhow::bail!("ClaudeCode 配置文件不存在，请先配置 API");
        };

        if !settings.is_object() {
            anyhow::bail!("配置文件格式错误");
        }

        let obj = settings.as_object_mut().unwrap();
        let env = obj
            .get("env")
            .and_then(|v| v.as_object())
            .ok_or_else(|| anyhow::anyhow!("配置文件缺少 env 字段"))?;

        // 获取真实的 API Key 和 Base URL
        let real_api_key = env
            .get("ANTHROPIC_AUTH_TOKEN")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("未找到 API Key，请先配置"))?
            .to_string();

        let real_base_url = env
            .get("ANTHROPIC_BASE_URL")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("未找到 Base URL，请先配置"))?
            .to_string();

        // 2. 保存真实配置到全局配置
        global_config.transparent_proxy_real_api_key = Some(real_api_key);
        global_config.transparent_proxy_real_base_url = Some(real_base_url);

        // 3. 修改 ClaudeCode 配置指向本地代理
        let env_mut = obj.get_mut("env").unwrap().as_object_mut().unwrap();
        env_mut.insert(
            "ANTHROPIC_AUTH_TOKEN".to_string(),
            Value::String(local_proxy_key.to_string()),
        );
        env_mut.insert(
            "ANTHROPIC_BASE_URL".to_string(),
            Value::String(format!("http://127.0.0.1:{}", local_proxy_port)),
        );

        // 写入配置
        let json = serde_json::to_string_pretty(&settings)?;
        fs::write(&config_path, json)?;

        println!("✅ 透明代理已启用，ClaudeCode 配置已指向本地代理");

        Ok(())
    }

    /// 禁用透明代理 - 恢复真实配置到 ClaudeCode
    pub fn disable_transparent_proxy(tool: &Tool, global_config: &GlobalConfig) -> Result<()> {
        if tool.id != "claude-code" {
            anyhow::bail!("透明代理目前仅支持 ClaudeCode");
        }

        let real_api_key = global_config
            .transparent_proxy_real_api_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("未找到保存的真实 API Key"))?;

        let real_base_url = global_config
            .transparent_proxy_real_base_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("未找到保存的真实 Base URL"))?;

        let config_path = tool.config_dir.join(&tool.config_file);

        // 读取配置
        let mut settings = if config_path.exists() {
            let content = fs::read_to_string(&config_path).context("读取配置文件失败")?;
            serde_json::from_str::<Value>(&content).unwrap_or(Value::Object(Map::new()))
        } else {
            Value::Object(Map::new())
        };

        if !settings.is_object() {
            settings = serde_json::json!({});
        }

        let obj = settings.as_object_mut().unwrap();
        if !obj.contains_key("env") {
            obj.insert("env".to_string(), Value::Object(Map::new()));
        }

        // 恢复真实配置
        let env = obj.get_mut("env").unwrap().as_object_mut().unwrap();
        env.insert(
            "ANTHROPIC_AUTH_TOKEN".to_string(),
            Value::String(real_api_key.clone()),
        );
        env.insert(
            "ANTHROPIC_BASE_URL".to_string(),
            Value::String(real_base_url.clone()),
        );

        // 写入配置
        let json = serde_json::to_string_pretty(&settings)?;
        fs::write(&config_path, json)?;

        println!("✅ 透明代理已禁用，ClaudeCode 配置已恢复");

        Ok(())
    }

    /// 更新透明代理的真实配置（切换配置时调用）
    pub fn update_real_config(
        tool: &Tool,
        global_config: &mut GlobalConfig,
        new_api_key: &str,
        new_base_url: &str,
    ) -> Result<()> {
        if tool.id != "claude-code" {
            anyhow::bail!("透明代理目前仅支持 ClaudeCode");
        }

        // 只更新全局配置中保存的真实配置，不修改 ClaudeCode 配置文件
        global_config.transparent_proxy_real_api_key = Some(new_api_key.to_string());
        global_config.transparent_proxy_real_base_url = Some(new_base_url.to_string());

        println!("✅ 透明代理真实配置已更新");

        Ok(())
    }

    /// 更新 ClaudeCode 配置指向本地代理（不备份真实配置）
    pub fn update_config_to_proxy(
        tool: &Tool,
        local_proxy_port: u16,
        local_proxy_key: &str,
    ) -> Result<()> {
        if tool.id != "claude-code" {
            anyhow::bail!("透明代理目前仅支持 ClaudeCode");
        }

        let config_path = tool.config_dir.join(&tool.config_file);

        // 读取当前 ClaudeCode 配置
        let mut settings = if config_path.exists() {
            let content = fs::read_to_string(&config_path).context("读取配置文件失败")?;
            serde_json::from_str::<Value>(&content).unwrap_or(Value::Object(Map::new()))
        } else {
            anyhow::bail!("ClaudeCode 配置文件不存在");
        };

        if !settings.is_object() {
            anyhow::bail!("配置文件格式错误");
        }

        let obj = settings.as_object_mut().unwrap();
        if !obj.contains_key("env") {
            obj.insert("env".to_string(), Value::Object(Map::new()));
        }

        // 修改 ClaudeCode 配置指向本地代理
        let env = obj.get_mut("env").unwrap().as_object_mut().unwrap();
        env.insert(
            "ANTHROPIC_AUTH_TOKEN".to_string(),
            Value::String(local_proxy_key.to_string()),
        );
        env.insert(
            "ANTHROPIC_BASE_URL".to_string(),
            Value::String(format!("http://127.0.0.1:{}", local_proxy_port)),
        );

        // 写入配置
        let json = serde_json::to_string_pretty(&settings)?;
        fs::write(&config_path, json)?;

        println!("✅ ClaudeCode 配置已更新指向本地代理");

        Ok(())
    }

    /// 获取真实的 API 配置（用于代理服务）
    pub fn get_real_config(global_config: &GlobalConfig) -> Result<(String, String)> {
        let api_key = global_config
            .transparent_proxy_real_api_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("未找到真实 API Key"))?
            .clone();

        let base_url = global_config
            .transparent_proxy_real_base_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("未找到真实 Base URL"))?
            .clone();

        Ok((api_key, base_url))
    }

    /// 从备份配置文件读取真实的 API 配置
    pub fn read_real_config_from_backup(
        tool: &Tool,
        profile_name: &str,
    ) -> Result<(String, String)> {
        if tool.id != "claude-code" {
            anyhow::bail!("透明代理目前仅支持 ClaudeCode");
        }

        let backup_path = tool.backup_path(profile_name);

        if !backup_path.exists() {
            anyhow::bail!("备份配置文件不存在: {:?}", backup_path);
        }

        let content = fs::read_to_string(&backup_path).context("读取备份配置失败")?;
        let backup_data: Value = serde_json::from_str(&content).context("解析备份配置失败")?;

        // 兼容新旧格式
        let api_key = backup_data
            .get("ANTHROPIC_AUTH_TOKEN")
            .and_then(|v| v.as_str())
            .or_else(|| {
                backup_data
                    .get("env")
                    .and_then(|env| env.get("ANTHROPIC_AUTH_TOKEN"))
                    .and_then(|v| v.as_str())
            })
            .ok_or_else(|| anyhow::anyhow!("备份配置中未找到 API Key"))?
            .to_string();

        let base_url = backup_data
            .get("ANTHROPIC_BASE_URL")
            .and_then(|v| v.as_str())
            .or_else(|| {
                backup_data
                    .get("env")
                    .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
                    .and_then(|v| v.as_str())
            })
            .ok_or_else(|| anyhow::anyhow!("备份配置中未找到 Base URL"))?
            .to_string();

        Ok((api_key, base_url))
    }
}
