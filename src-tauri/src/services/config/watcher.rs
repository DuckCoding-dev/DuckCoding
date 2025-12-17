//! 配置文件外部变更检测与监听模块
//!
//! 提供两种监听机制：
//! - `ConfigWatcher`: 基于轮询的文件监听（跨平台兼容）
//! - `NotifyWatcherManager`: 基于 OS 通知的实时监听（性能更优）

use super::types::{ExternalConfigChange, ImportExternalChangeResult};
use crate::models::Tool;
use crate::services::profile_manager::ProfileManager;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use notify::{
    Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;
use tauri::Emitter;
use tracing::{debug, warn};

/// 文件变更事件（用于监听器内部）
#[derive(Debug, Clone, Serialize)]
pub struct FileChangeEvent {
    pub tool_id: String,
    pub path: PathBuf,
    pub checksum: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub dirty: bool,
    pub fallback_poll: bool,
}

/// Tauri 事件名称（外部配置变更通知）
pub const EXTERNAL_CHANGE_EVENT: &str = "external-config-changed";

// ========== 核心函数：配置路径与校验和 ==========

/// 返回工具配置文件列表（包含主配置和附属文件）
pub(crate) fn config_paths(tool: &Tool) -> Vec<PathBuf> {
    let mut paths = vec![tool.config_dir.join(&tool.config_file)];
    match tool.id.as_str() {
        "codex" => {
            paths.push(tool.config_dir.join("auth.json"));
        }
        "gemini-cli" => {
            paths.push(tool.config_dir.join(".env"));
        }
        "claude-code" => {
            paths.push(tool.config_dir.join("config.json"));
        }
        _ => {}
    }
    paths
}

/// 计算配置文件组合哈希（SHA256）
///
/// 任一文件变动都会改变校验和，用于检测外部修改
pub(crate) fn compute_native_checksum(tool: &Tool) -> Option<String> {
    let mut paths = config_paths(tool);
    paths.sort();

    let mut hasher = Sha256::new();
    let mut any_exists = false;
    for path in paths {
        hasher.update(path.to_string_lossy().as_bytes());
        if path.exists() {
            any_exists = true;
            match fs::read(&path) {
                Ok(content) => hasher.update(&content),
                Err(_) => return None,
            }
        } else {
            hasher.update(b"MISSING");
        }
    }

    if any_exists {
        Some(format!("{:x}", hasher.finalize()))
    } else {
        None
    }
}

// ========== 外部变更检测与管理 ==========

/// 将外部修改导入为 Profile
///
/// # Arguments
///
/// * `tool` - 目标工具
/// * `profile_name` - Profile 名称
/// * `as_new` - 是否作为新 Profile（true 时如果已存在则报错）
///
/// # Errors
///
/// 当 Profile 名称为空、已存在（as_new=true）或导入失败时返回错误
pub fn import_external_change(
    tool: &Tool,
    profile_name: &str,
    as_new: bool,
) -> Result<ImportExternalChangeResult> {
    let target_profile = profile_name.trim();
    if target_profile.is_empty() {
        anyhow::bail!("profile 名称不能为空");
    }

    let profile_manager = ProfileManager::new()?;

    // 检查 Profile 是否存在
    let existing = profile_manager.list_profiles(&tool.id)?;
    let exists = existing.iter().any(|p| p == target_profile);
    if as_new && exists {
        anyhow::bail!("profile 已存在: {target_profile}");
    }

    let checksum_before = compute_native_checksum(tool);

    // 使用 ProfileManager 的 capture_from_native 方法
    profile_manager.capture_from_native(&tool.id, target_profile)?;

    let checksum = compute_native_checksum(tool);
    let replaced = !as_new && exists;

    Ok(ImportExternalChangeResult {
        profile_name: target_profile.to_string(),
        was_new: as_new,
        replaced,
        before_checksum: checksum_before,
        checksum,
    })
}

/// 扫描所有工具的原生配置，检测外部修改
///
/// # Returns
///
/// 返回变更列表，每项包含工具 ID、路径、校验和和脏标记
///
/// # Errors
///
/// 当 ProfileManager 初始化失败或状态访问失败时返回错误
pub fn detect_external_changes() -> Result<Vec<ExternalConfigChange>> {
    let mut changes = Vec::new();
    let profile_manager = ProfileManager::new()?;

    for tool in Tool::all() {
        // 只检测已经有 active_state 的工具（跳过从未使用过的工具）
        let active_opt = profile_manager.get_active_state(&tool.id)?;
        if active_opt.is_none() {
            continue;
        }

        let current_checksum = compute_native_checksum(&tool);
        let active = active_opt.ok_or_else(|| anyhow!("工具 {} 无激活 Profile", tool.id))?;
        let last_checksum = active.native_checksum.clone();

        if last_checksum.as_ref() != current_checksum.as_ref() {
            // 标记脏，但保留旧 checksum 以便前端确认后再更新
            profile_manager.mark_active_dirty(&tool.id, true)?;

            changes.push(ExternalConfigChange {
                tool_id: tool.id.clone(),
                path: tool
                    .config_dir
                    .join(&tool.config_file)
                    .to_string_lossy()
                    .to_string(),
                checksum: current_checksum.clone(),
                detected_at: Utc::now(),
                dirty: true,
            });
        } else if active.dirty {
            // 仍在脏状态时保持报告
            changes.push(ExternalConfigChange {
                tool_id: tool.id.clone(),
                path: tool
                    .config_dir
                    .join(&tool.config_file)
                    .to_string_lossy()
                    .to_string(),
                checksum: current_checksum.clone(),
                detected_at: Utc::now(),
                dirty: true,
            });
        }
    }
    Ok(changes)
}

/// 标记外部修改（用于事件监听场景）
///
/// # Arguments
///
/// * `tool` - 目标工具
/// * `path` - 发生变更的文件路径
/// * `checksum` - 新的校验和
///
/// # Returns
///
/// 返回变更事件，包含脏标记（仅当校验和变化时为 true）
pub fn mark_external_change(
    tool: &Tool,
    path: PathBuf,
    checksum: Option<String>,
) -> Result<ExternalConfigChange> {
    let profile_manager = ProfileManager::new()?;
    let active_opt = profile_manager.get_active_state(&tool.id)?;

    let last_checksum = active_opt.as_ref().and_then(|a| a.native_checksum.clone());

    // 若与当前记录的 checksum 一致，则视为内部写入，保持非脏状态
    let checksum_changed = last_checksum.as_ref() != checksum.as_ref();

    // 更新 checksum 和 dirty 状态
    profile_manager.update_active_sync_state(&tool.id, checksum.clone(), checksum_changed)?;

    Ok(ExternalConfigChange {
        tool_id: tool.id.clone(),
        path: path.to_string_lossy().to_string(),
        checksum,
        detected_at: Utc::now(),
        dirty: checksum_changed,
    })
}

/// 确认/清除外部修改状态，刷新校验和
///
/// # Arguments
///
/// * `tool` - 目标工具
///
/// # Errors
///
/// 当 ProfileManager 操作失败时返回错误
pub fn acknowledge_external_change(tool: &Tool) -> Result<()> {
    let current_checksum = compute_native_checksum(tool);

    let profile_manager = ProfileManager::new()?;
    profile_manager.update_active_sync_state(&tool.id, current_checksum, false)?;

    Ok(())
}

// ========== 文件监听器：轮询模式 ==========

/// 基于轮询的配置文件监听器
///
/// 通过定期检查文件校验和来检测变更，兼容性好但资源占用较高
pub struct ConfigWatcher {
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl ConfigWatcher {
    /// 轮询监听单个文件变更
    ///
    /// # Arguments
    ///
    /// * `tool_id` - 工具 ID
    /// * `path` - 文件路径
    /// * `poll_interval` - 轮询间隔
    /// * `mark_dirty` - 是否标记为脏状态
    ///
    /// # Returns
    ///
    /// 返回监听器实例和事件接收器
    pub fn watch_file_polling(
        tool_id: impl Into<String>,
        path: PathBuf,
        poll_interval: Duration,
        mark_dirty: bool,
    ) -> Result<(Self, mpsc::Receiver<FileChangeEvent>)> {
        use crate::utils::file_helpers::file_checksum;

        let tool_id = tool_id.into();
        let mut last_checksum = file_checksum(&path).ok();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_token = stop.clone();
        let (tx, rx) = mpsc::channel();
        let watch_path = path.clone();

        let handle = thread::spawn(move || {
            while !stop_token.load(Ordering::Relaxed) {
                let checksum = file_checksum(&watch_path).ok();
                if checksum.is_some() && checksum != last_checksum {
                    // 轻微防抖，避免写入过程中的空文件/瞬时内容导致重复事件
                    thread::sleep(Duration::from_millis(10));
                    let stable_checksum = file_checksum(&watch_path).ok().or(checksum.clone());

                    if stable_checksum.is_some() && stable_checksum != last_checksum {
                        last_checksum = stable_checksum.clone();
                        let change = FileChangeEvent {
                            tool_id: tool_id.clone(),
                            path: watch_path.clone(),
                            checksum: stable_checksum,
                            timestamp: Utc::now(),
                            dirty: mark_dirty,
                            fallback_poll: true,
                        };
                        let _ = tx.send(change);
                    }
                }
                thread::sleep(poll_interval);
            }
        });

        Ok((
            Self {
                stop,
                handle: Some(handle),
            },
            rx,
        ))
    }
}

impl Drop for ConfigWatcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

// ========== 文件监听器：OS 通知模式 ==========

/// 基于 notify 的实时配置文件监听管理器
///
/// 使用操作系统级文件通知，性能优异但依赖平台支持
pub struct NotifyWatcherManager {
    _watchers: Vec<RecommendedWatcher>,
}

impl NotifyWatcherManager {
    /// 监听单个配置文件
    fn watch_single(
        tool: Tool,
        path: PathBuf,
        app: tauri::AppHandle,
    ) -> Result<RecommendedWatcher> {
        let path_for_cb = path.clone();
        let tool_for_state = tool.clone();
        let mut last_checksum = compute_native_checksum(&tool_for_state);
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    match event.kind {
                        EventKind::Modify(_) | EventKind::Create(_) => {
                            let checksum = compute_native_checksum(&tool_for_state);
                            // 去重：相同 checksum 不重复触发
                            if checksum == last_checksum {
                                return;
                            }
                            last_checksum = checksum.clone();

                            match mark_external_change(
                                &tool_for_state,
                                path_for_cb.clone(),
                                checksum,
                            ) {
                                Ok(change) => {
                                    // 仅在确实变脏时通知前端，避免内部写入误报
                                    if change.dirty {
                                        debug!(
                                            tool = %change.tool_id,
                                            path = %change.path,
                                            checksum = ?change.checksum,
                                            "检测到配置文件改动（notify watcher）"
                                        );
                                        let _ = app.emit(EXTERNAL_CHANGE_EVENT, change);
                                    }
                                }
                                Err(err) => {
                                    warn!(
                                        tool = %tool_for_state.id,
                                        path = ?path_for_cb,
                                        error = ?err,
                                        "标记外部变更失败"
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }
            },
            NotifyConfig::default(),
        )?;

        watcher.watch(&path, RecursiveMode::NonRecursive)?;
        Ok(watcher)
    }

    /// 为所有已存在的配置文件启动监听器
    ///
    /// # Arguments
    ///
    /// * `app` - Tauri AppHandle，用于发送事件到前端
    ///
    /// # Returns
    ///
    /// 返回管理器实例，持有所有监听器
    pub fn start_all(app: tauri::AppHandle) -> Result<Self> {
        let mut watchers = Vec::new();
        for tool in Tool::all() {
            let mut seen = HashSet::new();
            for path in config_paths(&tool) {
                if !seen.insert(path.clone()) {
                    continue;
                }
                if !path.exists() {
                    warn!(
                        tool = %tool.id,
                        path = ?path,
                        "配置文件不存在，跳过通知 watcher（将依赖轮询/手动刷新）"
                    );
                    continue;
                }
                let watcher = Self::watch_single(tool.clone(), path, app.clone())?;
                watchers.push(watcher);
            }
        }
        debug!(count = watchers.len(), "通知 watcher 启动完成");
        Ok(Self {
            _watchers: watchers,
        })
    }
}

// ========== 测试 ==========

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::SystemTime;

    #[test]
    fn watcher_emits_on_change_and_filters_duplicate_checksum() -> Result<()> {
        let dir = std::env::temp_dir().join(format!(
            "duckcoding_watch_test_{}",
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
        fs::create_dir_all(&dir)?;
        let path = dir.join("settings.json");
        fs::write(&path, r#"{"env":{"KEY":"A"}}"#)?;

        let (_watcher, rx) = ConfigWatcher::watch_file_polling(
            "claude-code",
            path.clone(),
            Duration::from_millis(50),
            true,
        )?;

        // 改变内容，期望收到事件
        fs::write(&path, r#"{"env":{"KEY":"B"}}"#)?;
        let change = rx
            .recv_timeout(Duration::from_secs(3))
            .expect("should receive change event");
        assert_eq!(change.tool_id, "claude-code");
        assert_eq!(change.path, path);
        assert!(change.checksum.is_some());

        // 再写入相同内容，不应再次触发
        fs::write(&path, r#"{"env":{"KEY":"B"}}"#)?;
        assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());

        let _ = fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn watcher_respects_mark_dirty_flag() -> Result<()> {
        let dir = std::env::temp_dir().join(format!(
            "duckcoding_watch_test_mark_dirty_{}",
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
        fs::create_dir_all(&dir)?;
        let path = dir.join("settings.json");
        fs::write(&path, r#"{"env":{"KEY":"X"}}"#)?;

        // mark_dirty = false，应当仍能收到事件，但 dirty 为 false
        let (_watcher, rx) = ConfigWatcher::watch_file_polling(
            "codex",
            path.clone(),
            Duration::from_millis(30),
            false,
        )?;

        fs::write(&path, r#"{"env":{"KEY":"Y"}}"#)?;
        let change = rx
            .recv_timeout(Duration::from_secs(3))
            .expect("should receive change event");

        assert_eq!(change.tool_id, "codex");
        assert_eq!(change.path, path);
        assert!(change.checksum.is_some());
        assert!(!change.dirty, "dirty flag should respect mark_dirty=false");
        assert!(
            change.fallback_poll,
            "polling watcher should mark fallback_poll"
        );

        let _ = fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    #[ignore = "需要使用 ProfileManager API 重写"]
    fn mark_external_change_clears_dirty_when_checksum_unchanged() -> Result<()> {
        // TODO: 需要使用 ProfileManager API 重写此测试
        unimplemented!("需要使用 ProfileManager API 重写此测试")
    }

    #[test]
    #[ignore = "需要使用 ProfileManager API 重写"]
    fn mark_external_change_preserves_last_synced_at() -> Result<()> {
        // TODO: 需要使用 ProfileManager API 重写此测试
        unimplemented!("需要使用 ProfileManager API 重写此测试")
    }

    #[test]
    #[ignore = "需要使用 ProfileManager API 重写"]
    fn detect_and_ack_external_change_updates_state() -> Result<()> {
        // TODO: 需要使用 ProfileManager API 重写此测试
        unimplemented!("需要使用 ProfileManager API 重写此测试")
    }

    #[test]
    #[ignore = "需要使用 ProfileManager API 重写"]
    fn delete_profile_marks_active_dirty_when_matching() -> Result<()> {
        // TODO: 需要使用 ProfileManager API 重写此测试
        unimplemented!("需要使用 ProfileManager API 重写此测试")
    }
}
