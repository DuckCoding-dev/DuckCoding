use crate::models::update::{
    DownloadProgress, DownloadTask, PackageFormatInfo, PlatformInfo as UpdatePlatformInfo,
    UpdateApiResponse, UpdateInfo, UpdateStatus, UpdateUrls,
};
use crate::services::downloader::{DownloadEvent, FileDownloader};
use anyhow::{anyhow, Context, Result};
#[cfg(target_os = "macos")]
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::fs;
use tokio::sync::{Mutex, RwLock};

#[cfg(target_os = "linux")]
use std::os::unix::fs::PermissionsExt;

/// 更新管理服务
#[derive(Clone)]
pub struct UpdateService {
    current_version: String,
    status: Arc<RwLock<UpdateStatus>>,
    download_task: Arc<Mutex<Option<DownloadTask>>>,
    downloader: FileDownloader,
    update_dir: PathBuf,
}

impl UpdateService {
    pub fn new() -> Self {
        let current_version = env!("CARGO_PKG_VERSION").to_string();

        Self {
            current_version,
            status: Arc::new(RwLock::new(UpdateStatus::Idle)),
            download_task: Arc::new(Mutex::new(None)),
            downloader: FileDownloader::new(),
            update_dir: dirs::cache_dir()
                .unwrap_or_else(|| dirs::home_dir().expect("No home directory"))
                .join("duckcoding")
                .join("updates"),
        }
    }

    /// 初始化更新服务
    pub async fn initialize(&self) -> Result<()> {
        // 确保更新目录存在
        fs::create_dir_all(&self.update_dir)
            .await
            .context("Failed to create update directory")?;

        Ok(())
    }

    /// 检查是否有可用更新
    pub async fn check_for_updates(&self) -> Result<UpdateInfo> {
        // 更新状态为检查中
        *self.status.write().await = UpdateStatus::Checking;

        let result = self.fetch_update_info().await;

        // 恢复状态
        let current_status = self.status.read().await.clone();
        if current_status == UpdateStatus::Checking {
            *self.status.write().await = UpdateStatus::Idle;
        }

        result
    }

    /// 从镜像站获取更新信息
    async fn fetch_update_info(&self) -> Result<UpdateInfo> {
        let client = crate::http_client::build_client()
            .map_err(|e| anyhow!("Failed to create HTTP client: {e}"))?;

        let response = client
            .get("https://mirror.duckcoding.com/api/v1/update")
            .send()
            .await
            .context("Failed to fetch update info")?;

        if !response.status().is_success() {
            return Err(anyhow!("Update API returned status: {}", response.status()));
        }

        let api_response: UpdateApiResponse = response
            .json()
            .await
            .context("Failed to parse update response")?;

        // 检查版本是否需要更新
        let has_update = self.compare_versions(&self.current_version, &api_response.version);

        // 获取对应平台的更新URL
        let update_url = self.get_platform_update_url(&api_response.update);

        // 获取文件大小
        let file_size = if let Some(url) = &update_url {
            self.downloader.get_file_size(url).await.ok().flatten()
        } else {
            None
        };

        Ok(UpdateInfo {
            current_version: self.current_version.clone(),
            latest_version: api_response.version,
            has_update,
            update_url,
            update: Some(api_response.update),
            release_notes: api_response.release_notes,
            file_size,
            required: api_response.required.unwrap_or(false),
        })
    }

