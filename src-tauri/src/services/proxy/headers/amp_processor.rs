// AMP Code 请求处理器
//
// 路由逻辑：
// 0. 本地工具拦截：webSearch2 / extractWebPageContent → 本地处理
// 1. /api/provider/anthropic/* → Claude Profile（提取 /v1/messages）
// 2. /api/provider/openai/* → Codex Profile（提取 /v1/responses 或 /v1/chat/completions）
// 3. /api/provider/google/* → Gemini Profile（提取 /v1beta/...）
// 4. 其他 /api/* → ampcode.com（使用 AMP Access Token）
// 5. 直接 LLM 路径 → 按路径/headers/model 判断

use super::{
    ClaudeHeadersProcessor, CodexHeadersProcessor, GeminiHeadersProcessor, ProcessedRequest,
    RequestProcessor,
};
use crate::services::profile_manager::ProfileManager;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::StreamExt;
use hyper::HeaderMap as HyperHeaderMap;
use once_cell::sync::Lazy;
use reqwest::redirect::Policy;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::net::IpAddr;
use url::Url;
use uuid::Uuid;

/// 全局 HTTP Client（复用连接池，禁止重定向，允许系统代理）
static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .connect_timeout(std::time::Duration::from_secs(10))
        .redirect(Policy::none()) // 禁止重定向，防止 SSRF 绕过
        .build()
        .expect("Failed to create HTTP client")
});

static MCP_NAME_PREFIX_RE: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r#""name"\s*:\s*"mcp_([^"]+)""#).expect("mcp name 前缀正则非法")
});

static BRAND_SANITIZE_RE: Lazy<regex::Regex> = Lazy::new(|| {
    // 不区分大小写 + 单词边界替换，避免误伤子串（例如 "example" 中的 "amp"）。
    regex::Regex::new(r"(?i)\b(?:opencode|amp(?:-?code)?)\b").expect("清洗正则非法")
});

const CLAUDE_CODE_PREAMBLE: &str = "You are Claude Code, Anthropic's official CLI for Claude.";

