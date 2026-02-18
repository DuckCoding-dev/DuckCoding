use crate::http_client::build_client;
use crate::models::pricing::{ModelPrice, PricingTemplate};
use crate::services::pricing::PRICING_MANAGER;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const REMOTE_URL: &str = "https://raw.githubusercontent.com/Wei-Shaw/claude-relay-service/price-mirror/model_prices_and_context_window.json";

/// 远程同步状态（持久化到 remote_sync_state.json）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemoteSyncState {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub last_success_at: Option<i64>,
}

/// 远程模型定价数据（宽松解析，所有字段可选）
#[derive(Debug, Deserialize)]
struct RemoteModelData {
    litellm_provider: Option<String>,
    input_cost_per_token: Option<f64>,
    output_cost_per_token: Option<f64>,
    cache_creation_input_token_cost: Option<f64>,
    cache_read_input_token_cost: Option<f64>,
    reasoning_cost_per_token: Option<f64>,
    mode: Option<String>,
}

/// 从远程同步最新模型定价数据并更新本地内置模板
///
/// 返回 Ok(true) 表示有更新，Ok(false) 表示无需更新（304）
pub async fn sync_remote_prices() -> Result<bool> {
    let client = build_client().map_err(|e| anyhow::anyhow!(e))?;

    let state = PRICING_MANAGER.load_sync_state().unwrap_or_default();

    let mut request = client.get(REMOTE_URL);
    if let Some(etag) = &state.etag {
        request = request.header("If-None-Match", etag);
    }

    let response = request.send().await.context("远程价格同步请求失败")?;

    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        tracing::info!("远程价格数据未变化 (304)，跳过同步");
        return Ok(false);
    }

    if !response.status().is_success() {
        anyhow::bail!("远程价格同步失败，HTTP 状态码: {}", response.status());
    }

    let new_etag = response
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let new_last_modified = response
        .headers()
        .get("last-modified")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let body = response.text().await.context("读取远程价格数据失败")?;
    let all_models: HashMap<String, RemoteModelData> =
        serde_json::from_str(&body).context("解析远程价格 JSON 失败")?;

    // 按 provider 分组过滤
    let mut anthropic_models: HashMap<String, &RemoteModelData> = HashMap::new();
    let mut openai_models: HashMap<String, &RemoteModelData> = HashMap::new();
    let mut gemini_models: HashMap<String, &RemoteModelData> = HashMap::new();

    for (key, data) in &all_models {
        // 过滤掉包含 `/` 的第三方平台条目
        if key.contains('/') {
            continue;
        }

        // 过滤 mode：仅保留 chat/responses 模式（排除 embedding/image_generation 等）
        if let Some(mode) = &data.mode {
            if mode != "chat" && mode != "responses" {
                continue;
            }
        }

        // 过滤掉没有有效价格的条目
        let has_input = data.input_cost_per_token.is_some_and(|v| v > 0.0);
        let has_output = data.output_cost_per_token.is_some_and(|v| v > 0.0);
        if !has_input || !has_output {
            continue;
        }

        match data.litellm_provider.as_deref() {
            Some("anthropic") => {
                anthropic_models.insert(key.clone(), data);
            }
            Some("openai") => {
                openai_models.insert(key.clone(), data);
            }
            // Gemini 模型的 litellm_provider 以 "vertex_ai" 开头，且 key 以 "gemini-" 开头
            Some(p) if p.starts_with("vertex_ai") && key.starts_with("gemini-") => {
                gemini_models.insert(key.clone(), data);
            }
            _ => {}
        }
    }

    let mut updated_count = 0;

    // 生成并保存 Anthropic 模板
    if !anthropic_models.is_empty() {
        let existing = PRICING_MANAGER.get_template("builtin_claude").ok();
        let template =
            build_template_from_remote("anthropic", &anthropic_models, existing.as_ref());
        PRICING_MANAGER
            .save_template(&template)
            .context("保存远程 Anthropic 价格模板失败")?;
        updated_count += anthropic_models.len();
        tracing::info!("同步 Anthropic 模型定价：{} 个模型", anthropic_models.len());
    }

    // 生成并保存 OpenAI 模板
    if !openai_models.is_empty() {
        let existing = PRICING_MANAGER.get_template("builtin_openai").ok();
        let template = build_template_from_remote("openai", &openai_models, existing.as_ref());
        PRICING_MANAGER
            .save_template(&template)
            .context("保存远程 OpenAI 价格模板失败")?;
        updated_count += openai_models.len();
        tracing::info!("同步 OpenAI 模型定价：{} 个模型", openai_models.len());
    }

    // 生成并保存 Gemini 模板
    if !gemini_models.is_empty() {
        let existing = PRICING_MANAGER.get_template("builtin_gemini").ok();
        let template = build_template_from_remote("gemini", &gemini_models, existing.as_ref());
        PRICING_MANAGER
            .save_template(&template)
            .context("保存远程 Gemini 价格模板失败")?;
        updated_count += gemini_models.len();
        tracing::info!("同步 Gemini 模型定价：{} 个模型", gemini_models.len());
    }

    // 更新同步状态
    let new_state = RemoteSyncState {
        etag: new_etag,
        last_modified: new_last_modified,
        last_success_at: Some(chrono::Utc::now().timestamp_millis()),
    };
    if let Err(e) = PRICING_MANAGER.save_sync_state(&new_state) {
        tracing::warn!("保存远程同步状态失败: {}", e);
    }

    tracing::info!("远程价格同步完成，共更新 {} 个模型", updated_count);
    Ok(true)
}