    /// 获取当前平台的更新URL（按优先级选择最适合的包格式）
    fn get_platform_update_url(&self, update_urls: &UpdateUrls) -> Option<String> {
        #[cfg(target_os = "windows")]
        {
            // Windows 平台优先级：msi > exe > windows
            update_urls
                .windows_msi
                .clone()
                .or_else(|| update_urls.windows_exe.clone())
                .or_else(|| update_urls.windows.clone())
                .or_else(|| update_urls.universal.clone())
        }

        #[cfg(target_os = "macos")]
        {
            // macOS 平台优先级：dmg > macos
            update_urls
                .macos_dmg
                .clone()
                .or_else(|| update_urls.macos.clone())
                .or_else(|| update_urls.universal.clone())
        }

        #[cfg(target_os = "linux")]
        {
            // Linux 平台优先级：AppImage > deb > rpm > linux
            update_urls
                .linux_appimage
                .clone()
                .or_else(|| {
                    // 根据发行版选择包格式
                    if self.is_debian_based() {
                        update_urls.linux_deb.clone()
                    } else if self.is_redhat_based() {
                        update_urls.linux_rpm.clone()
                    } else {
                        None
                    }
                })
                .or_else(|| update_urls.linux.clone())
                .or_else(|| update_urls.universal.clone())
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            // 其他平台尝试通用包
            update_urls.universal.clone()
        }
    }

    /// 检测是否为基于Debian的发行版
    #[cfg(target_os = "linux")]
    fn is_debian_based(&self) -> bool {
        // 这里可以添加更复杂的检测逻辑
        // 简单检查常见的Debian包管理器
        std::path::Path::new("/etc/debian_version").exists()
            || std::path::Path::new("/usr/bin/apt").exists()
            || std::path::Path::new("/usr/bin/dpkg").exists()
    }

    /// 检测是否为基于RedHat的发行版
    #[cfg(target_os = "linux")]
    fn is_redhat_based(&self) -> bool {
        // 简单检查常见的RedHat包管理器
        std::path::Path::new("/etc/redhat-release").exists()
            || std::path::Path::new("/usr/bin/rpm").exists()
            || std::path::Path::new("/usr/bin/yum").exists()
            || std::path::Path::new("/usr/bin/dnf").exists()
    }

    /// 下载更新
    pub async fn download_update<F>(&self, url: &str, progress_callback: F) -> Result<String>
    where
        F: Fn(DownloadProgress) + Send + 'static,
    {
        let current_status = self.status.read().await.clone();
        if !self.can_download().await {
            return Err(anyhow!(
                "Cannot download update in current state: {current_status:?}"
            ));
        }

        let file_name = self.extract_filename_from_url(url)?;
        let file_path = self.update_dir.join(&file_name);

        // 创建下载任务
        let task = DownloadTask {
            url: url.to_string(),
            file_path: file_path.clone(),
            total_size: self.downloader.get_file_size(url).await.ok().flatten(),
            downloaded: 0,
            start_time: Instant::now(),
        };

        *self.download_task.lock().await = Some(task);
        *self.status.write().await = UpdateStatus::Downloading;

        // 开始下载
        let downloader = self.downloader.clone();

        let result = downloader
            .download_with_progress(url, &file_path, move |event| {
                match event {
                    DownloadEvent::Started => {
                        // 可以发送初始进度
                    }
                    DownloadEvent::Progress(downloaded, total) => {
                        let percentage = if total > 0 {
                            (downloaded as f32 / total as f32) * 100.0
                        } else {
                            0.0
                        };
                        progress_callback(DownloadProgress {
                            downloaded_bytes: downloaded,
                            total_bytes: total,
                            percentage,
                            speed: None,
                            eta: None,
                        });
                    }
                    DownloadEvent::Speed(_speed) => {
                        // 可以更新速度信息
                    }
                    DownloadEvent::Completed => {
                        // 注意：这里不能使用await，需要在异步上下文中处理
                        // 让下载器在完成后设置状态
                        progress_callback(DownloadProgress {
                            downloaded_bytes: 0,
                            total_bytes: 0,
                            percentage: 100.0,
                            speed: None,
                            eta: None,
                        });
                    }
                    DownloadEvent::Failed(error) => {
                        tracing::error!(error = %error, "下载失败");
                        // 注意：这里不能使用await，需要在异步上下文中处理
                        progress_callback(DownloadProgress {
                            downloaded_bytes: 0,
                            total_bytes: 0,
                            percentage: 0.0,
                            speed: None,
                            eta: None,
                        });
                    }
                }
            })
            .await;

        match result {
            Ok(_) => {
                *self.status.write().await = UpdateStatus::Downloaded;
                Ok(file_path.to_string_lossy().to_string())
            }
            Err(e) => {
                *self.status.write().await = UpdateStatus::Failed(e.to_string());
                // 清理部分下载的文件
                let _ = fs::remove_file(&file_path).await;
                Err(e)
            }
        }
    }

