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

        // 启动服务器
        let handle = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _addr)) => {
                        let config = Arc::clone(&config_clone);
                        let processor = Arc::clone(&processor_clone);
                        let tool_id_inner = tool_id.clone();
                        let tool_id_for_error = tool_id.clone();

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

                            if let Err(err) =
                                http1::Builder::new().serve_connection(io, service).await
                            {
                                tracing::error!(
                                    tool_id = %tool_id_for_error,
                                    error = ?err,
                                    "处理连接失败"
                                );
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!(
                            tool_id = %tool_id,
                            error = ?e,
                            "接受连接失败"
                        );
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
        let handle = {
            let mut h = self.server_handle.write().await;
            h.take()
        };

        if let Some(handle) = handle {
            handle.abort();
            tracing::info!(tool_id = %self.tool_id, "透明代理已停止");
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

    let base = proxy_config
        .real_base_url
        .as_ref()
        .unwrap()
        .trim_end_matches('/');

    // 读取请求体（消费 req）
    let body_bytes = if method != Method::GET && method != Method::HEAD {
        req.collect().await?.to_bytes()
    } else {
        Bytes::new()
    };

    // 使用 RequestProcessor 统一处理请求（URL + headers + body）
    let processed = processor
        .process_outgoing_request(
            base,
            proxy_config.real_api_key.as_ref().unwrap(),
            &path,
            query.as_deref(),
            &headers,
            &body_bytes,
        )
        .await
        .context("处理出站请求失败")?;

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
            return Err(anyhow::anyhow!("上游请求失败: {}", e));
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
        use std::sync::{Arc, Mutex};

        let config_name = proxy_config
            .real_profile_name
            .clone()
            .unwrap_or_else(|| "default".to_string());

        // 使用 Arc<Mutex<Vec>> 在流处理过程中收集数据
        let sse_chunks = Arc::new(Mutex::new(Vec::new()));
        let sse_chunks_clone = Arc::clone(&sse_chunks);

        let stream = upstream_res.bytes_stream();

        // 拦截流数据并收集
        let mapped_stream = stream.map(move |result| {
            if let Ok(chunk) = &result {
                if let Ok(mut chunks) = sse_chunks_clone.lock() {
                    chunks.push(chunk.clone());
                }
            }
            result
                .map(Frame::data)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        });

        // 在流结束后异步记录日志
        let processor_clone = Arc::clone(&processor);
        let client_ip_clone = client_ip.clone();
        let request_body_clone = processed.body.clone();
        let response_status = status.as_u16();

        tokio::spawn(async move {
            // 等待流结束（延迟确保所有 chunks 已收集）
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            let chunks = match sse_chunks.lock() {
                Ok(guard) => guard.clone(),
                Err(e) => {
                    tracing::error!(error = ?e, "获取 SSE chunks 锁失败");
                    return;
                }
            };

            // 将所有 chunk 合并为完整响应体
            let mut full_data = Vec::new();
            for chunk in &chunks {
                full_data.extend_from_slice(chunk);
            }

            // 调用工具特定的日志记录
            if let Err(e) = processor_clone
                .record_request_log(
                    &client_ip_clone,
                    &config_name,
                    &request_body_clone,
                    response_status,
                    &full_data,
                    true, // is_sse
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

        // 获取配置名称
        let config_name = proxy_config
            .real_profile_name
            .clone()
            .unwrap_or_else(|| "default".to_string());

        // 异步记录日志
        let processor_clone = Arc::clone(&processor);
        let client_ip_clone = client_ip.clone();
        let request_body_clone = processed.body.clone();
        let response_body_clone = body_bytes.clone();
        let response_status = status.as_u16();

        tokio::spawn(async move {
            // 调用工具特定的日志记录
            if let Err(e) = processor_clone
                .record_request_log(
                    &client_ip_clone,
                    &config_name,
                    &request_body_clone,
                    response_status,
                    &response_body_clone,
                    false, // is_sse
                )
                .await
            {
                tracing::error!(error = ?e, "日志记录失败");
            }
        });

        Ok(response
            .body(box_body(http_body_util::Full::new(body_bytes)))
            .unwrap())
    }
}
