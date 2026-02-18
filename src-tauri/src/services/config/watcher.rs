// 配置文件监听模块 - 重写版本
//
// 功能：
// 1. 启动时自动保存所有工具的配置快照到 GlobalConfig
// 2. 监听配置文件变更（notify）
// 3. 检测变更并发送事件到前端
// 4. Block/Allow 操作在 commands 层实现

use crate::data::changelogs::ConfigChangeRecord;
use crate::models::config::{ConfigWatchConfig, WatchMode};
use crate::models::Tool;
use anyhow::{anyhow, Result};
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

// ========== 导出类型 ==========

/// 变更类型
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeType {
    Modified,
    Added,
    Deleted,
}

/// 单个字段的变更
#[derive(Debug, Clone, Serialize)]
pub struct FieldChange {
    /// 字段路径
    pub path: String,
    /// 旧值（删除时为 Some，新增时为 None）
    pub old_value: Option<JsonValue>,
    /// 新值（新增时为 Some，删除时为 None）
    pub new_value: Option<JsonValue>,
    /// 变更类型
    pub change_type: ChangeType,
}

/// 外部配置变更事件
#[derive(Debug, Clone, Serialize)]
pub struct ExternalConfigChange {
    /// 工具 ID
    pub tool_id: String,
    /// 配置文件路径
    pub path: String,
    /// 变更字段列表
    pub changed_fields: Vec<FieldChange>,
    /// 是否包含敏感字段
    pub is_sensitive: bool,
}

// ========== 快照管理 ==========

/// 启动时初始化所有工具的配置快照
pub fn initialize_snapshots() -> Result<()> {
    tracing::info!("初始化配置快照...");

    let tools = vec![Tool::claude_code(), Tool::codex(), Tool::gemini_cli()];

    for tool in tools {
        if let Err(e) = save_snapshot_for_tool(&tool) {
            tracing::warn!("保存 {} 配置快照失败: {}", tool.id, e);
        } else {
            tracing::debug!("已保存 {} 配置快照", tool.id);
        }
    }

    Ok(())
}

/// 为单个工具保存配置快照
pub fn save_snapshot_for_tool(tool: &Tool) -> Result<()> {
    use crate::data::DataManager;
    use std::collections::HashMap;

    let manager = DataManager::new();
    let mut files = HashMap::new();

    // 读取所有配置文件
    for filename in tool.config_files() {
        let config_path = tool.config_dir.join(&filename);
        if !config_path.exists() {
            tracing::debug!("配置文件不存在，跳过: {}", config_path.display());
            continue;
        }

        let content = if filename.ends_with(".json") {
            // JSON 文件：直接读取
            manager.json_uncached().read(&config_path)?
        } else if filename.ends_with(".toml") {
            // TOML 文件：读取 DocumentMut 并转换为 JSON
            let doc = manager.toml().read_document(&config_path)?;
            toml_to_json(&doc)?
        } else if filename.ends_with(".env") || filename == ".env" {
            // ENV 文件：读取并转换为 JSON 对象
            let env_map = manager.env().read(&config_path)?;
            serde_json::to_value(env_map)?
        } else {
            tracing::warn!("不支持的配置文件格式: {}", filename);
            continue;
        };

        files.insert(filename.clone(), content);
    }

    if files.is_empty() {
        tracing::warn!("工具 {} 没有可用的配置文件", tool.id);
        return Ok(());
    }

    // 保存到独立快照文件
    crate::data::snapshots::save_snapshot_files(&tool.id, files)?;

    Ok(())
}

/// 将 TOML DocumentMut 转换为 JSON
fn toml_to_json(doc: &toml_edit::DocumentMut) -> Result<serde_json::Value> {
    let toml_str = doc.to_string();
    let toml_value: toml::Value =
        toml::from_str(&toml_str).map_err(|e| anyhow!("TOML 解析失败: {}", e))?;
    Ok(serde_json::to_value(toml_value)?)
}

// ========== 差异分析 ==========

