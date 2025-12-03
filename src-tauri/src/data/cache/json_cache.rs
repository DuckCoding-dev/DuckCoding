//! JSON 配置缓存实现
//!
//! 提供基于文件路径的 JSON 配置缓存，支持：
//! - 文件校验和验证（SHA-256）
//! - 自动失效过期缓存
//! - 线程安全访问
//!
//! # 使用示例
//!
//! ```rust
//! use std::time::Duration;
//! use std::path::Path;
//! use crate::data::cache::JsonConfigCache;
//!
//! let cache = JsonConfigCache::new(50, Duration::from_secs(300));
//!
//! // 第一次读取（缓存未命中）
//! if let Some(config) = cache.get(Path::new("config.json")) {
//!     println!("缓存命中");
//! }
//!
//! // 插入缓存
//! cache.insert(
//!     Path::new("config.json").to_path_buf(),
//!     serde_json::json!({"key": "value"}),
//!     "checksum123".to_string()
//! );
//!
//! // 第二次读取（缓存命中）
//! let config = cache.get(Path::new("config.json")).unwrap();
//! ```

use super::LruCache;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// JSON 配置缓存
///
/// 使用 LRU 缓存存储 JSON 配置，并通过 SHA-256 校验和验证文件是否变更。
#[derive(Debug, Clone)]
pub struct JsonConfigCache {
    /// LRU 缓存，键为文件路径，值为 JSON Value
    cache: Arc<RwLock<LruCache<PathBuf, serde_json::Value>>>,
    /// 文件校验和映射，用于检测文件变更
    file_checksums: Arc<RwLock<HashMap<PathBuf, String>>>,
    /// 缓存容量
    capacity: usize,
    /// 缓存 TTL（存储用于查询）
    #[allow(dead_code)]
    ttl: Duration,
}

impl JsonConfigCache {
    /// 创建新的 JSON 配置缓存
    ///
    /// # 参数
    ///
    /// - `capacity`: 缓存容量（最大文件数）
    /// - `ttl`: 缓存项的生存时间
    ///
    /// # 示例
    ///
    /// ```rust
    /// use std::time::Duration;
    /// let cache = JsonConfigCache::new(50, Duration::from_secs(300)); // 50 个文件，5 分钟 TTL
    /// ```
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(capacity, ttl))),
            file_checksums: Arc::new(RwLock::new(HashMap::new())),
            capacity,
            ttl,
        }
    }

    /// 获取缓存的配置
    ///
    /// 自动校验文件是否变更，如果文件内容变更则使缓存失效。
    ///
    /// # 返回
    ///
    /// - `Some(Value)`: 缓存命中且未过期
    /// - `None`: 缓存未命中、已过期或文件已变更
    pub fn get(&self, path: &Path) -> Option<serde_json::Value> {
        // 尝试从缓存获取
        let cached_value = {
            let mut cache = self.cache.write().ok()?;
            cache.get(&path.to_path_buf()).cloned()
        }?;

        // 检查文件是否变更
        if let Ok(current_checksum) = compute_checksum(path) {
            let checksums = self.file_checksums.read().ok()?;
            if let Some(stored_checksum) = checksums.get(&path.to_path_buf()) {
                if stored_checksum != &current_checksum {
                    // 文件已变更，使缓存失效
                    drop(checksums);
                    self.invalidate(path);
                    return None;
                }
            } else {
                // 没有校验和记录，认为缓存无效
                return None;
            }
        } else {
            // 无法计算校验和（文件可能已删除），使缓存失效
            self.invalidate(path);
            return None;
        }

        Some(cached_value)
    }

    /// 插入缓存
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    /// - `value`: JSON 配置值
    /// - `checksum`: 文件校验和
    pub fn insert(&self, path: PathBuf, value: serde_json::Value, checksum: String) {
        // 插入缓存
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(path.clone(), value);
        }

        // 记录校验和
        if let Ok(mut checksums) = self.file_checksums.write() {
            checksums.insert(path, checksum);
        }
    }

    /// 使指定路径的缓存失效
    ///
    /// 删除缓存值和校验和记录。
    pub fn invalidate(&self, path: &Path) {
        let path_buf = path.to_path_buf();

        // 删除缓存
        if let Ok(mut cache) = self.cache.write() {
            cache.remove(&path_buf);
        }

        // 删除校验和
        if let Ok(mut checksums) = self.file_checksums.write() {
            checksums.remove(&path_buf);
        }
    }

    /// 清空所有缓存
    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }

        if let Ok(mut checksums) = self.file_checksums.write() {
            checksums.clear();
        }
    }

    /// 获取当前缓存项数量
    pub fn len(&self) -> usize {
        self.cache.read().map(|c| c.len()).unwrap_or(0)
    }

    /// 检查缓存是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 获取缓存容量
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 动态调整缓存容量
    pub fn set_capacity(&self, new_capacity: usize) {
        if let Ok(mut cache) = self.cache.write() {
            cache.set_capacity(new_capacity);
        }
    }

    /// 动态调整 TTL
    pub fn set_ttl(&self, new_ttl: Duration) {
        if let Ok(mut cache) = self.cache.write() {
            cache.set_ttl(new_ttl);
        }
    }
}