    /// 安装更新
    pub async fn install_update(&self, update_path: &str) -> Result<()> {
        let current_status = self.status.read().await.clone();
        if current_status != UpdateStatus::Downloaded {
            return Err(anyhow!("Update not downloaded yet"));
        }

        *self.status.write().await = UpdateStatus::Installing;

        // 备份当前版本
        let backup_result = self.backup_current_version().await;

        match backup_result {
            Ok(_) => {
                // 准备安装更新
                if let Err(e) = self.prepare_installation(update_path).await {
                    *self.status.write().await = UpdateStatus::Failed(e.to_string());
                    return Err(e);
                }

                *self.status.write().await = UpdateStatus::Installed;
                Ok(())
            }
            Err(e) => {
                *self.status.write().await = UpdateStatus::Failed(format!("Backup failed: {e}"));
                Err(e)
            }
        }
    }

    /// 回滚更新
    pub async fn rollback_update(&self) -> Result<()> {
        *self.status.write().await = UpdateStatus::Rollback;

        // 实现回滚逻辑
        let backup_dir = self.update_dir.join("backup");
        if backup_dir.exists() {
            // 恢复备份文件
            if let Err(e) = self.restore_from_backup(&backup_dir).await {
                *self.status.write().await = UpdateStatus::Failed(e.to_string());
                return Err(e);
            }
        }

        *self.status.write().await = UpdateStatus::RolledBack;
        Ok(())
    }

    /// 获取当前更新状态
    pub async fn get_status(&self) -> UpdateStatus {
        self.status.read().await.clone()
    }

    /// 获取当前版本
    pub fn get_current_version(&self) -> &str {
        &self.current_version
    }

    /// 获取当前平台信息
    pub fn get_platform_info(&self) -> UpdatePlatformInfo {
        UpdatePlatformInfo {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            is_windows: cfg!(target_os = "windows"),
            is_macos: cfg!(target_os = "macos"),
            is_linux: cfg!(target_os = "linux"),
        }
    }

    /// 获取推荐的包格式信息
    pub fn get_recommended_package_format(&self) -> PackageFormatInfo {
        #[cfg(target_os = "windows")]
        {
            PackageFormatInfo {
                platform: "windows".to_string(),
                preferred_formats: vec![
                    "windows_msi".to_string(),
                    "windows_exe".to_string(),
                    "windows".to_string(),
                ],
                fallback_format: "windows".to_string(),
            }
        }

        #[cfg(target_os = "macos")]
        {
            PackageFormatInfo {
                platform: "macos".to_string(),
                preferred_formats: vec!["macos_dmg".to_string(), "macos".to_string()],
                fallback_format: "macos".to_string(),
            }
        }

        #[cfg(target_os = "linux")]
        {
            let mut formats = vec!["linux_appimage".to_string()];

            if self.is_debian_based() {
                formats.push("linux_deb".to_string());
            } else if self.is_redhat_based() {
                formats.push("linux_rpm".to_string());
            }

            formats.push("linux".to_string());

            PackageFormatInfo {
                platform: "linux".to_string(),
                preferred_formats: formats,
                fallback_format: "linux".to_string(),
            }
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            PackageFormatInfo {
                platform: "unknown".to_string(),
                preferred_formats: vec!["universal".to_string()],
                fallback_format: "universal".to_string(),
            }
        }
    }

    // 私有辅助方法

    fn compare_versions(&self, current: &str, latest: &str) -> bool {
        // 简单的版本比较，实际项目中应该使用semver库
        current != latest
    }

    fn extract_filename_from_url(&self, url: &str) -> Result<String> {
        let parsed_url = url::Url::parse(url).context("Invalid update URL")?;

        let filename = parsed_url
            .path_segments()
            .and_then(|mut segments| segments.next_back())
            .ok_or_else(|| anyhow!("Cannot extract filename from URL"))?;

        Ok(filename.to_string())
    }