/// 从远程数据构建内置价格模板
fn build_template_from_remote(
    provider: &str,
    models: &HashMap<String, &RemoteModelData>,
    existing_template: Option<&PricingTemplate>,
) -> PricingTemplate {
    let (template_id, name, description, tags) = match provider {
        "anthropic" => (
            "builtin_claude",
            "内置Claude价格",
            "Anthropic 官方定价（远程同步）",
            vec!["official".to_string(), "claude".to_string()],
        ),
        "openai" => (
            "builtin_openai",
            "内置OpenAI价格",
            "OpenAI 官方定价（远程同步）",
            vec![
                "official".to_string(),
                "openai".to_string(),
                "codex".to_string(),
            ],
        ),
        "gemini" => (
            "builtin_gemini",
            "内置Gemini价格",
            "Google Gemini 官方定价（远程同步）",
            vec![
                "official".to_string(),
                "gemini".to_string(),
                "google".to_string(),
            ],
        ),
        _ => ("builtin_unknown", "未知提供商", "未知提供商定价", vec![]),
    };

    let mut custom_models = HashMap::new();

    for (key, data) in models {
        let input_per_1m = data.input_cost_per_token.unwrap_or(0.0) * 1_000_000.0;
        let output_per_1m = data.output_cost_per_token.unwrap_or(0.0) * 1_000_000.0;
        let cache_write = data
            .cache_creation_input_token_cost
            .map(|v| v * 1_000_000.0);
        // Anthropic 1h 缓存写入价格 = input * 2.0（远程数据仅提供 5m 价格）
        let cache_write_1h = if provider == "anthropic" {
            data.input_cost_per_token.map(|v| v * 2.0 * 1_000_000.0)
        } else {
            None
        };
        let cache_read = data.cache_read_input_token_cost.map(|v| v * 1_000_000.0);
        let reasoning = data.reasoning_cost_per_token.map(|v| v * 1_000_000.0);

        let aliases = generate_aliases(key);

        let model_price = ModelPrice::new(
            provider.to_string(),
            input_per_1m,
            output_per_1m,
            cache_write,
            cache_write_1h,
            cache_read,
            reasoning,
            aliases,
        );

        custom_models.insert(key.clone(), model_price);
    }

    let now = chrono::Utc::now().timestamp_millis();
    let created_at = existing_template.map(|t| t.created_at).unwrap_or(now);

    PricingTemplate {
        id: template_id.to_string(),
        name: name.to_string(),
        description: description.to_string(),
        version: "1.0".to_string(),
        created_at,
        updated_at: now,
        inherited_models: vec![],
        custom_models,
        tags,
        is_default_preset: true,
    }
}

/// 为模型 key 生成别名列表
///
/// 规则：
/// - 始终包含 key 本身
/// - 带 8 位日期后缀（-YYYYMMDD）的 key 生成无日期版本
/// - 名字中有 `-X-Y`（X/Y 均为单数字）模式时生成 `.X.Y` 版本
fn generate_aliases(model_key: &str) -> Vec<String> {
    let mut aliases = vec![model_key.to_string()];

    let mut base = model_key.to_string();

    // 检测并去掉 8 位日期后缀（如 -20250929）
    let has_date_suffix = if model_key.len() > 9 {
        let suffix = &model_key[model_key.len() - 9..];
        suffix.starts_with('-') && suffix[1..].chars().all(|c| c.is_ascii_digit())
    } else {
        false
    };

    if has_date_suffix {
        base = model_key[..model_key.len() - 9].to_string();
        if base != model_key {
            aliases.push(base.clone());
        }
    }

    // 查找 `-X-Y` 模式（X/Y 均为单数字）并生成 `.X.Y` 版本
    // 同时也生成含 `.` 替换 `-` 的反向版本
    let dot_version = replace_digit_dashes_with_dots(&base);
    if dot_version != base && !aliases.contains(&dot_version) {
        aliases.push(dot_version);
    }

    // 反向：如果 base 含有 `.X.Y` 模式，生成 `-X-Y` 版本
    let dash_version = replace_digit_dots_with_dashes(&base);
    if dash_version != base && !aliases.contains(&dash_version) {
        aliases.push(dash_version);
    }

    aliases
}

