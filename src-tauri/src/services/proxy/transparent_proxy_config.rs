// 透明代理配置管理服务
use crate::models::{GlobalConfig, Tool, ToolProxyConfig};
use anyhow::{Context, Result};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fs;

pub struct TransparentProxyConfigService;

impl TransparentProxyConfigService {
    /// 启用透明代理 - 保存真实配置并修改工具配置指向本地代理
    pub fn enable_transparent_proxy(
        tool: &Tool,
        global_config: &mut GlobalConfig,
        local_proxy_port: u16,
        local_proxy_key: &str,
    ) -> Result<()> {
        // 1. 读取当前工具的真实配置
        let (real_api_key, real_base_url) = Self::read_tool_config(tool)?;

        // 2. 保存真实配置到 proxy_configs
        global_config.ensure_proxy_config(&tool.id, local_proxy_port);
        if let Some(proxy_config) = global_config.get_proxy_config_mut(&tool.id) {
            proxy_config.real_api_key = Some(real_api_key);
            proxy_config.real_base_url = Some(real_base_url);
            proxy_config.local_api_key = Some(local_proxy_key.to_string());

            // 对于 Codex，还需要保存原始的 model_provider
            if tool.id == "codex" {
                let model_provider = Self::read_codex_model_provider(tool)?;
                proxy_config.real_model_provider = Some(model_provider);
            }
        }

        // 兼容旧字段（仅 claude-code）
        if tool.id == "claude-code" {
            let (real_api_key_clone, real_base_url_clone) = {
                let proxy_config = global_config.get_proxy_config(&tool.id);
                (
                    proxy_config.and_then(|c| c.real_api_key.clone()),
                    proxy_config.and_then(|c| c.real_base_url.clone()),
                )
            };
            global_config.transparent_proxy_real_api_key = real_api_key_clone;
            global_config.transparent_proxy_real_base_url = real_base_url_clone;
        }

        // 3. 修改工具配置指向本地代理
        Self::write_proxy_config(tool, local_proxy_port, local_proxy_key)?;

        tracing::info!(
            tool_id = %tool.id,
            "透明代理已启用，配置已指向本地代理"
        );

        Ok(())
    }