    async fn can_download(&self) -> bool {
        let status = self.status.read().await.clone();
        // 允许在非下载状态下进行下载
        !matches!(status, UpdateStatus::Downloading | UpdateStatus::Installing)
    }

    async fn backup_current_version(&self) -> Result<()> {
        // 实现备份逻辑
        let backup_dir = self.update_dir.join("backup");
        fs::create_dir_all(&backup_dir)
            .await
            .context("Failed to create backup directory")?;

        // 这里应该备份当前的可执行文件
        // 具体实现取决于应用结构

        Ok(())
    }

    async fn prepare_installation(&self, update_path: &str) -> Result<()> {
        // 简化版安装逻辑 - 先创建一个模拟的安装过程

        // 1. 验证文件存在
        let file_path = std::path::Path::new(update_path);
        if !file_path.exists() {
            return Err(anyhow!("Update file not found: {update_path}"));
        }

        // 2. 获取文件信息
        let metadata = tokio::fs::metadata(update_path)
            .await
            .context("Failed to read update file metadata")?;

        let file_size = metadata.len();
        tracing::info!(
            update_path = %update_path,
            file_size = file_size,
            "开始安装更新文件"
        );

        // 3. 根据文件扩展名执行安装
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        #[cfg(target_os = "windows")]
        {
            // Windows 处理
            if file_name.ends_with(".exe") {
                tracing::info!(update_path = %update_path, "启动 Windows EXE 安装程序");

                // 直接启动 EXE 安装程序，不使用任何静默参数，让用户看到标准的安装界面
                match tokio::process::Command::new(update_path).spawn() {
                    Ok(mut child) => {
                        tracing::info!("已启动安装程序，显示标准安装界面");

                        // 等待安装程序完成
                        match child.wait().await {
                            Ok(status) => {
                                if status.success() {
                                    tracing::info!("用户完成了 EXE 安装");
                                    return Ok(());
                                } else {
                                    let exit_code = status.code().unwrap_or(-1);
                                    return Err(anyhow!(
                                        "EXE installer failed with exit code: {exit_code}"
                                    ));
                                }
                            }
                            Err(e) => {
                                return Err(anyhow!("Failed to wait for EXE installer: {e}"));
                            }
                        }
                    }
                    Err(_e) => {
                        // 如果直接启动失败，尝试用资源管理器打开文件
                        tracing::warn!("直接启动 EXE 失败，尝试用资源管理器打开文件");
                        match tokio::process::Command::new("explorer")
                            .arg(update_path)
                            .spawn()
                        {
                            Ok(_) => {
                                tracing::info!("已用资源管理器打开 EXE 文件，请手动完成安装");
                                // 等待几秒钟给用户看到文件管理器
                                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                                return Ok(());
                            }
                            Err(e) => {
                                return Err(anyhow!(
                                    "Failed to start EXE installer and open file manager: {e}"
                                ));
                            }
                        }
                    }
                }
            } else if file_name.ends_with(".msi") {
                tracing::info!("执行 Windows MSI 安装程序");

                // 尝试多种 MSI 安装方式
                let install_methods = [
                    vec!["/i", update_path, "/quiet", "/norestart"], // 静默安装
                    vec!["/i", update_path, "/passive", "/norestart"], // 被动安装（显示进度）
                    vec!["/i", update_path, "/qn", "/norestart"],    // 无界面安装
                    vec!["/i", update_path],                         // 基本安装
                ];

                let mut last_error = None;

                for (i, args) in install_methods.iter().enumerate() {
                    tracing::debug!(method = i + 1, args = ?args, "尝试 MSI 安装方法");

                    match tokio::process::Command::new("msiexec").args(args).spawn() {
                        Ok(mut child) => match child.wait().await {
                            Ok(status) => {
                                if status.success() {
                                    tracing::info!(method = i + 1, "MSI 安装成功");
                                    return Ok(());
                                } else {
                                    let error_msg = format!(
                                        "MSI installer (方法 {}) failed with exit code: {:?}",
                                        i + 1,
                                        status.code()
                                    );
                                    tracing::warn!("{}", error_msg);
                                    last_error = Some(anyhow!(error_msg));
                                }
                            }
                            Err(e) => {
                                let error_msg =
                                    format!("MSI installer (方法 {}) wait failed: {}", i + 1, e);
                                tracing::warn!("{}", error_msg);
                                last_error = Some(anyhow!(error_msg));
                            }
                        },
                        Err(e) => {
                            let error_msg =
                                format!("Failed to start MSI installer (方法 {}): {}", i + 1, e);
                            tracing::warn!("{}", error_msg);
                            last_error = Some(anyhow!(error_msg));
                        }
                    }

                    // 等待一下再尝试下一种方法
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                }

                // 如果所有方法都失败了，尝试用资源管理器打开文件
                tracing::warn!("所有 MSI 安装方法都失败，尝试用资源管理器打开文件");
                match tokio::process::Command::new("explorer")
                    .arg(update_path)
                    .spawn()
                {
                    Ok(_) => {
                        tracing::info!("已用资源管理器打开 MSI 文件，请手动安装");
                        return Ok(()); // 不报错，让用户手动安装
                    }
                    Err(e) => {
                        return Err(anyhow!("All MSI installation methods failed and couldn't open file manager: {}. Last error: {}", e, last_error.unwrap_or_else(|| anyhow!("No previous error"))));
                    }
                }
            } else {
                // 其他格式，尝试打开文件资源管理器
                tracing::info!(update_path = %update_path, "尝试打开文件资源管理器");
                tokio::process::Command::new("explorer")
                    .arg("/select,")
                    .arg(update_path)
                    .spawn()
                    .context("Failed to open explorer")?;
            }
        }

        #[cfg(target_os = "macos")]
        {
            if file_name.ends_with(".dmg") {
                tracing::info!(update_path = %update_path, "准备安装 macOS DMG 包");
                self.install_dmg_package(update_path).await?;
            } else if file_name.ends_with(".pkg") {
                // macOS 安装包
                tracing::info!("执行 macOS PKG 安装程序");
                let mut child = tokio::process::Command::new("open")
                    .arg(update_path)
                    .spawn()
                    .context("Failed to open PKG installer")?;

                let status = child.wait().await.context("PKG installer failed")?;
                if !status.success() {
                    return Err(anyhow!(
                        "PKG installer failed with exit code: {:?}",
                        status.code()
                    ));
                }
            } else {
                // 其他格式，尝试用 Finder 打开
                tracing::info!(update_path = %update_path, "尝试用 Finder 打开");
                tokio::process::Command::new("open")
                    .arg(update_path)
                    .spawn()
                    .context("Failed to open file")?;
            }
        }

        #[cfg(target_os = "linux")]
        {
            if file_name.ends_with(".AppImage") {
                // AppImage 安装
                tracing::info!("设置 AppImage 执行权限");
                tokio::fs::set_permissions(update_path, std::fs::Permissions::from_mode(0o755))
                    .await
                    .context("Failed to set execute permissions")?;

                tracing::info!(update_path = %update_path, "启动 AppImage");
                let mut child = tokio::process::Command::new(update_path)
                    .spawn()
                    .context("Failed to start AppImage")?;

                let status = child.wait().await.context("AppImage execution failed")?;
                if !status.success() {
                    return Err(anyhow!(
                        "AppImage execution failed with exit code: {:?}",
                        status.code()
                    ));
                }
            } else if file_name.ends_with(".deb") {
                tracing::info!("执行 DEB 包安装");
                let mut child = tokio::process::Command::new("sudo")
                    .arg("apt")
                    .arg("install")
                    .arg("-y")
                    .arg(update_path)
                    .spawn()
                    .context("Failed to start DEB installer")?;

                let status = child.wait().await.context("DEB installer failed")?;
                if !status.success() {
                    return Err(anyhow!(
                        "DEB installer failed with exit code: {:?}",
                        status.code()
                    ));
                }
            } else if file_name.ends_with(".rpm") {
                tracing::info!("执行 RPM 包安装");
                let mut child = tokio::process::Command::new("sudo")
                    .arg("dnf")
                    .arg("install")
                    .arg("-y")
                    .arg(update_path)
                    .spawn()
                    .context("Failed to start RPM installer")?;

                let status = child.wait().await.context("RPM installer failed")?;
                if !status.success() {
                    return Err(anyhow!(
                        "RPM installer failed with exit code: {:?}",
                        status.code()
                    ));
                }
            } else {
                // 其他格式，尝试用系统默认程序打开
                tracing::info!(update_path = %update_path, "尝试用系统默认程序打开");
                tokio::process::Command::new("xdg-open")
                    .arg(update_path)
                    .spawn()
                    .context("Failed to open file with default application")?;
            }
        }

        tracing::info!("安装过程完成");
        Ok(())
    }