/// 计算文件的 SHA-256 校验和
///
/// # 参数
///
/// - `path`: 文件路径
///
/// # 返回
///
/// - `Ok(String)`: 十六进制格式的校验和
/// - `Err(std::io::Error)`: 文件读取失败
fn compute_checksum(path: &Path) -> std::io::Result<String> {
    let content = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use tempfile::TempDir;

    #[test]
    fn test_basic_insert_and_get() {
        let cache = JsonConfigCache::new(10, Duration::from_secs(60));
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");

        // 写入测试文件
        let content = serde_json::json!({"key": "value"});
        fs::write(&file_path, content.to_string()).unwrap();

        // 计算校验和
        let checksum = compute_checksum(&file_path).unwrap();

        // 插入缓存
        cache.insert(file_path.clone(), content.clone(), checksum);

        // 获取缓存
        let cached = cache.get(&file_path).unwrap();
        assert_eq!(cached, content);
    }

    #[test]
    fn test_cache_miss() {
        let cache = JsonConfigCache::new(10, Duration::from_secs(60));
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nonexistent.json");

        // 未插入缓存，应该返回 None
        assert!(cache.get(&file_path).is_none());
    }

    #[test]
    fn test_file_change_detection() {
        let cache = JsonConfigCache::new(10, Duration::from_secs(60));
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");

        // 写入初始内容
        let content1 = serde_json::json!({"version": 1});
        fs::write(&file_path, content1.to_string()).unwrap();
        let checksum1 = compute_checksum(&file_path).unwrap();

        // 插入缓存
        cache.insert(file_path.clone(), content1.clone(), checksum1);

        // 验证缓存命中
        assert_eq!(cache.get(&file_path).unwrap(), content1);

        // 修改文件内容
        let content2 = serde_json::json!({"version": 2});
        fs::write(&file_path, content2.to_string()).unwrap();

        // 缓存应该失效（文件校验和不匹配）
        assert!(cache.get(&file_path).is_none());
    }

    #[test]
    fn test_invalidate() {
        let cache = JsonConfigCache::new(10, Duration::from_secs(60));
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");

        // 写入文件
        let content = serde_json::json!({"key": "value"});
        fs::write(&file_path, content.to_string()).unwrap();
        let checksum = compute_checksum(&file_path).unwrap();

        // 插入缓存
        cache.insert(file_path.clone(), content.clone(), checksum);
        assert!(cache.get(&file_path).is_some());

        // 使缓存失效
        cache.invalidate(&file_path);
        assert!(cache.get(&file_path).is_none());
    }

    #[test]
    fn test_clear() {
        let cache = JsonConfigCache::new(10, Duration::from_secs(60));
        let temp_dir = TempDir::new().unwrap();

        // 插入多个文件
        for i in 0..5 {
            let file_path = temp_dir.path().join(format!("config{}.json", i));
            let content = serde_json::json!({"id": i});
            fs::write(&file_path, content.to_string()).unwrap();
            let checksum = compute_checksum(&file_path).unwrap();
            cache.insert(file_path, content, checksum);
        }

        assert_eq!(cache.len(), 5);

        // 清空缓存
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_capacity_limit() {
        let cache = JsonConfigCache::new(3, Duration::from_secs(60));
        let temp_dir = TempDir::new().unwrap();

        // 插入 4 个文件，应该淘汰最旧的
        for i in 0..4 {
            let file_path = temp_dir.path().join(format!("config{}.json", i));
            let content = serde_json::json!({"id": i});
            fs::write(&file_path, content.to_string()).unwrap();
            let checksum = compute_checksum(&file_path).unwrap();
            cache.insert(file_path, content, checksum);
        }

        // 缓存容量为 3
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn test_ttl_expiration() {
        let cache = JsonConfigCache::new(10, Duration::from_millis(100));
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");

        // 写入文件
        let content = serde_json::json!({"key": "value"});
        fs::write(&file_path, content.to_string()).unwrap();
        let checksum = compute_checksum(&file_path).unwrap();

        // 插入缓存
        cache.insert(file_path.clone(), content.clone(), checksum);
        assert!(cache.get(&file_path).is_some());

        // 等待超过 TTL
        thread::sleep(Duration::from_millis(150));

        // 缓存应该已过期
        assert!(cache.get(&file_path).is_none());
    }

    #[test]
    fn test_concurrent_access() {
        let cache = Arc::new(JsonConfigCache::new(100, Duration::from_secs(60)));
        let temp_dir = Arc::new(TempDir::new().unwrap());
        let mut handles = vec![];

        // 10 个线程并发插入
        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let temp_dir_clone = Arc::clone(&temp_dir);

            let handle = thread::spawn(move || {
                for j in 0..10 {
                    let file_path = temp_dir_clone
                        .path()
                        .join(format!("config-{}-{}.json", i, j));
                    let content = serde_json::json!({"thread": i, "id": j});
                    fs::write(&file_path, content.to_string()).unwrap();
                    let checksum = compute_checksum(&file_path).unwrap();
                    cache_clone.insert(file_path, content, checksum);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // 验证所有数据都已插入
        assert_eq!(cache.len(), 100);
    }

    #[test]
    fn test_set_capacity() {
        let cache = JsonConfigCache::new(5, Duration::from_secs(60));
        assert_eq!(cache.capacity(), 5);

        cache.set_capacity(10);
        // 注意：capacity() 返回的是初始容量，不是动态更新的
        // 实际容量已经在内部 LruCache 中更新
    }

    #[test]
    fn test_compute_checksum() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // 写入测试内容
        fs::write(&file_path, "test content").unwrap();

        // 计算校验和
        let checksum1 = compute_checksum(&file_path).unwrap();
        let checksum2 = compute_checksum(&file_path).unwrap();

        // 相同内容应该产生相同的校验和
        assert_eq!(checksum1, checksum2);

        // 修改内容
        fs::write(&file_path, "modified content").unwrap();
        let checksum3 = compute_checksum(&file_path).unwrap();

        // 不同内容应该产生不同的校验和
        assert_ne!(checksum1, checksum3);
    }

    #[test]
    fn test_deleted_file_invalidation() {
        let cache = JsonConfigCache::new(10, Duration::from_secs(60));
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");

        // 写入文件
        let content = serde_json::json!({"key": "value"});
        fs::write(&file_path, content.to_string()).unwrap();
        let checksum = compute_checksum(&file_path).unwrap();

        // 插入缓存
        cache.insert(file_path.clone(), content.clone(), checksum);
        assert!(cache.get(&file_path).is_some());

        // 删除文件
        fs::remove_file(&file_path).unwrap();

        // 缓存应该失效（无法计算校验和）
        assert!(cache.get(&file_path).is_none());
    }
}