/// 计算两个配置对象的差异
///
/// # Arguments
///
/// * `old` - 旧配置
/// * `new` - 新配置
/// * `prefix` - 当前路径前缀
///
/// # Returns
///
/// 返回变更字段列表（包含变更前后值）
fn compute_diff(old: &JsonValue, new: &JsonValue, prefix: &str) -> Vec<FieldChange> {
    let mut changes = Vec::new();

    match (old, new) {
        (JsonValue::Object(old_map), JsonValue::Object(new_map)) => {
            // 检查删除的字段
            for (key, old_val) in old_map.iter() {
                if !new_map.contains_key(key) {
                    let path = if prefix.is_empty() {
                        key.to_string()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    changes.push(FieldChange {
                        path,
                        old_value: Some(old_val.clone()),
                        new_value: None,
                        change_type: ChangeType::Deleted,
                    });
                }
            }

            // 检查新增和修改的字段
            for (key, new_val) in new_map.iter() {
                let child_prefix = if prefix.is_empty() {
                    key.to_string()
                } else {
                    format!("{}.{}", prefix, key)
                };

                if let Some(old_val) = old_map.get(key) {
                    if old_val != new_val {
                        // 递归比较子对象
                        let mut child_changes = compute_diff(old_val, new_val, &child_prefix);
                        changes.append(&mut child_changes);
                    }
                } else {
                    // 新增字段
                    changes.push(FieldChange {
                        path: child_prefix,
                        old_value: None,
                        new_value: Some(new_val.clone()),
                        change_type: ChangeType::Added,
                    });
                }
            }
        }
        (JsonValue::Array(old_arr), JsonValue::Array(new_arr)) => {
            if old_arr != new_arr {
                // 数组整体变更
                changes.push(FieldChange {
                    path: prefix.to_string(),
                    old_value: Some(old.clone()),
                    new_value: Some(new.clone()),
                    change_type: ChangeType::Modified,
                });
            }
        }
        _ => {
            // 基本类型或类型变更
            if old != new {
                changes.push(FieldChange {
                    path: prefix.to_string(),
                    old_value: Some(old.clone()),
                    new_value: Some(new.clone()),
                    change_type: ChangeType::Modified,
                });
            }
        }
    }

    changes
}

/// 过滤黑名单字段
fn filter_blacklist(fields: Vec<FieldChange>, blacklist: &[String]) -> Vec<FieldChange> {
    fields
        .into_iter()
        .filter(|field| {
            for pattern in blacklist {
                if pattern.ends_with(".*") {
                    let prefix = &pattern[..pattern.len() - 2];
                    if field.path.starts_with(prefix) {
                        return false;
                    }
                } else if &field.path == pattern || field.path.starts_with(&format!("{}.", pattern))
                {
                    return false;
                }
            }
            true
        })
        .collect()
}

/// 检查是否包含敏感字段
fn contains_sensitive(fields: &[FieldChange], sensitive: &[String]) -> bool {
    for field in fields {
        if contains_sensitive_field(&field.path, sensitive) {
            return true;
        }
    }
    false
}

// ========== 变更检测 ==========

/// 检测单个工具的配置变更
fn detect_tool_change(
    tool: &Tool,
    watch_config: &ConfigWatchConfig,
) -> Result<Option<ExternalConfigChange>> {
    use crate::data::DataManager;

    // 获取快照
    let snapshot = match crate::data::snapshots::get_snapshot(&tool.id)? {
        Some(s) => s,
        None => {
            // 首次检测：自动保存当前状态作为快照
            tracing::debug!("首次检测配置文件，自动保存快照: {}", tool.id);
            save_snapshot_for_tool(tool)?;
            return Ok(None);
        }
    };

    // 读取所有当前配置文件
    let manager = DataManager::new();
    let mut current_files = std::collections::HashMap::new();

    for filename in tool.config_files() {
        let config_path = tool.config_dir.join(&filename);
        if !config_path.exists() {
            continue;
        }

        let content = if filename.ends_with(".json") {
            manager.json_uncached().read(&config_path)?
        } else if filename.ends_with(".toml") {
            let doc = manager.toml().read_document(&config_path)?;
            toml_to_json(&doc)?
        } else if filename.ends_with(".env") || filename == ".env" {
            let env_map = manager.env().read(&config_path)?;
            serde_json::to_value(env_map)?
        } else {
            continue;
        };

        current_files.insert(filename.clone(), content);
    }

    // 比较所有文件的变更
    let mut all_changes = Vec::new();

    for (filename, new_content) in &current_files {
        let old_content = snapshot.files.get(filename);
        if let Some(old) = old_content {
            let file_changes = compute_diff(old, new_content, "");
            // 为每个变更字段添加文件前缀（如果不是主配置文件）
            for mut change in file_changes {
                if filename != &tool.config_file {
                    change.path = format!("{}:{}", filename, change.path);
                }
                all_changes.push(change);
            }
        } else {
            // 新增文件
            tracing::debug!("检测到新增配置文件: {}", filename);
        }
    }

    // 检测删除的文件
    for filename in snapshot.files.keys() {
        if !current_files.contains_key(filename) {
            tracing::debug!("检测到删除的配置文件: {}", filename);
        }
    }

    if all_changes.is_empty() {
        return Ok(None);
    }

    // 应用黑名单过滤
    let mut changed_fields = all_changes;
    if let Some(blacklist) = watch_config.blacklist.get(&tool.id) {
        changed_fields = filter_blacklist(changed_fields, blacklist);
    }

    // 根据监听模式过滤
    match watch_config.mode {
        WatchMode::Default => {
            // 默认模式：仅保留敏感字段变更
            if let Some(sensitive) = watch_config.sensitive_fields.get(&tool.id) {
                changed_fields.retain(|field| contains_sensitive_field(&field.path, sensitive));
            } else {
                // 没有敏感字段定义，清空变更列表
                changed_fields.clear();
            }
        }
        WatchMode::Full => {
            // 全量模式：保留所有非黑名单字段
        }
    }

    if changed_fields.is_empty() {
        return Ok(None);
    }

    // 检查是否包含敏感字段
    let is_sensitive = if let Some(sensitive) = watch_config.sensitive_fields.get(&tool.id) {
        contains_sensitive(&changed_fields, sensitive)
    } else {
        false
    };

    let config_path = tool.config_dir.join(&tool.config_file);
    Ok(Some(ExternalConfigChange {
        tool_id: tool.id.clone(),
        path: config_path.to_string_lossy().to_string(),
        changed_fields,
        is_sensitive,
    }))
}

/// 检查字段路径是否匹配敏感字段模式（支持文件前缀）
fn contains_sensitive_field(field_path: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        // 支持带文件前缀的模式（如 auth.json:OPENAI_API_KEY）
        let matches = if pattern.ends_with(".*") {
            let prefix = &pattern[..pattern.len() - 2];
            field_path.starts_with(prefix)
        } else if pattern.contains(':') {
            // 带文件前缀的精确匹配或子字段匹配
            field_path == pattern || field_path.starts_with(&format!("{}.", pattern))
        } else {
            // 不带文件前缀的模式：匹配主配置文件中的字段
            field_path == pattern || field_path.starts_with(&format!("{}.", pattern))
        };

        if matches {
            return true;
        }
    }
    false
}

