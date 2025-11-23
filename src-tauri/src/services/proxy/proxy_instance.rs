// 单个代理实例管理
//
// ProxyInstance 封装单个工具的透明代理服务实例，负责：
// - HTTP 服务器的启动和停止
// - 请求的接收和转发
// - Headers 处理的协调

use anyhow::{Context, Result};
use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::body::{Body, Frame, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use pin_project_lite::pin_project;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

use super::headers::RequestProcessor;
use crate::models::ToolProxyConfig;

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
            println!(
                "⚠️  警告：{} 代理启动时缺少配置，将在运行时拦截请求",
                self.tool_id
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

        println!("🚀 {} 透明代理启动: http://{}", self.tool_id, addr);
        println!(
            "   绑定模式: {}",
            if config.allow_public {
                "允许局域网访问 (0.0.0.0)"
            } else {
                "仅本地访问 (127.0.0.1)"
            }
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
                                eprintln!("❌ {tool_id_for_error} 处理连接失败: {err:?}");
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("❌ {tool_id} 接受连接失败: {e:?}");
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
            println!("🛑 {} 透明代理已停止", self.tool_id);
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
        println!("✅ {} 透明代理配置已更新", self.tool_id);
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
            eprintln!("❌ {tool_id} 请求处理失败: {e:?}");
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(box_body(http_body_util::Full::new(Bytes::from(format!(
                    "代理错误: {e}"
                )))))
                .unwrap())
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
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("content-type", "application/json")
                .body(box_body(http_body_util::Full::new(Bytes::from(format!(
                    r#"{{
  "error": "CONFIGURATION_MISSING",
  "message": "{tool_id} 透明代理配置不完整",
  "details": "请先配置有效的 API Key 和 Base URL"
}}"#
                )))))
                .unwrap());
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
            return Ok(Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(box_body(http_body_util::Full::new(Bytes::from(
                    "Unauthorized: Invalid API Key",
                ))))
                .unwrap());
        }
    }

    // 提取请求信息（先借用，避免与后续的 collect 冲突）
    let path = req.uri().path().to_string();
    let query = req.uri().query().map(|s| s.to_string());
    let method = req.method().clone();
    let headers = req.headers().clone();

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
    let loop_urls = vec![
        format!("http://127.0.0.1:{}", own_port),
        format!("https://127.0.0.1:{}", own_port),
        format!("http://localhost:{}", own_port),
        format!("https://localhost:{}", own_port),
    ];

    for loop_url in &loop_urls {
        if processed.target_url.starts_with(loop_url) {
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("content-type", "application/json")
                .body(box_body(http_body_util::Full::new(Bytes::from(format!(
                    r#"{{
  "error": "PROXY_LOOP_DETECTED",
  "message": "{tool_id} 透明代理配置错误导致回环",
  "details": "请检查代理配置，确保 Base URL 不指向本地代理端口"
}}"#
                )))))
                .unwrap());
        }
    }

    println!(
        "🔄 {} 代理请求: {} {} -> {}",
        tool_id, method, &path, processed.target_url
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
    let upstream_res = reqwest_builder.send().await.context("上游请求失败")?;

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
        println!("📡 {tool_id} SSE 流式响应");
        use futures_util::StreamExt;

        let stream = upstream_res.bytes_stream();
        let mapped_stream = stream.map(|result| {
            result
                .map(Frame::data)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        });

        let body = http_body_util::StreamBody::new(mapped_stream);
        Ok(response.body(box_body(body)).unwrap())
    } else {
        // 普通响应
        let body_bytes = upstream_res.bytes().await.context("读取响应体失败")?;
        Ok(response
            .body(box_body(http_body_util::Full::new(body_bytes)))
            .unwrap())
    }
}

// Body 类型定义
pin_project! {
    pub struct BoxBody {
        #[pin]
        inner: Pin<Box<dyn Body<Data = Bytes, Error = Box<dyn std::error::Error + Send + Sync>> + Send>>,
    }
}

impl Body for BoxBody {
    type Data = Bytes;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.project().inner.poll_frame(cx)
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> hyper::body::SizeHint {
        self.inner.size_hint()
    }
}

// 辅助函数：创建 BoxBody
fn box_body<B>(body: B) -> BoxBody
where
    B: Body<Data = Bytes> + Send + 'static,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    BoxBody {
        inner: Box::pin(body.map_err(Into::into)),
    }
}