    /// 禁用透明代理 - 恢复真实配置到工具
    pub fn disable_transparent_proxy(tool: &Tool, global_config: &GlobalConfig) -> Result<()> {
        let (real_api_key, real_base_url, real_model_provider) =
            if let Some(proxy_config) = global_config.get_proxy_config(&tool.id) {
                let api_key = proxy_config
                    .real_api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("未找到 {} 保存的真实 API Key", tool.id))?;
                let base_url = proxy_config
                    .real_base_url
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("未找到 {} 保存的真实 Base URL", tool.id))?;
                let model_provider = proxy_config.real_model_provider.clone();
                (api_key.clone(), base_url.clone(), model_provider)
            } else {
                // 兼容旧字段（仅 claude-code）
                if tool.id == "claude-code" {
                    let api_key = global_config
                        .transparent_proxy_real_api_key
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("未找到保存的真实 API Key"))?;
                    let base_url = global_config
                        .transparent_proxy_real_base_url
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("未找到保存的真实 Base URL"))?;
                    (api_key.clone(), base_url.clone(), None)
                } else {
                    anyhow::bail!("未找到 {} 的代理配置", tool.id);
                }
            };

        // 恢复真实配置
        Self::write_real_config_with_provider(
            tool,
            &real_api_key,
            &real_base_url,
            real_model_provider.as_deref(),
        )?;

        tracing::info!(
            tool_id = %tool.id,
            "透明代理已禁用，配置已恢复"
        );

        Ok(())
    }

    /// 更新透明代理的真实配置（切换配置时调用）
    pub fn update_real_config(
        tool: &Tool,
        global_config: &mut GlobalConfig,
        new_api_key: &str,
        new_base_url: &str,
    ) -> Result<()> {
        // 更新 proxy_configs 中保存的真实配置
        if let Some(proxy_config) = global_config.get_proxy_config_mut(&tool.id) {
            proxy_config.real_api_key = Some(new_api_key.to_string());
            proxy_config.real_base_url = Some(new_base_url.to_string());
        }

        // 兼容旧字段（仅 claude-code）
        if tool.id == "claude-code" {
            global_config.transparent_proxy_real_api_key = Some(new_api_key.to_string());
            global_config.transparent_proxy_real_base_url = Some(new_base_url.to_string());
        }

        tracing::info!(
            tool_id = %tool.id,
            "透明代理真实配置已更新"
        );

        Ok(())
    }

    /// 更新工具配置指向本地代理（不备份真实配置）
    pub fn update_config_to_proxy(
        tool: &Tool,
        local_proxy_port: u16,
        local_proxy_key: &str,
    ) -> Result<()> {
        Self::write_proxy_config(tool, local_proxy_port, local_proxy_key)?;
        tracing::info!(
            tool_id = %tool.id,
            "配置已更新指向本地代理"
        );
        Ok(())
    }

    /// 获取真实的 API 配置（用于代理服务）
    pub fn get_real_config(global_config: &GlobalConfig) -> Result<(String, String)> {
        // 兼容旧字段
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

    /// 获取指定工具的真实配置
    pub fn get_tool_real_config(
        tool_id: &str,
        proxy_config: &ToolProxyConfig,
    ) -> Result<(String, String)> {
        let api_key = proxy_config
            .real_api_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("未找到 {tool_id} 的真实 API Key"))?
            .clone();

        let base_url = proxy_config
            .real_base_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("未找到 {tool_id} 的真实 Base URL"))?
            .clone();

        Ok((api_key, base_url))
    }

    // ==================== 私有方法 ====================

    /// 读取工具的当前配置（API Key 和 Base URL）
    fn read_tool_config(tool: &Tool) -> Result<(String, String)> {
        match tool.id.as_str() {
            "claude-code" => Self::read_claude_config(tool),
            "codex" => Self::read_codex_config(tool),
            "gemini-cli" => Self::read_gemini_config(tool),
            _ => anyhow::bail!("不支持的工具: {}", tool.id),
        }
    }

    /// 写入代理配置到工具
    fn write_proxy_config(tool: &Tool, port: u16, api_key: &str) -> Result<()> {
        let base_url = format!("http://127.0.0.1:{port}");
        match tool.id.as_str() {
            "claude-code" => Self::write_claude_config(tool, api_key, &base_url),
            "codex" => Self::write_codex_config(tool, api_key, &base_url),
            "gemini-cli" => Self::write_gemini_config(tool, api_key, &base_url),
            _ => anyhow::bail!("不支持的工具: {}", tool.id),
        }
    }

    /// 写入真实配置到工具（带 model_provider 参数，用于 Codex）
    fn write_real_config_with_provider(
        tool: &Tool,
        api_key: &str,
        base_url: &str,
        model_provider: Option<&str>,
    ) -> Result<()> {
        match tool.id.as_str() {
            "claude-code" => Self::write_claude_config(tool, api_key, base_url),
            "codex" => {
                Self::write_codex_config_with_provider(tool, api_key, base_url, model_provider)
            }
            "gemini-cli" => Self::write_gemini_config(tool, api_key, base_url),
            _ => anyhow::bail!("不支持的工具: {}", tool.id),
        }
    }

    // ==================== Claude Code ====================

    fn read_claude_config(tool: &Tool) -> Result<(String, String)> {
        let config_path = tool.config_dir.join(&tool.config_file);

        if !config_path.exists() {
            anyhow::bail!("Claude Code 配置文件不存在，请先配置 API");
        }

        let content = fs::read_to_string(&config_path).context("读取 Claude Code 配置失败")?;
        let settings: Value =
            serde_json::from_str(&content).context("解析 Claude Code 配置失败")?;

        let env = settings
            .get("env")
            .and_then(|v| v.as_object())
            .ok_or_else(|| anyhow::anyhow!("配置文件缺少 env 字段"))?;

        let api_key = env
            .get("ANTHROPIC_AUTH_TOKEN")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("未找到 API Key"))?
            .to_string();

        let base_url = env
            .get("ANTHROPIC_BASE_URL")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("未找到 Base URL"))?
            .to_string();

        Ok((api_key, base_url))
    }

    fn write_claude_config(tool: &Tool, api_key: &str, base_url: &str) -> Result<()> {
        let config_path = tool.config_dir.join(&tool.config_file);

        let mut settings = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
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

        let env = obj.get_mut("env").unwrap().as_object_mut().unwrap();
        env.insert(
            "ANTHROPIC_AUTH_TOKEN".to_string(),
            Value::String(api_key.to_string()),
        );
        env.insert(
            "ANTHROPIC_BASE_URL".to_string(),
            Value::String(base_url.to_string()),
        );

        let json = serde_json::to_string_pretty(&settings)?;
        fs::write(&config_path, json)?;

        Ok(())
    }

    // ==================== Codex ====================

    fn read_codex_config(tool: &Tool) -> Result<(String, String)> {
        let auth_path = tool.config_dir.join("auth.json");
        let config_path = tool.config_dir.join(&tool.config_file);

        // 读取 API Key from auth.json
        if !auth_path.exists() {
            anyhow::bail!("Codex auth.json 不存在，请先配置 API");
        }

        let auth_content = fs::read_to_string(&auth_path).context("读取 Codex auth.json 失败")?;
        let auth_data: Value =
            serde_json::from_str(&auth_content).context("解析 Codex auth.json 失败")?;

        let api_key = auth_data
            .get("OPENAI_API_KEY")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Codex auth.json 中未找到 OPENAI_API_KEY"))?
            .to_string();

        // 读取 Base URL from config.toml
        if !config_path.exists() {
            anyhow::bail!("Codex config.toml 不存在，请先配置 API");
        }

        let config_content =
            fs::read_to_string(&config_path).context("读取 Codex config.toml 失败")?;
        let config: toml::Value =
            toml::from_str(&config_content).context("解析 Codex config.toml 失败")?;

        // 获取当前 provider
        let provider = config
            .get("model_provider")
            .and_then(|v| v.as_str())
            .unwrap_or("custom");

        // 从 model_providers.[provider].base_url 获取
        let base_url = config
            .get("model_providers")
            .and_then(|mp| mp.get(provider))
            .and_then(|p| p.get("base_url"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                anyhow::anyhow!("Codex config.toml 中未找到 model_providers.{provider}.base_url")
            })?
            .to_string();

        Ok((api_key, base_url))
    }

    /// 读取 Codex 当前的 model_provider
    fn read_codex_model_provider(tool: &Tool) -> Result<String> {
        let config_path = tool.config_dir.join(&tool.config_file);

        if !config_path.exists() {
            anyhow::bail!("Codex config.toml 不存在");
        }

        let config_content =
            fs::read_to_string(&config_path).context("读取 Codex config.toml 失败")?;
        let config: toml::Value =
            toml::from_str(&config_content).context("解析 Codex config.toml 失败")?;

        let provider = config
            .get("model_provider")
            .and_then(|v| v.as_str())
            .unwrap_or("custom")
            .to_string();

        Ok(provider)
    }

    fn write_codex_config(tool: &Tool, api_key: &str, base_url: &str) -> Result<()> {
        // 默认行为：根据 URL 判断 provider
        Self::write_codex_config_with_provider(tool, api_key, base_url, None)
    }

    fn write_codex_config_with_provider(
        tool: &Tool,
        api_key: &str,
        base_url: &str,
        model_provider: Option<&str>,
    ) -> Result<()> {
        let auth_path = tool.config_dir.join("auth.json");
        let config_path = tool.config_dir.join(&tool.config_file);

        // 确保目录存在
        fs::create_dir_all(&tool.config_dir)?;

        // 更新 auth.json
        let mut auth_data = if auth_path.exists() {
            let content = fs::read_to_string(&auth_path)?;
            serde_json::from_str::<Value>(&content).unwrap_or(Value::Object(Map::new()))
        } else {
            Value::Object(Map::new())
        };

        if let Value::Object(ref mut auth_obj) = auth_data {
            auth_obj.insert(
                "OPENAI_API_KEY".to_string(),
                Value::String(api_key.to_string()),
            );
        }

        fs::write(&auth_path, serde_json::to_string_pretty(&auth_data)?)?;

        // 更新 config.toml（使用 toml_edit 保留注释）
        let mut doc = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            content
                .parse::<toml_edit::DocumentMut>()
                .unwrap_or_else(|_| toml_edit::DocumentMut::new())
        } else {
            toml_edit::DocumentMut::new()
        };

        let root_table = doc.as_table_mut();

        // 判断 provider 类型
        // 如果提供了 model_provider 参数，直接使用它（用于恢复原始配置）
        // 否则根据 URL 判断：本地代理使用 "proxy"，DuckCoding 使用 "duckcoding"，其他使用 "custom"
        let provider_key = if let Some(provider) = model_provider {
            provider
        } else {
            let is_local_proxy = base_url.contains("127.0.0.1") || base_url.contains("localhost");
            let is_duckcoding = base_url.contains("duckcoding");

            if is_local_proxy {
                "proxy"
            } else if is_duckcoding {
                "duckcoding"
            } else {
                "custom"
            }
        };

        // 更新 model_provider
        root_table.insert("model_provider", toml_edit::value(provider_key));

        // 确保 /v1 后缀（配置文件需要包含 /v1）
        let normalized_base = base_url.trim_end_matches('/');
        let base_url_with_v1 = if normalized_base.ends_with("/v1") {
            normalized_base.to_string()
        } else {
            format!("{normalized_base}/v1")
        };

        // 确保 model_providers 表存在
        if !root_table
            .get("model_providers")
            .map(|item| item.is_table())
            .unwrap_or(false)
        {
            let mut table = toml_edit::Table::new();
            table.set_implicit(false);
            root_table.insert("model_providers", toml_edit::Item::Table(table));
        }

        let providers_table = root_table
            .get_mut("model_providers")
            .and_then(|item| item.as_table_mut())
            .ok_or_else(|| anyhow::anyhow!("model_providers 不是表结构"))?;

        if !providers_table.contains_key(provider_key) {
            let mut table = toml_edit::Table::new();
            table.set_implicit(false);
            providers_table.insert(provider_key, toml_edit::Item::Table(table));
        }

        if let Some(provider_table) = providers_table
            .get_mut(provider_key)
            .and_then(|item| item.as_table_mut())
        {
            provider_table.insert("name", toml_edit::value(provider_key));
            provider_table.insert("base_url", toml_edit::value(base_url_with_v1));
            provider_table.insert("wire_api", toml_edit::value("responses"));
            provider_table.insert("requires_openai_auth", toml_edit::value(true));
        }

        fs::write(&config_path, doc.to_string())?;

        Ok(())
    }

    // ==================== Gemini CLI ====================

    fn read_gemini_config(tool: &Tool) -> Result<(String, String)> {
        let env_path = tool.config_dir.join(".env");

        if !env_path.exists() {
            anyhow::bail!("Gemini CLI .env 不存在，请先配置 API");
        }

        let content = fs::read_to_string(&env_path).context("读取 Gemini .env 失败")?;

        let mut env_vars = HashMap::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                if let Some((key, value)) = trimmed.split_once('=') {
                    env_vars.insert(key.trim().to_string(), value.trim().to_string());
                }
            }
        }

        let api_key = env_vars
            .get("GEMINI_API_KEY")
            .ok_or_else(|| anyhow::anyhow!("Gemini .env 中未找到 GEMINI_API_KEY"))?
            .clone();

        let base_url = env_vars
            .get("GOOGLE_GEMINI_BASE_URL")
            .ok_or_else(|| anyhow::anyhow!("Gemini .env 中未找到 GOOGLE_GEMINI_BASE_URL"))?
            .clone();

        Ok((api_key, base_url))
    }

    fn write_gemini_config(tool: &Tool, api_key: &str, base_url: &str) -> Result<()> {
        let env_path = tool.config_dir.join(".env");

        // 确保目录存在
        fs::create_dir_all(&tool.config_dir)?;

        // 读取现有 .env
        let mut env_vars = HashMap::new();
        if env_path.exists() {
            let content = fs::read_to_string(&env_path)?;
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    if let Some((key, value)) = trimmed.split_once('=') {
                        env_vars.insert(key.trim().to_string(), value.trim().to_string());
                    }
                }
            }
        }

        // 更新 API 相关字段
        env_vars.insert("GEMINI_API_KEY".to_string(), api_key.to_string());
        env_vars.insert("GOOGLE_GEMINI_BASE_URL".to_string(), base_url.to_string());

        // 写入 .env
        let mut env_content = String::new();
        for (key, value) in &env_vars {
            env_content.push_str(&format!("{key}={value}\n"));
        }

        fs::write(&env_path, env_content)?;

        Ok(())
    }

    /// 从备份配置文件读取真实的 API 配置（仅 Claude Code）
    pub fn read_real_config_from_backup(
        tool: &Tool,
        profile_name: &str,
    ) -> Result<(String, String)> {
        if tool.id != "claude-code" {
            anyhow::bail!("从备份读取配置目前仅支持 Claude Code");
        }

        let backup_path = tool.backup_path(profile_name);

        if !backup_path.exists() {
            anyhow::bail!("备份配置文件不存在: {backup_path:?}");
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
