// 透明代理服务 - 用于 ClaudeCode 账户快速切换
// 本地 HTTP 代理，拦截请求并替换 API Key 和 URL，支持 SSE 流式响应

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

// 代理配置
#[derive(Clone, Debug)]
pub struct ProxyConfig {
    pub target_api_key: String,
    pub target_base_url: String,
    pub local_api_key: String, // 用于保护本地代理的 API Key
}

// 代理服务状态
pub struct TransparentProxyService {
    config: Arc<RwLock<Option<ProxyConfig>>>,
    server_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    port: u16,
}

impl TransparentProxyService {
    pub fn new(port: u16) -> Self {
        Self {
            config: Arc::new(RwLock::new(None)),
            server_handle: Arc::new(RwLock::new(None)),
            port,
        }
    }

    /// 启动代理服务
    pub async fn start(&self, config: ProxyConfig, allow_public: bool) -> Result<()> {
        // 检查是否已经在运行
        {
            let handle = self.server_handle.read().await;
            if handle.is_some() {
                anyhow::bail!("透明代理已在运行");
            }
        }

        // 验证配置有效性 - 允许空配置，但会在运行时检查
        if config.target_api_key.is_empty() {
            tracing::warn!("透明代理启动时缺少 API Key 配置，将在运行时拦截请求");
        }

        if config.target_base_url.is_empty() {
            tracing::warn!("透明代理启动时缺少 Base URL 配置，将在运行时拦截请求");
        }

        tracing::debug!("透明代理配置加载完成");
        if !config.target_api_key.is_empty() {
            tracing::debug!(
                api_key_prefix = &config.target_api_key[..4.min(config.target_api_key.len())],
                "目标 API Key"
            );
        } else {
            tracing::debug!("目标 API Key: 未配置");
        }
        if !config.target_base_url.is_empty() {
            tracing::debug!(base_url = %config.target_base_url, "目标 Base URL");
        } else {
            tracing::debug!("目标 Base URL: 未配置");
        }

        // 保存配置
        {
            let mut cfg = self.config.write().await;
            *cfg = Some(config);
        }

        // 绑定到指定地址
        let addr = if allow_public {
            SocketAddr::from(([0, 0, 0, 0], self.port))
        } else {
            SocketAddr::from(([127, 0, 0, 1], self.port))
        };

        tracing::info!(
            bind_mode = if allow_public { "0.0.0.0" } else { "127.0.0.1" },
            "透明代理绑定模式"
        );

        let listener = TcpListener::bind(addr).await.context("绑定代理端口失败")?;

        tracing::info!(addr = %addr, "透明代理启动成功");

        let config_clone = Arc::clone(&self.config);
        let port = self.port; // 保存端口信息

        // 启动服务器
        let handle = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        let config = Arc::clone(&config_clone);
                        tokio::spawn(async move {
                            let io = TokioIo::new(stream);
                            let service = service_fn(move |req| {
                                let config = Arc::clone(&config);
                                async move { handle_request(req, config, port).await }
                            });

                            if let Err(err) =
                                http1::Builder::new().serve_connection(io, service).await
                            {
                                tracing::error!(
                                    client_addr = %addr,
                                    error = ?err,
                                    "处理连接失败"
                                );
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!(error = ?e, "接受连接失败");
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
            tracing::info!("透明代理已停止");
        }

        // 清空配置
        {
            let mut cfg = self.config.write().await;
            *cfg = None;
        }

        Ok(())
    }

    /// 检查服务是否在运行
    pub async fn is_running(&self) -> bool {
        let handle = self.server_handle.read().await;
        handle.is_some()
    }

    /// 更新配置（无需重启）
    pub async fn update_config(&self, config: ProxyConfig) -> Result<()> {
        let mut cfg = self.config.write().await;
        *cfg = Some(config);
        tracing::info!("透明代理配置已更新");
        Ok(())
    }
}

// 处理单个请求
async fn handle_request(
    req: Request<Incoming>,
    config: Arc<RwLock<Option<ProxyConfig>>>,
    own_port: u16,
) -> Result<Response<BoxBody>, Infallible> {
    match handle_request_inner(req, config, own_port).await {
        Ok(res) => Ok(res),
        Err(e) => {
            tracing::error!(error = ?e, "请求处理失败");
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
    config: Arc<RwLock<Option<ProxyConfig>>>,
    own_port: u16,
) -> Result<Response<BoxBody>> {
    // 获取配置
    let proxy_config = {
        let cfg = config.read().await;
        match cfg.as_ref() {
            Some(config) => {
                // 检查配置是否有效
                if config.target_api_key.is_empty() || config.target_base_url.is_empty() {
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .header("content-type", "application/json")
                        .body(box_body(http_body_util::Full::new(Bytes::from(r#"{
  "error": "CONFIGURATION_MISSING",
  "message": "透明代理配置不完整",
  "details": "检测到透明代理功能已开启，但缺少有效的API配置。请先在DuckCoding中选择一个有效的配置文件，然后再启动透明代理。",
  "suggestion": "请检查以下配置：\n1. 确保已选择有效的ClaudeCode配置文件\n2. 配置文件包含有效的API Key和Base URL\n3. 重新启动透明代理服务"
}"#))))
                        .unwrap());
                }
                config.clone()
            }
            None => {
                return Ok(Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .header("content-type", "application/json")
                    .body(box_body(http_body_util::Full::new(Bytes::from(r#"{
  "error": "PROXY_NOT_CONFIGURED",
  "message": "透明代理未配置",
  "details": "透明代理服务正在运行，但没有找到有效的转发配置。这可能是因为：\n1. 透明代理启动时没有备份原始配置\n2. 配置文件已损坏或丢失",
  "suggestion": "请重新启动透明代理服务以重新配置，或者在设置中禁用透明代理功能"
}"#))))
                    .unwrap());
            }
        }
    };

    // 验证本地 API Key
    let auth_header = req
        .headers()
        .get("authorization")
        .or_else(|| req.headers().get("x-api-key"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // 提取 Bearer token
    let provided_key = if let Some(stripped) = auth_header.strip_prefix("Bearer ") {
        stripped
    } else if let Some(stripped) = auth_header.strip_prefix("x-api-key ") {
        stripped
    } else {
        auth_header
    };

    if provided_key != proxy_config.local_api_key {
        return Ok(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(box_body(http_body_util::Full::new(Bytes::from(
                "Unauthorized: Invalid API Key",
            ))))
            .unwrap());
    }

    // 构建目标 URL
    let path = req.uri().path();
    let query = req
        .uri()
        .query()
        .map(|q| format!("?{q}"))
        .unwrap_or_default();

    // 确保 base_url 不包含尾部斜杠
    let base = proxy_config.target_base_url.trim_end_matches('/');

    // 如果 base_url 以 /v1 结尾，且 path 以 /v1 开头，则去掉 path 中的 /v1
    // 这是因为 Codex 的配置文件要求 base_url 包含 /v1，
    // 但 Codex 发送请求时也会带上 /v1 前缀
    let adjusted_path = if base.ends_with("/v1") && path.starts_with("/v1") {
        &path[3..] // 去掉 "/v1"
    } else {
        path
    };

    let target_url = format!("{base}{adjusted_path}{query}");

    // 回环检测 - 只检测自己的端口
    let own_proxy_url1 = format!("http://127.0.0.1:{own_port}");
    let own_proxy_url2 = format!("https://127.0.0.1:{own_port}");
    let own_proxy_url3 = format!("http://localhost:{own_port}");
    let own_proxy_url4 = format!("https://localhost:{own_port}");

    if target_url.starts_with(&own_proxy_url1)
        || target_url.starts_with(&own_proxy_url2)
        || target_url.starts_with(&own_proxy_url3)
        || target_url.starts_with(&own_proxy_url4)
    {
        tracing::error!(
            target_url = %target_url,
            proxy_port = own_port,
            "检测到透明代理回环"
        );
        return Ok(Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .header("content-type", "application/json")
            .body(box_body(http_body_util::Full::new(Bytes::from(r#"{
  "error": "PROXY_LOOP_DETECTED",
  "message": "透明代理配置错误导致回环",
  "details": "检测到透明代理正在将请求转发给自己，这通常是因为：\n1. 透明代理的真实配置未正确设置\n2. ClaudeCode配置文件中的Base URL仍指向本地代理\n3. 配置更新过程中出现同步问题",
  "suggestion": "请尝试以下解决方案：\n1. 在DuckCoding中重新选择一个有效的配置文件\n2. 确保选择的配置文件包含有效的API Key和Base URL\n3. 如果问题持续，请禁用透明代理功能并重新启用"
}"#))))
            .unwrap());
    }

    // 先获取 headers 和 method
    let method = req.method().clone();
    let headers = req.headers().clone();

    tracing::debug!(
        method = %method,
        path = %path,
        target_url = %target_url,
        base_url = %base,
        api_key_prefix = &proxy_config.target_api_key[..4.min(proxy_config.target_api_key.len())],
        "代理请求"
    );

    // 读取请求体（会消费 req）
    let body_bytes = if method != Method::GET && method != Method::HEAD {
        req.collect().await?.to_bytes()
    } else {
        Bytes::new()
    };

    // 使用 reqwest 发送请求（支持 HTTPS）
    let mut reqwest_builder = reqwest::Client::new().request(method.clone(), &target_url);

    // 复制 headers
    for (name, value) in headers.iter() {
        let name_str = name.as_str();
        if name_str.eq_ignore_ascii_case("host") {
            continue;
        }
        if name_str.eq_ignore_ascii_case("authorization")
            || name_str.eq_ignore_ascii_case("x-api-key")
        {
            reqwest_builder = reqwest_builder.header(
                "authorization",
                format!("Bearer {}", proxy_config.target_api_key),
            );
            continue;
        }
        reqwest_builder = reqwest_builder.header(name, value);
    }

    // 确保有 Authorization header
    if !headers.contains_key("authorization") && !headers.contains_key("x-api-key") {
        reqwest_builder = reqwest_builder.header(
            "authorization",
            format!("Bearer {}", proxy_config.target_api_key),
        );
    }

    // 添加请求体
    if !body_bytes.is_empty() {
        reqwest_builder = reqwest_builder.body(body_bytes.to_vec());
    }

    // 发送请求
    let upstream_res = reqwest_builder.send().await.context("上游请求失败")?;

    // 获取状态码和 headers
    let status = StatusCode::from_u16(upstream_res.status().as_u16())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    // 检查是否是 SSE 流
    let is_sse = upstream_res
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/event-stream"))
        .unwrap_or(false);

    // 构建响应
    let mut response = Response::builder().status(status);

    // 复制所有响应 headers
    for (name, value) in upstream_res.headers().iter() {
        response = response.header(name.as_str(), value.as_bytes());
    }

    if is_sse {
        tracing::debug!("SSE 流式响应");
        // SSE 流式响应 - 使用 bytes_stream
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
        // 普通响应 - 读取完整 body
        let body_bytes = upstream_res.bytes().await.context("读取响应体失败")?;
        Ok(response
            .body(box_body(http_body_util::Full::new(body_bytes)))
            .unwrap())
    }
}

// Body 类型定义
pin_project! {
    struct BoxBody {
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