// ========== 文件监听 ==========

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::Mutex;

/// 全局 Watcher 实例
static WATCHER_HANDLE: once_cell::sync::Lazy<Mutex<Option<WatcherHandle>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(None));

/// 内部配置写入抑制窗口（tool_id -> 到期时间）
static INTERNAL_CHANGE_SUPPRESS_UNTIL: once_cell::sync::Lazy<Mutex<HashMap<String, Instant>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

/// Watcher 句柄
struct WatcherHandle {
    _watcher: RecommendedWatcher,
    stop_signal: Arc<AtomicBool>,
}

/// 标记某个工具发生内部配置写入，在短时间内跳过外部变更检测。
pub fn suppress_external_detection_for_tool(tool_id: &str, duration: Duration) {
    let expire_at = Instant::now() + duration;
    INTERNAL_CHANGE_SUPPRESS_UNTIL
        .lock()
        .unwrap()
        .insert(tool_id.to_string(), expire_at);

    tracing::debug!(
        tool_id = %tool_id,
        duration_secs = duration.as_secs_f32(),
        "标记内部配置写入，短时间跳过外部变更检测"
    );
}

/// 检查工具是否在内部写入抑制窗口内，并清理已过期项。
fn is_external_detection_suppressed(tool_id: &str) -> bool {
    let now = Instant::now();
    let mut suppressed = INTERNAL_CHANGE_SUPPRESS_UNTIL.lock().unwrap();
    suppressed.retain(|_, expire_at| *expire_at > now);

    suppressed
        .get(tool_id)
        .is_some_and(|expire_at| *expire_at > now)
}

