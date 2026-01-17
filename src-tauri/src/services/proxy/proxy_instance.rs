// 单个代理实例管理
//
// ProxyInstance 封装单个工具的透明代理服务实例，负责：
// - HTTP 服务器的启动和停止
// - 请求的接收和转发
// - Headers 处理的协调

use anyhow::{Context, Result};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::{Frame, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use super::headers::RequestProcessor;
use super::utils::body::{box_body, BoxBody};
use super::utils::{error_responses, loop_detector};
use crate::models::proxy_config::ToolProxyConfig;

/// 单个代理实例
pub struct ProxyInstance {
    tool_id: String,
    config: Arc<RwLock<ToolProxyConfig>>,
    processor: Arc<dyn RequestProcessor>,
    server_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    cancel_token: CancellationToken,
}

impl ProxyInstance {
    /// 创建新的代理实例
    pub fn new(
        tool_id: String,
        config: ToolProxyConfig,
        processor: Box<dyn RequestProcessor>,
    ) -> Self {
        Self {
            tool_id,
            config: Arc::new(RwLock::new(config)),
            processor: Arc::from(processor),
            server_handle: Arc::new(RwLock::new(None)),
            cancel_token: CancellationToken::new(),
        }
    }

    /// 启动代理服务
    pub async fn start(&self) -> Result<()> {
        // 检查是否已经在运行
        {
            let handle = self.server_handle.read().await;
            if handle.is_some() {
                anyhow::bail!("代理实例已在运行");
            }
        }

        let config = self.config.read().await.clone();

        // 验证配置
        if config.real_api_key.is_none() || config.real_base_url.is_none() {
            tracing::warn!(
                tool_id = %self.tool_id,
                "代理启动时缺少配置，将在运行时拦截请求"
            );
        }

        // 绑定地址
        let addr = if config.allow_public {
            SocketAddr::from(([0, 0, 0, 0], config.port))
        } else {
            SocketAddr::from(([127, 0, 0, 1], config.port))
        };

        let listener = TcpListener::bind(addr)
            .await
            .context(format!("绑定端口 {} 失败", config.port))?;

        tracing::info!(
            tool_id = %self.tool_id,
            addr = %addr,
            bind_mode = if config.allow_public { "0.0.0.0" } else { "127.0.0.1" },
            "透明代理启动成功"
        );

        let config_clone = Arc::clone(&self.config);
        let processor_clone = Arc::clone(&self.processor);
        let port = config.port;
        let tool_id = self.tool_id.clone();
        let cancel_token = self.cancel_token.clone();

        // 启动服务器
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel_token.cancelled() => {
                        tracing::debug!(tool_id = %tool_id, "代理服务器收到取消信号");
                        break;
                    }
                    result = listener.accept() => {
                        match result {
                            Ok((stream, _addr)) => {
                                let config = Arc::clone(&config_clone);
                                let processor = Arc::clone(&processor_clone);
                                let tool_id_inner = tool_id.clone();
                                let tool_id_for_error = tool_id.clone();
                                let conn_cancel = cancel_token.clone();

                                tokio::spawn(async move {
                                    let io = TokioIo::new(stream);
                                    let service = service_fn(move |req| {
                                        let config = Arc::clone(&config);
                                        let processor = Arc::clone(&processor);
                                        let tool_id = tool_id_inner.clone();
                                        async move {
                                            handle_request(req, config, processor, port, &tool_id).await
                                        }
                                    });

                                    let conn = http1::Builder::new().serve_connection(io, service);
                                    tokio::pin!(conn);

                                    // 使用 select 在连接完成或取消时退出
                                    tokio::select! {
                                        _ = conn_cancel.cancelled() => {
                                            tracing::debug!(tool_id = %tool_id_for_error, "连接被取消");
                                        }
                                        result = &mut conn => {
                                            if let Err(err) = result {
                                                if !err.is_incomplete_message() {
                                                    tracing::error!(
                                                        tool_id = %tool_id_for_error,
                                                        error = ?err,
                                                        "处理连接失败"
                                                    );
                                                }
                                            }
                                        }
                                    }
                                });
                            }
                            Err(e) => {
                                tracing::error!(
                                    tool_id = %tool_id,
                                    error = ?e,
                                    "接受连接失败"
                                );

                                // 记录连接层错误到数据库（无 session_id）
                                let manager =
                                    crate::services::token_stats::manager::TokenStatsManager::get();
                                let error_detail = format!("连接处理失败: {:?}", e);
                                let _ = manager
                                    .log_failed_request(
                                        &tool_id,
                                        "connection_error", // 通用会话 ID
                                        "global",
                                        "unknown", // 无法获取客户端 IP
                                        &[],       // 无请求体
                                        "connection_error",
                                        &error_detail,
                                        "unknown", // 无法确定响应类型
                                        None,      // 无响应时间
                                    )
                                    .await;
                            }
                        }
                    }
                }
            }
        });

        // 保存服务器句柄
        {
            let mut h = self.server_handle.write().await;
            *h = Some(handle);
        }

        Ok(())
    }

    /// 停止代理服务
    pub async fn stop(&self) -> Result<()> {
        // 1. 发送取消信号给所有连接
        self.cancel_token.cancel();

        // 2. 等待服务器任务结束
        let handle = {
            let mut h = self.server_handle.write().await;
            h.take()
        };

        if let Some(handle) = handle {
            // 等待任务结束（带超时）
            match tokio::time::timeout(std::time::Duration::from_secs(2), handle).await {
                Ok(_) => {
                    tracing::info!(tool_id = %self.tool_id, "透明代理已停止");
                }
                Err(_) => {
                    tracing::warn!(tool_id = %self.tool_id, "透明代理停止超时，强制终止");
                }
            }
        }

        Ok(())
    }

    /// 检查服务是否在运行
    pub fn is_running(&self) -> bool {
        // 使用 blocking 方式读取，因为这是同步方法
        // 在实际使用中，ProxyManager 会使用异步版本
        false // 临时实现，将在异步上下文中使用 try_read
    }

    /// 异步检查是否运行
    pub async fn is_running_async(&self) -> bool {
        let handle = self.server_handle.read().await;
        handle.is_some()
    }

    /// 更新配置（无需重启）
    pub async fn update_config(&self, new_config: ToolProxyConfig) -> Result<()> {
        let mut config = self.config.write().await;
        *config = new_config;
        tracing::info!(tool_id = %self.tool_id, "透明代理配置已更新");
        Ok(())
    }
}

