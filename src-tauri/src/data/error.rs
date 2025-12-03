//! 统一错误类型定义
//!
//! 使用 `thiserror` 定义数据管理模块的所有错误类型，并提供与 `anyhow` 的兼容层。

use std::path::PathBuf;
use thiserror::Error;

/// 数据管理模块的统一错误类型
#[derive(Error, Debug)]
pub enum DataError {
    /// 文件 I/O 错误
    #[error("文件 I/O 错误: {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// JSON 序列化/反序列化错误
    #[error("JSON 序列化错误: {0}")]
    JsonSerialization(#[from] serde_json::Error),

    /// TOML 反序列化错误
    #[error("TOML 反序列化错误: {0}")]
    TomlDeserialization(#[from] toml::de::Error),

    /// TOML 编辑错误（toml_edit）
    #[error("TOML 编辑错误: {0}")]
    TomlEdit(String),

    /// 数据库错误
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),

    /// 资源未找到
    #[error("未找到资源: {0}")]
    NotFound(String),

    /// 权限错误
    #[error("权限错误: {0}")]
    Permission(String),

    /// 缓存校验失败
    #[error("缓存校验失败: {0}")]
    CacheValidation(String),

    /// 并发错误
    #[error("并发错误: {0}")]
    Concurrency(String),

    /// 无效的键路径
    #[error("无效的键路径: {0}")]
    InvalidKey(String),
}

/// 便于与现有代码集成的类型别名
pub type Result<T> = std::result::Result<T, DataError>;

// 注意：DataError 已通过 thiserror 实现了 std::error::Error trait，
// anyhow 会自动提供 From<DataError> for anyhow::Error 的实现，
// 因此无需手动实现，避免冲突。

/// 便捷的 I/O 错误构造器
impl DataError {
    /// 从 `std::io::Error` 和路径创建 I/O 错误
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = DataError::NotFound("config.json".to_string());
        assert_eq!(err.to_string(), "未找到资源: config.json");
    }

    #[test]
    fn test_io_error_construction() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = DataError::io("/path/to/file", io_err);
        assert!(err.to_string().contains("/path/to/file"));
    }

    #[test]
    fn test_json_error_conversion() {
        let json_err = serde_json::from_str::<serde_json::Value>("{invalid json").unwrap_err();
        let err: DataError = json_err.into();
        assert!(matches!(err, DataError::JsonSerialization(_)));
    }

    #[test]
    fn test_anyhow_conversion() {
        let err = DataError::NotFound("test".to_string());
        // DataError 实现了 std::error::Error，可自动转换为 anyhow::Error
        let anyhow_err: anyhow::Error = err.into();
        assert!(anyhow_err.to_string().contains("未找到资源"));
        assert!(anyhow_err.to_string().contains("test"));
    }

    #[test]
    fn test_cache_validation_error() {
        let err = DataError::CacheValidation("checksum mismatch".to_string());
        assert_eq!(err.to_string(), "缓存校验失败: checksum mismatch");
    }

    #[test]
    fn test_invalid_key_error() {
        let err = DataError::InvalidKey("".to_string());
        assert!(err.to_string().contains("无效的键路径"));
    }
}