/// 将 `-X-Y`（X/Y 为单数字）模式替换为 `.X.Y`（仅替换连续数字段之间的 `-`）
///
/// 例如：`claude-sonnet-4-5` → `claude-sonnet-4.5`
/// 例如：`claude-3-5-haiku` → `claude-3.5-haiku`
fn replace_digit_dashes_with_dots(s: &str) -> String {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() < 2 {
        return s.to_string();
    }

    let mut result = String::with_capacity(s.len());
    result.push_str(parts[0]);

    for i in 1..parts.len() {
        // 如果当前 part 是纯数字且前一个 part 也是纯数字，用 '.' 连接
        let curr_is_digit = !parts[i].is_empty() && parts[i].chars().all(|c| c.is_ascii_digit());
        let prev_is_digit =
            !parts[i - 1].is_empty() && parts[i - 1].chars().all(|c| c.is_ascii_digit());

        if curr_is_digit && prev_is_digit {
            result.push('.');
        } else {
            result.push('-');
        }
        result.push_str(parts[i]);
    }

    result
}

/// 将 `.X`（X 为单数字）模式替换为 `-X`
///
/// 例如：`gpt-5.2-codex` → `gpt-5-2-codex`
fn replace_digit_dots_with_dashes(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::with_capacity(s.len());
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '.'
            && i > 0
            && chars[i - 1].is_ascii_digit()
            && i + 1 < chars.len()
            && chars[i + 1].is_ascii_digit()
        {
            result.push('-');
        } else {
            result.push(chars[i]);
        }
        i += 1;
    }

    result
}

/// 启动定期同步调度器
pub async fn start_sync_scheduler() {
    tokio::spawn(async {
        // 首次延迟 5 秒，避免影响启动速度
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        // 首次同步
        match sync_remote_prices().await {
            Ok(true) => tracing::info!("首次远程价格同步成功"),
            Ok(false) => tracing::info!("首次远程价格同步：数据未变化"),
            Err(e) => tracing::warn!("首次远程价格同步失败: {}", e),
        }

        // 计算距离下一个整点的延迟
        let now = chrono::Utc::now();
        let secs_past_hour = (now.timestamp() % 3600) as u64;
        let secs_to_next_hour = 3600 - secs_past_hour;
        tokio::time::sleep(std::time::Duration::from_secs(secs_to_next_hour)).await;

        // 每小时循环同步
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            match sync_remote_prices().await {
                Ok(true) => tracing::info!("定时远程价格同步成功"),
                Ok(false) => tracing::info!("定时远程价格同步：数据未变化"),
                Err(e) => tracing::warn!("定时远程价格同步失败: {}", e),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_aliases_with_date_suffix() {
        let aliases = generate_aliases("claude-sonnet-4-5-20250929");
        assert_eq!(aliases[0], "claude-sonnet-4-5-20250929");
        assert!(aliases.contains(&"claude-sonnet-4-5".to_string()));
        assert!(aliases.contains(&"claude-sonnet-4.5".to_string()));
    }

    #[test]
    fn test_generate_aliases_without_date() {
        let aliases = generate_aliases("claude-opus-4");
        assert_eq!(aliases[0], "claude-opus-4");
        // 没有日期后缀，也没有 -X-Y 模式，只有自身
        assert_eq!(aliases.len(), 1);
    }

    #[test]
    fn test_generate_aliases_opus_with_date() {
        let aliases = generate_aliases("claude-opus-4-20250514");
        assert_eq!(aliases[0], "claude-opus-4-20250514");
        assert!(aliases.contains(&"claude-opus-4".to_string()));
    }

    #[test]
    fn test_generate_aliases_dot_to_dash() {
        let aliases = generate_aliases("gpt-5.2-codex");
        assert_eq!(aliases[0], "gpt-5.2-codex");
        assert!(aliases.contains(&"gpt-5-2-codex".to_string()));
    }

    #[test]
    fn test_generate_aliases_haiku_old_format() {
        let aliases = generate_aliases("claude-3-5-haiku-20241022");
        assert_eq!(aliases[0], "claude-3-5-haiku-20241022");
        assert!(aliases.contains(&"claude-3-5-haiku".to_string()));
        // claude-3-5-haiku 中 3-5 应生成 3.5 版本
        assert!(aliases.contains(&"claude-3.5-haiku".to_string()));
    }

    #[test]
    fn test_replace_digit_dashes_with_dots() {
        assert_eq!(
            replace_digit_dashes_with_dots("claude-sonnet-4-5"),
            "claude-sonnet-4.5"
        );
        assert_eq!(
            replace_digit_dashes_with_dots("claude-3-5-haiku"),
            "claude-3.5-haiku"
        );
        // 不应替换非版本号部分
        assert_eq!(
            replace_digit_dashes_with_dots("claude-opus-4"),
            "claude-opus-4"
        );
    }

    #[test]
    fn test_replace_digit_dots_with_dashes() {
        assert_eq!(
            replace_digit_dots_with_dashes("gpt-5.2-codex"),
            "gpt-5-2-codex"
        );
        assert_eq!(
            replace_digit_dots_with_dashes("claude-sonnet-4.5"),
            "claude-sonnet-4-5"
        );
    }
}