    async fn restore_from_backup(&self, _backup_dir: &std::path::Path) -> Result<()> {
        // 从备份恢复的具体实现
        Ok(())
    }

    #[cfg(target_os = "macos")]
    async fn install_dmg_package(&self, dmg_path: &str) -> Result<()> {
        use tokio::process::Command;

        let mount_point = self.update_dir.join("mount");
        if mount_point.exists() {
            let _ = fs::remove_dir_all(&mount_point).await;
        }
        fs::create_dir_all(&mount_point)
            .await
            .context("Failed to create DMG mount directory")?;

        let status = Command::new("hdiutil")
            .arg("attach")
            .arg(dmg_path)
            .arg("-mountpoint")
            .arg(&mount_point)
            .arg("-nobrowse")
            .arg("-quiet")
            .status()
            .await
            .context("Failed to execute hdiutil attach")?;

        if !status.success() {
            return Err(anyhow!(
                "DMG mounting failed with exit code: {:?}",
                status.code()
            ));
        }

        let install_result = async {
            let app_bundle = self
                .find_app_bundle_in_mount(&mount_point)
                .await
                .context("Failed to locate .app bundle inside DMG")?;
            let target_bundle =
                Self::resolve_current_app_bundle().context("Failed to resolve app bundle path")?;
            self.copy_app_bundle(&app_bundle, &target_bundle).await
        }
        .await;

        let _ = Command::new("hdiutil")
            .arg("detach")
            .arg(&mount_point)
            .arg("-quiet")
            .status()
            .await;

        let _ = fs::remove_dir_all(&mount_point).await;

        install_result
    }

