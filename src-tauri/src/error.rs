use std::fmt::Display;

use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON 解析错误: {0}")]
    Json(#[from] serde_json::Error),
    #[error("TOML 解析错误: {0}")]
    Toml(#[from] toml_edit::TomlError),
    #[error("HTTP 请求错误: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Tauri 错误: {0}")]
    Tauri(#[from] tauri::Error),
    #[error("命令执行失败: {0}")]
    Command(String),
    #[error("配置错误: {0}")]
    Config(String),
    #[error("{0}")]
    Other(String),
}

impl AppError {
    pub fn command<E: Display>(err: E) -> Self {
        Self::Command(err.to_string())
    }

    pub fn config<E: Display>(err: E) -> Self {
        Self::Config(err.to_string())
    }
}