/// 处理单个请求
async fn handle_request(
    req: Request<Incoming>,
    config: Arc<RwLock<ToolProxyConfig>>,
    processor: Arc<dyn RequestProcessor>,
    own_port: u16,
    tool_id: &str,
) -> Result<Response<BoxBody>, Infallible> {
    match handle_request_inner(req, config, processor, own_port, tool_id).await {
        Ok(res) => Ok(res),
        Err(e) => {
            tracing::error!(
                tool_id = %tool_id,
                error = ?e,
                "请求处理失败"
            );
            Ok(error_responses::internal_error(&e.to_string()))
        }
    }
}

async fn handle_request_inner(
    req: Request<Incoming>,
    config: Arc<RwLock<ToolProxyConfig>>,
    processor: Arc<dyn RequestProcessor>,
    own_port: u16,
    tool_id: &str,
) -> Result<Response<BoxBody>> {
    // 记录请求开始时间（用于计算响应时间）
    let start_time = std::time::Instant::now();

    // 获取配置
    let proxy_config = {
        let cfg = config.read().await;
        if cfg.real_api_key.is_none() || cfg.real_base_url.is_none() {
            return Ok(error_responses::configuration_missing(tool_id));
        }
        cfg.clone()
    };

    // 验证本地 API Key
    let auth_header = req
        .headers()
        .get("authorization")
        .or_else(|| req.headers().get("x-api-key"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let provided_key = if let Some(stripped) = auth_header.strip_prefix("Bearer ") {
        stripped
    } else if let Some(stripped) = auth_header.strip_prefix("x-api-key ") {
        stripped
    } else {
        auth_header
    };

    if let Some(local_key) = &proxy_config.local_api_key {
        if provided_key != local_key {
            return Ok(error_responses::unauthorized());
        }
    }

    // 提取请求信息（先借用，避免与后续的 collect 冲突）
    let path = req.uri().path().to_string();
    let query = req.uri().query().map(|s| s.to_string());
    let method = req.method().clone();
    let headers = req.headers().clone();

    // 拦截 count_tokens 接口，不转发到上游，直接返回权限错误
    if path == "/v1/messages/count_tokens" {
        tracing::warn!("拦截 count_tokens 请求，返回权限错误");
        let error_response = serde_json::json!({
            "type": "error",
            "error": {
                "type": "permission_error",
                "message": "count_tokens endpoint is not enabled for this channel. Please enable it in channel settings."
            }
        });
        let response_body = serde_json::to_string(&error_response)
            .unwrap_or_else(|_| r#"{"type":"error","error":{"type":"internal_error","message":"Failed to serialize error response"}}"#.to_string());

        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .header("content-type", "application/json")
            .body(box_body(Full::new(Bytes::from(response_body))))
            .map_err(|e| anyhow::anyhow!("Failed to build count_tokens error response: {}", e));
    }

    // 提取客户端IP（用于日志记录）
    let client_ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .unwrap_or("unknown")
        .to_string();

    // amp-code 在 processor 内部获取配置，这里传占位符
    let base = proxy_config
        .real_base_url
        .as_deref()
        .map(|s| s.trim_end_matches('/'))
        .unwrap_or("");

    // 读取请求体（消费 req）
    let body_bytes = if method != Method::GET && method != Method::HEAD {
        req.collect().await?.to_bytes()
    } else {
        Bytes::new()
    };

    // 使用 RequestProcessor 统一处理请求（URL + headers + body）
    // amp-code 忽略传入的 base/api_key，在内部通过 amp_selection 获取
    let processed = processor
        .process_outgoing_request(
            base,
            proxy_config.real_api_key.as_deref().unwrap_or(""),
            &path,
            query.as_deref(),
            &headers,
            &body_bytes,
        )
        .await
        .context("处理出站请求失败")?;

    // 本地工具处理：dc-local:// 协议标记的请求直接返回 body
    if processed.target_url.starts_with("dc-local://") {
        tracing::info!(
            tool_id = %tool_id,
            local_tool = %processed.target_url,
            "本地工具响应"
        );
        let mut response = Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json");

        for (name, value) in processed.headers.iter() {
            response = response.header(name.as_str(), value.as_bytes());
        }

        return Ok(response
            .body(box_body(http_body_util::Full::new(processed.body)))
            .unwrap());
    }

    // 回环检测
    if loop_detector::is_proxy_loop(&processed.target_url, own_port) {
        return Ok(error_responses::proxy_loop_detected(tool_id));
    }

    tracing::debug!(
        tool_id = %tool_id,
        method = %method,
        path = %path,
        target_url = %processed.target_url,
        "代理请求"
    );

    // 构建上游请求（使用处理后的信息）
    let mut reqwest_builder = reqwest::Client::new().request(method.clone(), &processed.target_url);

    // 应用处理后的 headers
    for (name, value) in processed.headers.iter() {
        reqwest_builder = reqwest_builder.header(name, value);
    }

    // 添加请求体
    if !processed.body.is_empty() {
        reqwest_builder = reqwest_builder.body(processed.body.to_vec());
    }

    // 发送请求
    let upstream_res = match reqwest_builder.send().await {
        Ok(res) => res,
        Err(e) => {
            // 上游请求失败，记录错误到数据库
            let processor_clone = Arc::clone(&processor);
            let client_ip_clone = client_ip.clone();
            let config_name_clone = proxy_config
                .real_profile_name
                .clone()
                .unwrap_or_else(|| "default".to_string());
            let proxy_pricing_template_id_clone = proxy_config.pricing_template_id.clone();
            let request_body_clone = processed.body.clone();
            let error_msg = e.to_string();

            // 从请求体中判断是否为流式请求
            let is_sse = serde_json::from_slice::<serde_json::Value>(&processed.body)
                .ok()
                .and_then(|json| json.get("stream").and_then(|v| v.as_bool()))
                .unwrap_or(false);

            tokio::spawn(async move {
                // 调用 record_request_log，传递 response_status=0 标记为上游失败
                let _ = processor_clone
                    .record_request_log(
                        &client_ip_clone,
                        &config_name_clone,
                        proxy_pricing_template_id_clone.as_deref(),
                        &request_body_clone,
                        0,      // response_status=0 标记上游请求失败
                        &[],    // 空响应体
                        is_sse, // 从请求体提取
                        Some(start_time.elapsed().as_millis() as i64),
                    )
                    .await;
            });

            return Err(anyhow::anyhow!("上游请求失败: {}", error_msg));
        }
    };

    // 构建响应
    let status = StatusCode::from_u16(upstream_res.status().as_u16())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    // 检查是否是 SSE 流
    let is_sse = upstream_res
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/event-stream"))
        .unwrap_or(false);

    let mut response = Response::builder().status(status);

    // 复制响应 headers
    for (name, value) in upstream_res.headers().iter() {
        response = response.header(name.as_str(), value.as_bytes());
    }

    if is_sse {
        tracing::debug!(tool_id = %tool_id, "SSE 流式响应");

        // SSE 流式响应：收集响应体并调用 processor.record_request_log
        use futures_util::StreamExt;
        use regex::Regex;
        use std::sync::{Arc, Mutex};

        let config_name = proxy_config
            .real_profile_name
            .clone()
            .unwrap_or_else(|| "default".to_string());

        let proxy_pricing_template_id = proxy_config.pricing_template_id.clone();

        // 使用 Arc<Mutex<Vec>> 在流处理过程中收集数据
        let sse_chunks = Arc::new(Mutex::new(Vec::new()));
        let sse_chunks_clone = Arc::clone(&sse_chunks);

        // 创建一个通道,在流完全消费后触发统计
        let (stream_end_tx, stream_end_rx) = tokio::sync::oneshot::channel::<()>();

        let stream = upstream_res.bytes_stream();

        // amp-code 需要移除工具名前缀
        let is_amp_code = tool_id == "amp-code";
        let prefix_regex = Regex::new(r#""name"\s*:\s*"mcp_([^"]+)""#).ok();

        // 拦截流数据并收集
        let mapped_stream = stream
            .map(move |result| {
                match &result {
                    Ok(chunk) => {
                        if let Ok(mut chunks) = sse_chunks_clone.lock() {
                            chunks.push(chunk.clone());
                        }
                    }
                    Err(_) => {
                        // 流错误 - 注意: stream_completed_clone 已被移除,不再需要通知
                    }
                }

                result
                    .map(|bytes| {
                        if is_amp_code {
                            if let Some(ref re) = prefix_regex {
                                let text = String::from_utf8_lossy(&bytes);
                                let cleaned = re.replace_all(&text, r#""name": "$1""#);
                                Frame::data(Bytes::from(cleaned.into_owned()))
                            } else {
                                Frame::data(bytes)
                            }
                        } else {
                            Frame::data(bytes)
                        }
                    })
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            })
            // 在流的最后一个元素之后插入完成信号
            .chain(futures_util::stream::once(async move {
                // 发送流完成信号
                let _ = stream_end_tx.send(());
                tracing::debug!("SSE 流已完全消费完毕,发送完成信号");
                // 返回一个永远不会被使用的 Err,会被下游过滤掉
                Err(Box::new(std::io::Error::other("__stream_end_marker__"))
                    as Box<dyn std::error::Error + Send + Sync>)
            }))
            // 过滤掉结束标记
            .filter(|item| {
                let is_end_marker = item
                    .as_ref()
                    .err()
                    .and_then(|e| e.downcast_ref::<std::io::Error>())
                    .map(|e| e.to_string().contains("__stream_end_marker__"))
                    .unwrap_or(false);
                futures_util::future::ready(!is_end_marker)
            });

        // 在流真正结束后异步记录日志
        let processor_clone = Arc::clone(&processor);
        let client_ip_clone = client_ip.clone();
        let request_body_clone = processed.body.clone();
        let response_status = status.as_u16();
        let start_time_clone = start_time;
        let proxy_pricing_template_id_clone = proxy_pricing_template_id.clone();

        tokio::spawn(async move {
            // 等待流完全消费的信号(无超时,真正等待流结束)
            match stream_end_rx.await {
                Ok(_) => {
                    tracing::info!("✓ 收到 SSE 流完成信号,流已完全消费");
                }
                Err(_) => {
                    tracing::warn!("✗ 未收到 SSE 流完成信号(sender 被 drop),可能流被提前取消");
                }
            }

            // 小延迟确保最后的 chunk 写入完成(异步锁竞争)
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            let chunks = match sse_chunks.lock() {
                Ok(guard) => guard.clone(),
                Err(e) => {
                    tracing::error!(error = ?e, "获取 SSE chunks 锁失败");
                    return;
                }
            };

            tracing::info!(
                chunks_count = chunks.len(),
                "开始处理 SSE chunks 进行 token 统计"
            );

            // 将所有 chunk 合并为完整响应体
            let mut full_data = Vec::new();
            for chunk in &chunks {
                full_data.extend_from_slice(chunk);
            }

            // 计算响应时间(从请求开始到流完全消费的总时间)
            let response_time_ms = start_time_clone.elapsed().as_millis() as i64;

            // 调用工具特定的日志记录
            if let Err(e) = processor_clone
                .record_request_log(
                    &client_ip_clone,
                    &config_name,
                    proxy_pricing_template_id_clone.as_deref(),
                    &request_body_clone,
                    response_status,
                    &full_data,
                    true, // is_sse
                    Some(response_time_ms),
                )
                .await
            {
                tracing::error!(error = ?e, "SSE 流日志记录失败");
            }
        });

        let body = http_body_util::StreamBody::new(mapped_stream);
        Ok(response.body(box_body(body)).unwrap())
    } else {
        // 普通响应：读取响应体并调用 processor.record_request_log
        let body_bytes = upstream_res.bytes().await.context("读取响应体失败")?;

        // amp-code 需要清理响应体中的工具名前缀
        let final_body = if tool_id == "amp-code" {
            let text = String::from_utf8_lossy(&body_bytes);
            let re = regex::Regex::new(r#""name"\s*:\s*"mcp_([^"]+)""#).unwrap();
            let cleaned = re.replace_all(&text, r#""name": "$1""#);
            Bytes::from(cleaned.into_owned())
        } else {
            body_bytes.clone()
        };

        // 获取配置名称
        let config_name = proxy_config
            .real_profile_name
            .clone()
            .unwrap_or_else(|| "default".to_string());

        let proxy_pricing_template_id = proxy_config.pricing_template_id.clone();

        // 异步记录日志
        let processor_clone = Arc::clone(&processor);
        let client_ip_clone = client_ip.clone();
        let request_body_clone = processed.body.clone();
        let response_body_clone = body_bytes.clone();
        let response_status = status.as_u16();
        let response_time_ms = start_time.elapsed().as_millis() as i64; // 计算响应时间

        tokio::spawn(async move {
            // 调用工具特定的日志记录
            if let Err(e) = processor_clone
                .record_request_log(
                    &client_ip_clone,
                    &config_name,
                    proxy_pricing_template_id.as_deref(),
                    &request_body_clone,
                    response_status,
                    &response_body_clone,
                    false, // is_sse
                    Some(response_time_ms),
                )
                .await
            {
                tracing::error!(error = ?e, "日志记录失败");
            }
        });

        Ok(response
            .body(box_body(http_body_util::Full::new(final_body)))
            .unwrap())
    }
}