    #[cfg(target_os = "macos")]
    async fn find_app_bundle_in_mount(&self, mount_point: &Path) -> Result<PathBuf> {
        let mut entries = fs::read_dir(mount_point)
            .await
            .context("Failed to read DMG contents")?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .context("Failed to enumerate DMG entries")?
        {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("app") {
                return Ok(path);
            }
        }

        Err(anyhow!("No .app bundle found inside mounted DMG"))
    }

    #[cfg(target_os = "macos")]
    async fn copy_app_bundle(&self, source: &Path, target: &Path) -> Result<()> {
        use tokio::process::Command;

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .await
                .context("Failed to create target application directory")?;
        }

        tracing::info!(target = ?target, source = ?source, "复制新的应用程序包");
        let status = Command::new("ditto")
            .arg(source)
            .arg(target)
            .status()
            .await
            .context("Failed to execute ditto command")?;

        if !status.success() {
            return Err(anyhow!("ditto failed with exit code: {:?}", status.code()));
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn resolve_current_app_bundle() -> Result<PathBuf> {
        let exe_path =
            std::env::current_exe().context("Unable to determine current executable path")?;
        let bundle_path = exe_path
            .ancestors()
            .nth(3)
            .ok_or_else(|| anyhow!("Failed to resolve current .app bundle path"))?;
        Ok(bundle_path.to_path_buf())
    }
}

impl Default for UpdateService {
    fn default() -> Self {
        Self::new()
    }
}