/// 启动配置文件监听
pub fn start_watcher(app_handle: AppHandle) -> Result<()> {
    // 读取配置判断是否启用
    let global_config = crate::utils::config::read_global_config()
        .map_err(|e| anyhow!(e))?
        .ok_or_else(|| anyhow!("全局配置文件不存在"))?;

    if !global_config.config_watch.enabled {
        tracing::info!("配置守护已禁用，跳过启动 watcher");
        return Ok(());
    }

    let scan_interval = global_config.config_watch.scan_interval;
    tracing::info!("启动配置守护，扫描间隔: {}秒", scan_interval);

    // 停止旧的 watcher
    stop_watcher()?;

    let (tx, rx) = mpsc::channel();
    let running = Arc::new(AtomicBool::new(true));

    // 创建 notify watcher
    let tools = vec![Tool::claude_code(), Tool::codex(), Tool::gemini_cli()];

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {
                        if let Some(path) = event.paths.first() {
                            let _ = tx.send(path.clone());
                        }
                    }
                    _ => {}
                }
            }
        },
        notify::Config::default().with_poll_interval(Duration::from_secs(scan_interval)),
    )?;

    // 监听所有工具的配置目录
    for tool in &tools {
        if tool.config_dir.exists() {
            watcher.watch(&tool.config_dir, RecursiveMode::NonRecursive)?;
            tracing::debug!("开始监听配置目录: {}", tool.config_dir.display());
        }
    }

    // 后台线程处理变更
    let running_clone = running.clone();
    thread::spawn(move || {
        let mut last_check = std::collections::HashMap::new();

        while running_clone.load(Ordering::Relaxed) {
            if let Ok(path) = rx.recv_timeout(Duration::from_millis(500)) {
                // 防抖：同一路径 500ms 内只处理一次
                let now = std::time::Instant::now();
                if let Some(last) = last_check.get(&path) {
                    if now.duration_since(*last) < Duration::from_millis(500) {
                        continue;
                    }
                }
                last_check.insert(path.clone(), now);

                // 检测变更
                if let Err(e) = handle_file_change(&path, &app_handle) {
                    tracing::error!("处理配置变更失败: {}", e);
                }
            }
        }
    });

    // 保存句柄
    let handle = WatcherHandle {
        _watcher: watcher,
        stop_signal: running,
    };
    *WATCHER_HANDLE.lock().unwrap() = Some(handle);

    Ok(())
}

/// 停止配置文件监听
pub fn stop_watcher() -> Result<()> {
    let mut handle = WATCHER_HANDLE.lock().unwrap();
    if let Some(h) = handle.take() {
        h.stop_signal.store(false, Ordering::Relaxed);
        tracing::info!("配置守护已停止");
    }
    Ok(())
}

/// 处理单个文件变更
fn handle_file_change(path: &Path, app_handle: &AppHandle) -> Result<()> {
    // 读取全局配置
    let global_config = crate::utils::config::read_global_config()
        .map_err(|e| anyhow!(e))?
        .ok_or_else(|| anyhow!("全局配置文件不存在"))?;

    let watch_config = &global_config.config_watch;

    // 找到对应的工具
    let tools = vec![Tool::claude_code(), Tool::codex(), Tool::gemini_cli()];

    for tool in tools {
        // 检查是否是该工具的任一配置文件
        let is_tool_config = tool.config_files().iter().any(|filename| {
            let config_path = tool.config_dir.join(filename);
            config_path == path
        });

        if is_tool_config {
            if is_external_detection_suppressed(&tool.id) {
                tracing::debug!(tool_id = %tool.id, "检测到内部写入，跳过外部变更通知");
                if let Err(error) = save_snapshot_for_tool(&tool) {
                    tracing::warn!(
                        error = ?error,
                        tool_id = %tool.id,
                        "内部写入后刷新配置快照失败"
                    );
                }
                break;
            }

            // 检测变更
            if let Some(change) = detect_tool_change(&tool, watch_config)? {
                tracing::info!(
                    "检测到配置变更: {} ({} 个字段)",
                    change.tool_id,
                    change.changed_fields.len()
                );

                // 记录到变更日志
                use crate::data::changelogs::ConfigChangeRecord;
                use std::collections::HashMap;

                let mut before_values = HashMap::new();
                let mut after_values = HashMap::new();
                let changed_field_paths: Vec<String> = change
                    .changed_fields
                    .iter()
                    .map(|f| {
                        if let Some(old) = &f.old_value {
                            before_values.insert(f.path.clone(), old.clone());
                        }
                        if let Some(new) = &f.new_value {
                            after_values.insert(f.path.clone(), new.clone());
                        }
                        f.path.clone()
                    })
                    .collect();

                let record = ConfigChangeRecord {
                    tool_id: change.tool_id.clone(),
                    timestamp: chrono::Utc::now(),
                    changed_fields: changed_field_paths,
                    is_sensitive: change.is_sensitive,
                    before_values,
                    after_values,
                    action: None, // 用户尚未操作
                };

                if let Err(e) = save_change_record(record) {
                    tracing::error!("保存变更日志失败: {}", e);
                }

                // 发送事件到前端
                app_handle.emit("external-config-changed", change)?;
            }
            break;
        }
    }

    Ok(())
}

/// 保存变更记录到日志
fn save_change_record(record: ConfigChangeRecord) -> Result<()> {
    use crate::data::changelogs::ChangeLogStore;

    let mut store = ChangeLogStore::load()?;
    store.add_record(record);
    store.save()?;
    Ok(())
}
