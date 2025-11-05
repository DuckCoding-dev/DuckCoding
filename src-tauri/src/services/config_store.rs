use std::fs;
use std::path::PathBuf;

use serde_json::{Map, Value};
use toml_edit::DocumentMut;

use crate::error::{AppError, AppResult};

use super::{backup_json, backup_toml};

pub struct JsonStore {
    path: PathBuf,
}

impl JsonStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn update<F>(&self, mutator: F) -> AppResult<Value>
    where
        F: FnOnce(&mut Value) -> AppResult<()>,
    {
        let mut doc = self.read()?;

        if self.path.exists() {
            backup_json(&self.path)?;
        }

        mutator(&mut doc)?;
        self.write(&doc)?;
        Ok(doc)
    }

    pub fn read(&self) -> AppResult<Value> {
        if !self.path.exists() {
            return Ok(Value::Object(Map::new()));
        }

        let content = fs::read_to_string(&self.path)?;
        let value = if content.trim().is_empty() {
            Value::Object(Map::new())
        } else {
            serde_json::from_str(&content)?
        };

        Ok(value)
    }

    pub fn write(&self, value: &Value) -> AppResult<()> {
        self.ensure_parent()?;
        let tmp_path = self.tmp_path();
        let content = serde_json::to_string_pretty(value)?;
        fs::write(&tmp_path, content)?;
        self.replace_with_tmp(tmp_path)
    }

    fn ensure_parent(&self) -> AppResult<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    fn tmp_path(&self) -> PathBuf {
        self.path.with_extension("tmp")
    }

    fn replace_with_tmp(&self, tmp_path: PathBuf) -> AppResult<()> {
        if self.path.exists() {
            fs::remove_file(&self.path)?;
        }
        fs::rename(tmp_path, &self.path)?;
        Ok(())
    }
}

pub struct TomlStore {
    path: PathBuf,
}

impl TomlStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn update<F>(&self, mutator: F) -> AppResult<DocumentMut>
    where
        F: FnOnce(&mut DocumentMut) -> AppResult<()>,
    {
        let mut doc = self.read()?;

        if self.path.exists() {
            backup_toml(&self.path)?;
        }

        mutator(&mut doc)?;
        self.write(&doc)?;
        Ok(doc)
    }

    pub fn read(&self) -> AppResult<DocumentMut> {
        if !self.path.exists() {
            return Ok(DocumentMut::new());
        }

        let content = fs::read_to_string(&self.path)?;
        if content.trim().is_empty() {
            Ok(DocumentMut::new())
        } else {
            content.parse::<DocumentMut>().map_err(AppError::from)
        }
    }

    pub fn write(&self, doc: &DocumentMut) -> AppResult<()> {
        self.ensure_parent()?;
        let tmp_path = self.tmp_path();
        fs::write(&tmp_path, doc.to_string())?;
        self.replace_with_tmp(tmp_path)
    }

    fn ensure_parent(&self) -> AppResult<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    fn tmp_path(&self) -> PathBuf {
        self.path.with_extension("tmp")
    }

    fn replace_with_tmp(&self, tmp_path: PathBuf) -> AppResult<()> {
        if self.path.exists() {
            fs::remove_file(&self.path)?;
        }
        fs::rename(tmp_path, &self.path)?;
        Ok(())
    }
}