pub(crate) fn strip_mcp_name_prefix_bytes(bytes: &Bytes) -> Bytes {
    let text = String::from_utf8_lossy(bytes);
    let cleaned = MCP_NAME_PREFIX_RE.replace_all(&text, r#""name": "$1""#);
    Bytes::from(cleaned.into_owned())
}

fn sanitize_brand_text(s: &str) -> String {
    BRAND_SANITIZE_RE.replace_all(s, "Claude Code").into_owned()
}

/// 统一 cache_control 为标准 5m ttl
fn normalize_cache_control(item: &mut Value) {
    if let Some(obj) = item.as_object_mut() {
        if obj.contains_key("cache_control") {
            obj.insert(
                "cache_control".to_string(),
                json!({ "type": "ephemeral", "ttl": "5m" }),
            );
        }
    }
}

/// 最大响应体大小（5MB）
const MAX_RESPONSE_SIZE: usize = 5 * 1024 * 1024;

#[derive(Debug)]
pub struct AmpHeadersProcessor;

#[derive(Debug, Clone, Copy, PartialEq)]
enum ApiType {
    AmpInternal,
    Claude,
    Codex,
    Gemini,
}

impl AmpHeadersProcessor {
    fn detect_api_type(path: &str, headers: &HyperHeaderMap, body: &[u8]) -> ApiType {
        let path_lower = path.to_lowercase();

        // 1. /api/provider/{provider}/* → LLM 端点
        if path_lower.starts_with("/api/provider/anthropic") {
            return ApiType::Claude;
        }
        if path_lower.starts_with("/api/provider/openai") {
            return ApiType::Codex;
        }
        if path_lower.starts_with("/api/provider/google") {
            return ApiType::Gemini;
        }

        // 2. 其他 /api/* → ampcode.com
        if path_lower.starts_with("/api/") {
            return ApiType::AmpInternal;
        }

        // 3. 直接 LLM 路径
        if path_lower.contains("/messages") && !path_lower.contains("/chat/completions") {
            return ApiType::Claude;
        }
        if path_lower.contains("/chat/completions")
            || path_lower.contains("/responses")
            || path_lower.ends_with("/completions")
        {
            return ApiType::Codex;
        }
        if path_lower.contains("/v1beta")
            || path_lower.contains(":generatecontent")
            || path_lower.contains(":streamgeneratecontent")
        {
            return ApiType::Gemini;
        }

        // 4. 按 headers
        if headers.contains_key("anthropic-version") {
            return ApiType::Claude;
        }

        // 5. 按 body.model
        if let Some(api_type) = Self::detect_by_model(body) {
            return api_type;
        }

        ApiType::Claude
    }

    fn detect_by_model(body: &[u8]) -> Option<ApiType> {
        if body.is_empty() {
            return None;
        }
        let json: serde_json::Value = serde_json::from_slice(body).ok()?;
        let model = json.get("model")?.as_str()?.to_lowercase();

        if model.contains("gemini") {
            Some(ApiType::Gemini)
        } else if model.contains("claude") {
            Some(ApiType::Claude)
        } else if model.contains("gpt") {
            Some(ApiType::Codex)
        } else {
            None
        }
    }

    fn extract_model_name(path: &str, body: &[u8]) -> String {
        // 1. 从路径提取：/v1beta/models/{model}:xxx
        if let Some(start) = path.find("/models/") {
            let after = &path[start + 8..];
            if let Some(end) = after.find(':') {
                return after[..end].to_string();
            }
            if let Some(end) = after.find('/') {
                return after[..end].to_string();
            }
            return after.to_string();
        }

        // 2. 从请求体提取
        if !body.is_empty() {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(body) {
                if let Some(model) = json.get("model").and_then(|m| m.as_str()) {
                    return model.to_string();
                }
            }
        }

        "gemini-2.0-flash".to_string()
    }

    /// 提取 LLM API 路径：/api/provider/xxx/v1/... → /v1/...
    /// Gemini 特殊处理：/v1beta1/publishers/google/models/xxx → /v1beta/models/xxx
    fn extract_llm_path(path: &str) -> String {
        // Gemini 路径转换：v1beta1/publishers/google/models/xxx → v1beta/models/xxx
        if let Some(pos) = path.find("/v1beta1/publishers/google/models/") {
            let model_part = &path[pos + "/v1beta1/publishers/google/models/".len()..];
            return format!("/v1beta/models/{}", model_part);
        }

        // 标准路径提取
        if let Some(pos) = path.find("/v1beta") {
            return path[pos..].to_string();
        }
        if let Some(pos) = path.find("/v1") {
            return path[pos..].to_string();
        }

        path.to_string()
    }

    fn get_user_agent(api_type: ApiType, path: &str, body: &[u8]) -> String {
        match api_type {
            ApiType::Claude => "claude-cli/2.1.2 (external, cli)".to_string(),
            ApiType::Codex => {
                "codex_cli_rs/0.77.0 (Mac OS 15.7.2; arm64) Apple_Terminal/455.1".to_string()
            }
            ApiType::Gemini => {
                let model = Self::extract_model_name(path, body);
                format!("GeminiCLI/0.22.5/{} (darwin; arm64)", model)
            }
            ApiType::AmpInternal => unreachable!(),
        }
    }

    fn add_tool_prefix(body: &[u8]) -> Vec<u8> {
        const TOOL_PREFIX: &str = "mcp_";

        if body.is_empty() {
            return body.to_vec();
        }

        let Ok(mut json) = serde_json::from_slice::<serde_json::Value>(body) else {
            return body.to_vec();
        };

        if let Some(model) = json.get("model").and_then(|m| m.as_str()) {
            if model.to_lowercase().contains("haiku") {
                return body.to_vec();
            }
        }

        // 0) system 文本清洗 + 注入 Claude Code 身份声明（对齐 JS 插件行为）
        // - 文本清洗：OpenCode/opencode/ampcode/amp-code/amp（不区分大小写）
        // - 注入：将声明插到 system 最前面
        // - 统一 cache_control 为 5m
        if let Some(system) = json.get_mut("system") {
            match system {
                serde_json::Value::Array(items) => {
                    for item in items.iter_mut() {
                        normalize_cache_control(item);

                        if item.get("type").and_then(|t| t.as_str()) != Some("text") {
                            continue;
                        }
                        if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                            item["text"] = serde_json::Value::String(sanitize_brand_text(text));
                        }
                    }

                    let already_prefixed = items
                        .first()
                        .and_then(|v| v.get("type").and_then(|t| t.as_str()))
                        == Some("text")
                        && items
                            .first()
                            .and_then(|v| v.get("text").and_then(|t| t.as_str()))
                            == Some(CLAUDE_CODE_PREAMBLE);
                    if !already_prefixed {
                        items.insert(0, json!({ "type": "text", "text": CLAUDE_CODE_PREAMBLE }));
                    }
                }
                serde_json::Value::String(s) => {
                    let cleaned = sanitize_brand_text(s);
                    if !cleaned.starts_with(CLAUDE_CODE_PREAMBLE) {
                        *s = format!("{}\n{}", CLAUDE_CODE_PREAMBLE, cleaned);
                    } else {
                        *s = cleaned;
                    }
                }
                _ => {
                    // 其他格式不处理
                }
            }
        } else {
            json["system"] = json!([{ "type": "text", "text": CLAUDE_CODE_PREAMBLE }]);
        }

        // 1) tools[].name 加前缀 + 统一 cache_control
        if let Some(tools) = json.get_mut("tools").and_then(|t| t.as_array_mut()) {
            for tool in tools.iter_mut() {
                normalize_cache_control(tool);

                if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
                    if !name.starts_with(TOOL_PREFIX) {
                        tool["name"] =
                            serde_json::Value::String(format!("{}{}", TOOL_PREFIX, name));
                    }
                }
            }
        }

        // 2) messages[].content[] 里 type=="tool_use" 的 name 也要加前缀 + 统一所有 content item 的 cache_control
        if let Some(messages) = json.get_mut("messages").and_then(|m| m.as_array_mut()) {
            for msg in messages.iter_mut() {
                let Some(content) = msg.get_mut("content") else {
                    continue;
                };

                let Some(arr) = content.as_array_mut() else {
                    continue;
                };

                for item in arr.iter_mut() {
                    normalize_cache_control(item);

                    if item.get("type").and_then(|t| t.as_str()) != Some("tool_use") {
                        continue;
                    }

                    if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                        if !name.starts_with(TOOL_PREFIX) {
                            item["name"] =
                                serde_json::Value::String(format!("{}{}", TOOL_PREFIX, name));
                        }
                    }
                }
            }
        }

        serde_json::to_vec(&json).unwrap_or_else(|_| body.to_vec())
    }

    async fn forward_to_amp(
        path: &str,
        query: Option<&str>,
        headers: &HyperHeaderMap,
        body: &[u8],
    ) -> Result<ProcessedRequest> {
        let proxy_mgr = crate::services::proxy_config_manager::ProxyConfigManager::new()
            .map_err(|e| anyhow!("ProxyConfigManager 初始化失败: {}", e))?;

        let config = proxy_mgr
            .get_config("amp-code")
            .map_err(|e| anyhow!("读取配置失败: {}", e))?
            .ok_or_else(|| anyhow!("AMP Code 代理未配置"))?;

        let token = config
            .real_api_key
            .ok_or_else(|| anyhow!("AMP Code Access Token 未配置"))?;

        let base_url = config
            .real_base_url
            .unwrap_or_else(|| "https://ampcode.com".to_string());

        let target_url = match query {
            Some(q) => format!("{}{}?{}", base_url, path, q),
            None => format!("{}{}", base_url, path),
        };

        tracing::info!("AMP Code → ampcode.com: {}", target_url);

        let mut new_headers = headers.clone();
        new_headers.remove(hyper::header::AUTHORIZATION);
        let x_api_key = hyper::header::HeaderName::from_static("x-api-key");
        new_headers.remove(&x_api_key);
        new_headers.insert(
            hyper::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().unwrap(),
        );
        new_headers.insert(x_api_key, token.parse().unwrap());

        Ok(ProcessedRequest {
            target_url,
            headers: new_headers,
            body: body.to_vec().into(),
        })
    }

    /// 检测是否为本地工具请求（精确匹配，避免误判）
    fn detect_local_tool(query: Option<&str>) -> Option<&'static str> {
        let q = query?;
        // 精确匹配：query string 必须等于工具名或以 & 分隔
        // 支持格式：?webSearch2 或 ?webSearch2&xxx 或 ?xxx&webSearch2
        let parts: Vec<&str> = q.split('&').collect();
        for part in parts {
            let key = part.split('=').next().unwrap_or(part);
            match key {
                "webSearch2" => return Some("webSearch2"),
                "extractWebPageContent" => return Some("extractWebPageContent"),
                _ => continue,
            }
        }
        None
    }

    /// 处理本地工具请求
    async fn handle_local_tool(
        tool_name: &str,
        body: &[u8],
        tavily_api_key: Option<&str>,
    ) -> Result<ProcessedRequest> {
        match tool_name {
            "webSearch2" => Self::handle_web_search(body, tavily_api_key).await,
            "extractWebPageContent" => Self::handle_extract_web_page(body).await,
            _ => Err(anyhow!("未知的本地工具: {}", tool_name)),
        }
    }

    /// 处理网页搜索请求
    async fn handle_web_search(
        body: &[u8],
        tavily_api_key: Option<&str>,
    ) -> Result<ProcessedRequest> {
        // 解析请求 JSON（不吞掉错误）
        let req_json: Value =
            serde_json::from_slice(body).map_err(|e| anyhow!("请求 JSON 解析失败: {}", e))?;
        let params = &req_json["params"];

        let objective = params["objective"].as_str().unwrap_or("");
        let search_queries: Vec<&str> = params["searchQueries"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();
        let max_results = params["maxResults"].as_i64().unwrap_or(5) as usize;

        // 构建查询列表
        let queries: Vec<&str> = if search_queries.is_empty() && !objective.is_empty() {
            vec![objective]
        } else {
            search_queries
        };

        tracing::info!(
            "本地搜索: queries={:?}, max_results={}",
            queries,
            max_results
        );

        // 尝试 Tavily，无 Key 则降级 DuckDuckGo
        let (results, provider) = if let Some(api_key) = tavily_api_key {
            tracing::info!("使用 Tavily 搜索服务");
            match Self::search_tavily(&queries, max_results, api_key).await {
                Ok(r) => (r, "tavily"),
                Err(e) => {
                    tracing::warn!("Tavily 搜索失败，降级 DuckDuckGo: {}", e);
                    (
                        Self::search_duckduckgo(&queries, max_results).await?,
                        "local-duckduckgo",
                    )
                }
            }
        } else {
            tracing::info!("使用 DuckDuckGo 本地搜索（未配置 Tavily API Key）");
            (
                Self::search_duckduckgo(&queries, max_results).await?,
                "local-duckduckgo",
            )
        };

        let response = json!({
            "ok": true,
            "result": {
                "results": results,
                "provider": provider,
                "showParallelAttribution": false
            },
            "creditsConsumed": "0"
        });

        tracing::info!("本地搜索完成: {} 条结果", results.len());
        Self::build_local_response("webSearch2", response)
    }

    /// Tavily 搜索（使用全局 Client）
    async fn search_tavily(
        queries: &[&str],
        max_results: usize,
        api_key: &str,
    ) -> Result<Vec<Value>> {
        let mut all_results = Vec::new();
        let mut seen_urls = std::collections::HashSet::new();

        for query in queries {
            if all_results.len() >= max_results {
                break;
            }

            let request_body = json!({
                "api_key": api_key,
                "query": query,
                "search_depth": "basic",
                "max_results": max_results.min(10),
                "include_answer": false
            });

            let resp = HTTP_CLIENT
                .post("https://api.tavily.com/search")
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(anyhow!("Tavily API 错误: {} - {}", status, text));
            }

            let data: Value = resp.json().await?;
            if let Some(results) = data["results"].as_array() {
                for r in results {
                    let url = r["url"].as_str().unwrap_or("");
                    if seen_urls.contains(url) {
                        continue;
                    }
                    seen_urls.insert(url.to_string());

                    all_results.push(json!({
                        "title": r["title"].as_str().unwrap_or(""),
                        "url": url,
                        "excerpts": [r["content"].as_str().unwrap_or("")]
                    }));

                    if all_results.len() >= max_results {
                        break;
                    }
                }
            }
        }

        Ok(all_results)
    }

    /// DuckDuckGo HTML 搜索（降级方案，使用全局 Client）
    async fn search_duckduckgo(queries: &[&str], max_results: usize) -> Result<Vec<Value>> {
        let mut all_results = Vec::new();
        let mut seen_urls = std::collections::HashSet::new();

        for query in queries {
            if all_results.len() >= max_results {
                break;
            }

            let url = format!(
                "https://html.duckduckgo.com/html/?q={}",
                urlencoding::encode(query)
            );

            let resp = HTTP_CLIENT
                .get(&url)
                .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36")
                .header("Accept", "text/html")
                .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
                .send()
                .await?;

            let html = resp.text().await?;
            let parsed = Self::parse_duckduckgo_html(&html);

            for r in parsed {
                if seen_urls.contains(&r.url) {
                    continue;
                }
                seen_urls.insert(r.url.clone());

                all_results.push(json!({
                    "title": r.title,
                    "url": r.url,
                    "excerpts": if r.snippet.is_empty() { vec![] } else { vec![r.snippet] }
                }));

                if all_results.len() >= max_results {
                    break;
                }
            }
        }

        Ok(all_results)
    }

    /// 解析 DuckDuckGo HTML 结果
    fn parse_duckduckgo_html(html: &str) -> Vec<DuckDuckGoResult> {
        let mut results = Vec::new();

        // 简单解析：查找 class="result__a" 的链接
        for part in html.split("class=\"result__a\"").skip(1) {
            // 提取 URL
            let url = if let Some(start) = part.find("href=\"") {
                let after = &part[start + 6..];
                if let Some(end) = after.find('"') {
                    Self::extract_ddg_actual_url(&after[..end])
                } else {
                    continue;
                }
            } else {
                continue;
            };

            if url.is_empty() {
                continue;
            }

            // 提取标题
            let title = if let Some(start) = part.find('>') {
                let after = &part[start + 1..];
                if let Some(end) = after.find("</a>") {
                    Self::clean_html(&after[..end])
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            // 提取摘要
            let snippet = if let Some(snip_start) = part.find("result__snippet") {
                let snip_part = &part[snip_start..];
                if let Some(start) = snip_part.find('>') {
                    let after = &snip_part[start + 1..];
                    if let Some(end) = after.find("</a>") {
                        Self::clean_html(&after[..end])
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            results.push(DuckDuckGoResult {
                title,
                url,
                snippet,
            });
        }

        results
    }

    /// 从 DuckDuckGo 重定向 URL 提取实际 URL
    fn extract_ddg_actual_url(ddg_url: &str) -> String {
        if ddg_url.contains("uddg=") {
            if let Some(pos) = ddg_url.find("uddg=") {
                let encoded = &ddg_url[pos + 5..];
                let end = encoded.find('&').unwrap_or(encoded.len());
                if let Ok(decoded) = urlencoding::decode(&encoded[..end]) {
                    return decoded.into_owned();
                }
            }
        }
        if ddg_url.starts_with("http") {
            ddg_url.to_string()
        } else {
            String::new()
        }
    }

    /// 清理 HTML 标签和实体
    fn clean_html(s: &str) -> String {
        let mut result = s.to_string();
        // 移除 HTML 标签
        while let Some(start) = result.find('<') {
            if let Some(end) = result[start..].find('>') {
                result = format!("{}{}", &result[..start], &result[start + end + 1..]);
            } else {
                break;
            }
        }
        // 解码常见 HTML 实体
        result = result
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&nbsp;", " ");
        result.trim().to_string()
    }

    /// 处理网页内容提取请求（增强 SSRF 防护 + 流式读取）
    async fn handle_extract_web_page(body: &[u8]) -> Result<ProcessedRequest> {
        // 解析请求 JSON（不吞掉错误）
        let req_json: Value =
            serde_json::from_slice(body).map_err(|e| anyhow!("请求 JSON 解析失败: {}", e))?;
        let target_url = req_json["params"]["url"]
            .as_str()
            .ok_or_else(|| anyhow!("缺少 URL 参数"))?;

        // SSRF 防护：使用 URL 解析进行精确校验
        Self::validate_url_security(target_url)?;

        tracing::info!("本地网页提取: {}", target_url);

        let resp = HTTP_CLIENT
            .get(target_url)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36")
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow!("HTTP {}", resp.status()));
        }

        // 流式读取并限制大小（防止 chunked 编码绕过 Content-Length 检查）
        let html = Self::read_response_with_limit(resp, MAX_RESPONSE_SIZE).await?;

        // 返回原始 HTML（与 AMP-Manager 行为一致）
        let response = json!({
            "ok": true,
            "result": {
                "fullContent": html,
                "excerpts": [],
                "provider": "local"
            }
        });

        tracing::info!("本地网页提取完成: {} bytes", html.len());
        Self::build_local_response("extractWebPageContent", response)
    }

    /// URL 安全校验（SSRF 防护）
    fn validate_url_security(url_str: &str) -> Result<()> {
        // 解析 URL
        let url = Url::parse(url_str).map_err(|e| anyhow!("URL 解析失败: {}", e))?;

        // 只允许 http/https
        match url.scheme() {
            "http" | "https" => {}
            scheme => return Err(anyhow!("不支持的协议: {}", scheme)),
        }

        // 禁止 userinfo（防止 http://good.com@evil.com 绕过）
        if url.username() != "" || url.password().is_some() {
            return Err(anyhow!("URL 不允许包含用户名/密码"));
        }

        // 检查 host
        let host = url.host_str().ok_or_else(|| anyhow!("URL 缺少主机名"))?;

        // 检查是否为 IP 地址
        if let Ok(ip) = host.parse::<IpAddr>() {
            if Self::is_private_ip(&ip) {
                return Err(anyhow!("禁止访问内网地址"));
            }
        } else {
            // 域名检查：禁止常见内网域名
            let host_lower = host.to_lowercase();
            if host_lower == "localhost"
                || host_lower.ends_with(".local")
                || host_lower.ends_with(".internal")
                || host_lower.ends_with(".localhost")
            {
                return Err(anyhow!("禁止访问内网域名"));
            }
        }

        Ok(())
    }

    /// 检查是否为私有/保留 IP 地址
    fn is_private_ip(ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => {
                ipv4.is_loopback()           // 127.0.0.0/8
                    || ipv4.is_private()     // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
                    || ipv4.is_link_local()  // 169.254.0.0/16
                    || ipv4.is_broadcast()   // 255.255.255.255
                    || ipv4.is_unspecified() // 0.0.0.0
                    || ipv4.is_multicast()   // 224.0.0.0/4
                    || ipv4.octets()[0] == 100 && (ipv4.octets()[1] & 0xc0) == 64
                // 100.64.0.0/10 (CGN)
            }
            IpAddr::V6(ipv6) => {
                ipv6.is_loopback()      // ::1
                    || ipv6.is_unspecified() // ::
                    || ipv6.is_multicast()
                    // IPv6 私有地址范围
                    || (ipv6.segments()[0] & 0xfe00) == 0xfc00 // fc00::/7 (ULA)
                    || (ipv6.segments()[0] & 0xffc0) == 0xfe80 // fe80::/10 (link-local)
            }
        }
    }

    /// 流式读取响应并限制大小
    async fn read_response_with_limit(resp: reqwest::Response, max_size: usize) -> Result<String> {
        let mut stream = resp.bytes_stream();
        let mut data = Vec::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| anyhow!("读取响应失败: {}", e))?;
            if data.len() + chunk.len() > max_size {
                return Err(anyhow!("响应体过大，超过 {} bytes 限制", max_size));
            }
            data.extend_from_slice(&chunk);
        }

        String::from_utf8(data).map_err(|e| anyhow!("响应不是有效的 UTF-8: {}", e))
    }

    /// 构建本地处理响应
    fn build_local_response(tool_name: &str, response: Value) -> Result<ProcessedRequest> {
        let body_bytes = serde_json::to_vec(&response)?;
        let mut headers = HyperHeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());

        Ok(ProcessedRequest {
            target_url: format!("dc-local://{}", tool_name),
            headers,
            body: Bytes::from(body_bytes),
        })
    }

    /// 生成 64 位 hex 用户指纹：SHA256(API_Key + UA)
    /// 使用完整 API Key 避免不同 key 碰撞，UA 作为辅助区分
    fn generate_user_hash(headers: &HyperHeaderMap, api_key: &str) -> String {
        let ua = headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");

        let mut hasher = Sha256::new();
        // 使用完整 API Key 作为主要标识，UA 作为辅助
        hasher.update(format!("{}:{}", api_key, ua));
        format!("{:x}", hasher.finalize())
    }

    /// 生成 UUID 格式会话标识
    /// - 有消息内容：基于前3条消息的 SHA256（支持会话复用）
    /// - 无消息内容：使用随机 UUID v4（避免碰撞）
    fn generate_session_uuid(messages: &Value) -> String {
        let content = messages
            .as_array()
            .map(|arr| {
                arr.iter()
                    .take(3)
                    .filter_map(|m| {
                        // 支持字符串和数组格式的 content
                        let c = m.get("content")?;
                        if let Some(s) = c.as_str() {
                            Some(s.to_string())
                        } else {
                            // 多模态内容：提取 text 类型
                            c.as_array().map(|arr| {
                                arr.iter()
                                    .filter_map(|item| {
                                        if item.get("type")?.as_str()? == "text" {
                                            item.get("text")?.as_str().map(|s| s.to_string())
                                        } else {
                                            None
                                        }
                                    })
                                    .collect::<Vec<_>>()
                                    .join("")
                            })
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("|")
            })
            .unwrap_or_default();

        // 空消息时使用随机 UUID，避免所有空请求共享同一 session
        if content.is_empty() {
            return Uuid::new_v4().to_string();
        }

        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash = format!("{:x}", hasher.finalize());

        // 转 UUID 格式：8-4-4-4-12
        format!(
            "{}-{}-{}-{}-{}",
            &hash[0..8],
            &hash[8..12],
            &hash[12..16],
            &hash[16..20],
            &hash[20..32]
        )
    }

    /// 注入 metadata.user_id 并保持字段顺序
    fn inject_metadata_with_order(body: &[u8], user_id: &str) -> Result<Vec<u8>> {
        let json: Value = serde_json::from_slice(body)?;
        let obj = json
            .as_object()
            .ok_or_else(|| anyhow!("请求体不是 JSON 对象"))?;

        // 定义字段顺序（官方顺序）
        let field_order = [
            "model",
            "system",
            "messages",
            "tools",
            "metadata",
            "max_tokens",
            "temperature",
            "top_p",
            "top_k",
            "thinking",
            "stream",
        ];

        let mut ordered: Map<String, Value> = Map::new();

        // 按顺序插入已有字段
        for &key in &field_order {
            if let Some(val) = obj.get(key) {
                if key == "metadata" {
                    // 注入 user_id 到 metadata
                    let mut meta = val.as_object().cloned().unwrap_or_default();
                    if !meta.contains_key("user_id") {
                        meta.insert("user_id".into(), json!(user_id));
                    }
                    ordered.insert(key.into(), Value::Object(meta));
                } else {
                    ordered.insert(key.into(), val.clone());
                }
            } else if key == "metadata" {
                // metadata 不存在则创建
                ordered.insert(key.into(), json!({ "user_id": user_id }));
            }
        }

        // 保留其他未知字段（放末尾）
        for (k, v) in obj.iter() {
            if !ordered.contains_key(k) {
                ordered.insert(k.clone(), v.clone());
            }
        }

        Ok(serde_json::to_vec(&ordered)?)
    }
}

/// DuckDuckGo 搜索结果
struct DuckDuckGoResult {
    title: String,
    url: String,
    snippet: String,
}

#[async_trait]
impl RequestProcessor for AmpHeadersProcessor {
    fn tool_id(&self) -> &str {
        "amp-code"
    }

    async fn process_outgoing_request(
        &self,
        _base_url: &str,
        _api_key: &str,
        path: &str,
        query: Option<&str>,
        original_headers: &HyperHeaderMap,
        body: &[u8],
    ) -> Result<ProcessedRequest> {
        // 0. 本地工具拦截：webSearch2 / extractWebPageContent
        if let Some(tool_name) = Self::detect_local_tool(query) {
            tracing::info!("AMP Code 本地工具: {}", tool_name);

            // 获取 Tavily API Key（如果配置了）
            let tavily_api_key = crate::services::proxy_config_manager::ProxyConfigManager::new()
                .ok()
                .and_then(|mgr| mgr.get_config("amp-code").ok().flatten())
                .and_then(|cfg| cfg.tavily_api_key);

            return Self::handle_local_tool(tool_name, body, tavily_api_key.as_deref()).await;
        }

        let api_type = Self::detect_api_type(path, original_headers, body);
        tracing::debug!("AMP Code 路由: path={}, type={:?}", path, api_type);

        if api_type == ApiType::AmpInternal {
            return Self::forward_to_amp(path, query, original_headers, body).await;
        }

        // LLM 请求 → 用户配置的 Profile
        let profile_mgr =
            ProfileManager::new().map_err(|e| anyhow!("ProfileManager 初始化失败: {}", e))?;

        let (claude, codex, gemini) = profile_mgr
            .resolve_amp_selection()
            .map_err(|e| anyhow!("Profile 解析失败: {}", e))?;

        let llm_path = Self::extract_llm_path(path);

        match api_type {
            ApiType::Claude => {
                let p = claude.ok_or_else(|| anyhow!("未配置 Claude Profile"))?;
                tracing::info!("AMP Code → Claude: {}{}", p.base_url, llm_path);
                let prefixed_body = Self::add_tool_prefix(body);

                // 检查并注入 metadata.user_id
                let final_body = if let Ok(json) = serde_json::from_slice::<Value>(&prefixed_body) {
                    let has_user_id = json
                        .get("metadata")
                        .and_then(|m| m.get("user_id"))
                        .and_then(|u| u.as_str())
                        .map(|s| !s.is_empty())
                        .unwrap_or(false);

                    if has_user_id {
                        // 已有 user_id，保持原样
                        prefixed_body
                    } else {
                        // 生成 user_id: user_{64位hex}_account__session_{uuid}
                        let user_hash = Self::generate_user_hash(original_headers, &p.api_key);
                        let session_uuid = Self::generate_session_uuid(&json["messages"]);
                        let user_id =
                            format!("user_{}_account__session_{}", user_hash, session_uuid);

                        tracing::debug!("AMP Code 生成 user_id: {}", user_id);

                        // 注入并保持字段顺序
                        Self::inject_metadata_with_order(&prefixed_body, &user_id)
                            .unwrap_or(prefixed_body)
                    }
                } else {
                    prefixed_body
                };

                let mut result = ClaudeHeadersProcessor
                    .process_outgoing_request(
                        &p.base_url,
                        &p.api_key,
                        &llm_path,
                        query,
                        original_headers,
                        &final_body,
                    )
                    .await?;

                let amp_headers: Vec<_> = result
                    .headers
                    .keys()
                    .filter(|k| k.as_str().starts_with("x-amp-"))
                    .cloned()
                    .collect();
                for key in amp_headers {
                    result.headers.remove(&key);
                }

                result.headers.remove("content-length");
                result.headers.remove("transfer-encoding");

                result.headers.insert(
                    "user-agent",
                    Self::get_user_agent(api_type, path, body).parse().unwrap(),
                );
                result.headers.insert("x-app", "cli".parse().unwrap());

                // 保留调用方传入的 anthropic-beta，同时确保必需 beta 存在（对齐 JS 插件行为）
                {
                    const REQUIRED_BETAS: [&str; 2] =
                        ["oauth-2025-04-20", "interleaved-thinking-2025-05-14"];
                    let incoming = result
                        .headers
                        .get("anthropic-beta")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("");

                    let mut betas = std::collections::BTreeSet::new();
                    for b in incoming
                        .split(',')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                    {
                        betas.insert(b.to_string());
                    }
                    for b in REQUIRED_BETAS {
                        betas.insert(b.to_string());
                    }

                    if !betas.is_empty() {
                        let merged = betas.into_iter().collect::<Vec<_>>().join(",");
                        result
                            .headers
                            .insert("anthropic-beta", merged.parse().unwrap());
                    }
                }

                if !result.target_url.contains("beta=true") {
                    if result.target_url.contains('?') {
                        result.target_url.push_str("&beta=true");
                    } else {
                        result.target_url.push_str("?beta=true");
                    }
                }

                Ok(result)
            }
            ApiType::Codex => {
                let p = codex.ok_or_else(|| anyhow!("未配置 Codex Profile"))?;
                let cleaned_body = if body.is_empty() {
                    None
                } else {
                    match serde_json::from_slice::<Value>(body) {
                        Ok(mut json_body) => {
                            let mut modified = false;

                            // 移除 max_output_tokens
                            if json_body
                                .as_object_mut()
                                .and_then(|obj| obj.remove("max_output_tokens"))
                                .is_some()
                            {
                                modified = true;
                            }

                            // 注入 instructions：从 input 数组中提取 role=system 的 content（含空字符串）
                            if let Some(obj) = json_body.as_object() {
                                if let Some(input) = obj.get("input").and_then(|v| v.as_array()) {
                                    // 检查是否存在 role=system 的消息
                                    let has_system = input.iter().any(|item| {
                                        item.get("role")
                                            .and_then(|r| r.as_str())
                                            .map(|r| r == "system")
                                            .unwrap_or(false)
                                    });

                                    if has_system {
                                        let system_content: String = input
                                            .iter()
                                            .filter(|item| {
                                                item.get("role")
                                                    .and_then(|r| r.as_str())
                                                    .map(|r| r == "system")
                                                    .unwrap_or(false)
                                            })
                                            .filter_map(|item| {
                                                item.get("content").and_then(|c| c.as_str())
                                            })
                                            .collect::<Vec<_>>()
                                            .join("\n\n");

                                        // 按顺序重建 JSON：model → instructions → 其他字段
                                        let mut ordered = Map::new();
                                        if let Some(model) = obj.get("model") {
                                            ordered.insert("model".to_string(), model.clone());
                                        }
                                        ordered.insert(
                                            "instructions".to_string(),
                                            Value::String(system_content),
                                        );
                                        for (k, v) in obj.iter() {
                                            if k != "model" && k != "instructions" {
                                                ordered.insert(k.clone(), v.clone());
                                            }
                                        }
                                        json_body = Value::Object(ordered);
                                        modified = true;
                                        tracing::debug!(
                                            "AMP Code Codex: 注入 instructions（来自 input system）"
                                        );
                                    }
                                }
                            }

                            if modified {
                                serde_json::to_vec(&json_body).ok()
                            } else {
                                None
                            }
                        }
                        Err(_) => None,
                    }
                };
                let body_to_forward: &[u8] = cleaned_body.as_deref().unwrap_or(body);
                let mut result = CodexHeadersProcessor
                    .process_outgoing_request(
                        &p.base_url,
                        &p.api_key,
                        &llm_path,
                        query,
                        original_headers,
                        body_to_forward,
                    )
                    .await?;
                if cleaned_body.is_some() {
                    result.headers.remove("content-length");
                    result.headers.remove("transfer-encoding");
                }
                tracing::info!("AMP Code → Codex: {}", result.target_url);
                result.headers.insert(
                    "user-agent",
                    Self::get_user_agent(api_type, path, body).parse().unwrap(),
                );
                Ok(result)
            }
            ApiType::Gemini => {
                let p = gemini.ok_or_else(|| anyhow!("未配置 Gemini Profile"))?;
                tracing::info!("AMP Code → Gemini: {}{}", p.base_url, llm_path);
                let mut result = GeminiHeadersProcessor
                    .process_outgoing_request(
                        &p.base_url,
                        &p.api_key,
                        &llm_path,
                        query,
                        original_headers,
                        body,
                    )
                    .await?;
                result.headers.insert(
                    "user-agent",
                    Self::get_user_agent(api_type, path, body).parse().unwrap(),
                );
                Ok(result)
            }
            ApiType::AmpInternal => unreachable!(),
        }
    }
}
