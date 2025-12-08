use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

/// 下载进度事件
#[derive(Debug, Clone)]
pub enum DownloadEvent {
    Started,
    Progress(u64, u64), // downloaded, total
    Completed,
    Failed(String),
    Speed(u64), // bytes per second
}

/// 文件下载器
#[derive(Clone)]
pub struct FileDownloader {}

impl FileDownloader {
    pub fn new() -> Self {
        Self {}
    }

    /// 创建 HTTP 客户端（每次调用时动态创建，以确保使用最新的代理配置）
    fn create_client() -> Result<reqwest::Client> {
        crate::http_client::build_client()
            .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {e}"))
    }

    /// 异步下载文件，支持进度回调
    pub async fn download_with_progress<F>(
        &self,
        url: &str,
        file_path: &PathBuf,
        mut progress_callback: F,
    ) -> Result<()>
    where
        F: FnMut(DownloadEvent) + Send + 'static,
    {
        // 发送开始事件
        progress_callback(DownloadEvent::Started);

        // 确保目标目录存在
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create download directory")?;
        }

        // 发起HTTP请求
        let client = Self::create_client()?;
        let response = client
            .get(url)
            .send()
            .await
            .with_context(|| format!("Failed to start download from URL: {url}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let url_str = url.to_string();

            // 尝试获取错误响应的详细信息
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error response".to_string());

            return Err(anyhow::anyhow!(
                "Download failed from {url_str}\nStatus: {status}\nError details: {error_text}"
            ));
        }

        let total_size = response.content_length();
        let mut downloaded = 0u64;
        let mut last_progress_time = Instant::now();
        let mut last_downloaded = 0u64;

        // 创建文件
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(file_path)
            .await
            .context("Failed to create download file")?;

        let mut bytes_stream = response.bytes_stream();
        use futures_util::StreamExt;

        while let Some(chunk_result) = bytes_stream.next().await {
            let chunk = chunk_result.context("Failed to read download chunk")?;

            file.write_all(&chunk)
                .await
                .context("Failed to write download chunk")?;

            downloaded += chunk.len() as u64;

            // 计算下载速度并更新进度（每秒更新一次）
            let now = Instant::now();
            if now.duration_since(last_progress_time) >= Duration::from_secs(1) {
                let elapsed = now.duration_since(last_progress_time).as_secs_f64();
                if elapsed > 0.0 {
                    let speed = ((downloaded - last_downloaded) as f64 / elapsed) as u64;
                    progress_callback(DownloadEvent::Speed(speed));
                }
                last_progress_time = now;
                last_downloaded = downloaded;
            }

            // 发送进度更新
            if let Some(total) = total_size {
                progress_callback(DownloadEvent::Progress(downloaded, total));
            }
        }

        file.flush()
            .await
            .context("Failed to flush downloaded file")?;

        // 发送完成事件
        progress_callback(DownloadEvent::Completed);

        Ok(())
    }

    /// 获取文件大小（如果支持）
    pub async fn get_file_size(&self, url: &str) -> Result<Option<u64>> {
        let client = Self::create_client()?;
        match client.head(url).send().await {
            Ok(response) => Ok(response.content_length()),
            Err(e) => {
                // 如果HEAD请求失败，可能是服务器不支持HEAD请求，记录但不阻断下载
                tracing::warn!(
                    error = ?e,
                    url = %url,
                    "HEAD 请求获取文件大小失败"
                );
                Ok(None)
            }
        }
    }
}

impl Default for FileDownloader {
    fn default() -> Self {
        Self::new()
    }
}
