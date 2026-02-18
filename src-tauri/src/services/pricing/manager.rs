use crate::data::DataManager;
use crate::models::pricing::{DefaultTemplatesConfig, ModelPrice, PricingTemplate};
use crate::services::pricing::builtin::{
    builtin_claude_official_template, builtin_gemini_official_template,
    builtin_openai_official_template,
};
use crate::services::pricing::remote_sync::RemoteSyncState;
use crate::utils::precision::price_precision;
use anyhow::{anyhow, Context, Result};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(test)]
use crate::models::pricing::InheritedModel;

/// 成本分解结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBreakdown {
    /// 输入部分价格（USD）
    #[serde(with = "price_precision")]
    pub input_price: f64,

    /// 输出部分价格（USD）
    #[serde(with = "price_precision")]
    pub output_price: f64,

    /// 缓存写入部分价格（USD）
    #[serde(with = "price_precision")]
    pub cache_write_price: f64,

    /// 缓存读取部分价格（USD）
    #[serde(with = "price_precision")]
    pub cache_read_price: f64,

    /// 推理输出部分价格（USD）
    #[serde(with = "price_precision")]
    pub reasoning_price: f64,

    /// 总成本（USD）
    #[serde(with = "price_precision")]
    pub total_cost: f64,

    /// 使用的价格模板 ID
    pub template_id: String,
}

lazy_static! {
    /// 全局 PricingManager 实例
    pub static ref PRICING_MANAGER: PricingManager = {
        PricingManager::init_global().expect("Failed to initialize PricingManager")
    };
}

/// 价格管理服务
pub struct PricingManager {
    /// DataManager 实例（Arc 包装以支持克隆）
    data_manager: Arc<DataManager>,

    /// 价格配置目录路径（保留用于未来扩展）
    #[allow(dead_code)]
    pricing_dir: PathBuf,

    /// 模板存储目录路径
    templates_dir: PathBuf,

    /// 默认模板配置文件路径
    default_templates_path: PathBuf,
}

impl PricingManager {
    /// 初始化全局实例（用于 lazy_static）
    pub fn init_global() -> Result<Self> {
        let home_dir = dirs::home_dir().ok_or_else(|| anyhow!("无法获取用户主目录"))?;
        let base_dir = home_dir.join(".duckcoding");

        let manager = Self::new(base_dir)?;
        manager.initialize()?;

        Ok(manager)
    }

    /// 创建新的价格管理服务实例（用于测试或自定义场景）
    pub fn new_with_manager(base_dir: PathBuf, data_manager: Arc<DataManager>) -> Self {
        let pricing_dir = base_dir.join("pricing");
        let templates_dir = pricing_dir.join("templates");
        let default_templates_path = pricing_dir.join("default_templates.json");

        Self {
            data_manager,
            pricing_dir,
            templates_dir,
            default_templates_path,
        }
    }

    /// 创建新的价格管理服务实例（使用全局 DataManager）
    pub fn new(base_dir: PathBuf) -> Result<Self> {
        Ok(Self::new_with_manager(
            base_dir,
            Arc::new(DataManager::new()),
        ))
    }

    /// 初始化价格配置目录和默认模板
    pub fn initialize(&self) -> Result<()> {
        // 创建目录
        std::fs::create_dir_all(&self.templates_dir)
            .context("Failed to create templates directory")?;

        // 保存内置 Claude 官方模板
        let builtin_claude_template = builtin_claude_official_template();
        self.save_template(&builtin_claude_template)?;

        // 保存内置 OpenAI 官方模板
        let builtin_openai_template = builtin_openai_official_template();
        self.save_template(&builtin_openai_template)?;

        // 保存内置 Gemini 官方模板
        let builtin_gemini_template = builtin_gemini_official_template();
        self.save_template(&builtin_gemini_template)?;

        // 初始化默认模板配置（如果不存在）
        if !self.default_templates_path.exists() {
            let mut config = DefaultTemplatesConfig::new();
            config.set_default("claude-code".to_string(), "builtin_claude".to_string());
            config.set_default("codex".to_string(), "builtin_openai".to_string());
            config.set_default("gemini-cli".to_string(), "builtin_gemini".to_string());

            let value = serde_json::to_value(&config)
                .context("Failed to serialize default templates config")?;

            self.data_manager
                .json()
                .write(&self.default_templates_path, &value)
                .context("Failed to write default templates config")?;
        }

        Ok(())
    }

    /// 获取指定的价格模板
    pub fn get_template(&self, template_id: &str) -> Result<PricingTemplate> {
        let template_path = self.templates_dir.join(format!("{}.json", template_id));

        if !template_path.exists() {
            return Err(anyhow!("Template {} not found", template_id));
        }

        let value = self
            .data_manager
            .json()
            .read(&template_path)
            .with_context(|| format!("Failed to read template {}", template_id))?;

        serde_json::from_value(value)
            .with_context(|| format!("Failed to parse template {}", template_id))
    }

    /// 列出所有价格模板
    pub fn list_templates(&self) -> Result<Vec<PricingTemplate>> {
        let mut templates = Vec::new();

        if !self.templates_dir.exists() {
            return Ok(templates);
        }

        let entries =
            std::fs::read_dir(&self.templates_dir).context("Failed to read templates directory")?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(value) = self.data_manager.json().read(&path) {
                    if let Ok(template) = serde_json::from_value::<PricingTemplate>(value) {
                        templates.push(template);
                    }
                }
            }
        }

        Ok(templates)
    }

    /// 保存价格模板
    pub fn save_template(&self, template: &PricingTemplate) -> Result<()> {
        let template_path = self.templates_dir.join(format!("{}.json", template.id));

        let value = serde_json::to_value(template)
            .with_context(|| format!("Failed to serialize template {}", template.id))?;

        self.data_manager
            .json()
            .write(&template_path, &value)
            .with_context(|| format!("Failed to save template {}", template.id))
    }

    /// 删除价格模板
    pub fn delete_template(&self, template_id: &str) -> Result<()> {
        let template_path = self.templates_dir.join(format!("{}.json", template_id));

        if !template_path.exists() {
            return Err(anyhow!("Template {} not found", template_id));
        }

        // 不允许删除内置预设模板
        if let Ok(template) = self.get_template(template_id) {
            if template.is_default_preset {
                return Err(anyhow!("Cannot delete built-in preset template"));
            }
        }

        std::fs::remove_file(&template_path)
            .with_context(|| format!("Failed to delete template {}", template_id))
    }

    /// 设置工具的默认模板
    pub fn set_default_template(&self, tool_id: &str, template_id: &str) -> Result<()> {
        // 验证模板是否存在
        self.get_template(template_id)?;

        let mut config = self.get_default_templates_config()?;
        config.set_default(tool_id.to_string(), template_id.to_string());

        let value = serde_json::to_value(&config)
            .context("Failed to serialize default templates config")?;

        self.data_manager
            .json()
            .write(&self.default_templates_path, &value)
            .context("Failed to update default templates config")
    }

    /// 获取工具的默认模板
    pub fn get_default_template(&self, tool_id: &str) -> Result<PricingTemplate> {
        let config = self.get_default_templates_config()?;

        let template_id = config
            .get_default(tool_id)
            .ok_or_else(|| anyhow!("No default template set for tool {}", tool_id))?;

        self.get_template(template_id)
    }

    /// 获取默认模板配置
    fn get_default_templates_config(&self) -> Result<DefaultTemplatesConfig> {
        if !self.default_templates_path.exists() {
            return Ok(DefaultTemplatesConfig::new());
        }

        let value = self
            .data_manager
            .json()
            .read(&self.default_templates_path)
            .context("Failed to read default templates config")?;

        serde_json::from_value(value).context("Failed to parse default templates config")
    }

    /// 加载远程同步状态
    pub fn load_sync_state(&self) -> Result<RemoteSyncState> {
        let state_path = self.pricing_dir.join("remote_sync_state.json");
        if !state_path.exists() {
            return Ok(RemoteSyncState::default());
        }

        let value = self
            .data_manager
            .json()
            .read(&state_path)
            .context("Failed to read remote sync state")?;

        serde_json::from_value(value).context("Failed to parse remote sync state")
    }

    /// 保存远程同步状态
    pub fn save_sync_state(&self, state: &RemoteSyncState) -> Result<()> {
        let state_path = self.pricing_dir.join("remote_sync_state.json");

        let value = serde_json::to_value(state).context("Failed to serialize remote sync state")?;

        self.data_manager
            .json()
            .write(&state_path, &value)
            .context("Failed to write remote sync state")
    }

    /// 计算成本（核心方法）
    ///
    /// # 参数
    ///
    /// - `template_id`: 价格模板 ID（None 时使用工具默认模板）
    /// - `tool_id`: 工具 ID（用于获取默认模板，当 template_id 为 None 时必须提供）
    /// - `model`: 模型名称
    /// - `input_tokens`: 输入 Token 数量
    /// - `output_tokens`: 输出 Token 数量
    /// - `cache_creation_tokens`: 缓存创建 Token 总量（5m + 1h）
    /// - `cache_creation_1h_tokens`: 1小时缓存创建 Token 数量（5m = total - 1h）
    /// - `cache_read_tokens`: 缓存读取 Token 数量
    /// - `reasoning_tokens`: 推理 Token 数量
    ///
    /// # 返回
    ///
    /// 成本分解结果
    #[allow(clippy::too_many_arguments)]
    pub fn calculate_cost(
        &self,
        template_id: Option<&str>,
        tool_id: Option<&str>,
        model: &str,
        input_tokens: i64,
        output_tokens: i64,
        cache_creation_tokens: i64,
        cache_creation_1h_tokens: i64,
        cache_read_tokens: i64,
        reasoning_tokens: i64,
    ) -> Result<CostBreakdown> {
        // 1. 获取模板
        let template = if let Some(id) = template_id {
            self.get_template(id)?
        } else {
            // 使用工具的默认模板（回退到 claude-code）
            let default_tool_id = tool_id.unwrap_or("claude-code");
            self.get_default_template(default_tool_id)?
        };

        // 2. 解析模型价格（别名 → 继承 → 倍率）
        let model_price = self.resolve_model_price(&template, model)?;

        // 3. 计算各部分价格
        let input_price = input_tokens as f64 * model_price.input_price_per_1m / 1_000_000.0;
        let output_price = output_tokens as f64 * model_price.output_price_per_1m / 1_000_000.0;

        // 缓存写入分别计价：5m 和 1h 使用不同价格
        let cache_5m_tokens = cache_creation_tokens - cache_creation_1h_tokens;
        let cache_write_5m_price = cache_5m_tokens as f64
            * model_price.cache_write_price_per_1m.unwrap_or(0.0)
            / 1_000_000.0;
        let cache_write_1h_price = cache_creation_1h_tokens as f64
            * model_price
                .cache_write_1h_price_per_1m
                .or(model_price.cache_write_price_per_1m) // 无 1h 价格时回退到 5m
                .unwrap_or(0.0)
            / 1_000_000.0;
        let cache_write_price = cache_write_5m_price + cache_write_1h_price;

        let cache_read_price = cache_read_tokens as f64
            * model_price.cache_read_price_per_1m.unwrap_or(0.0)
            / 1_000_000.0;

        // 计算推理 Token 价格（如果有专用价格则使用，否则使用普通输出价格）
        let reasoning_price =
            if let Some(reasoning_price_per_1m) = model_price.reasoning_output_price_per_1m {
                reasoning_tokens as f64 * reasoning_price_per_1m / 1_000_000.0
            } else {
                // 回退：使用普通输出价格
                reasoning_tokens as f64 * model_price.output_price_per_1m / 1_000_000.0
            };

        // 4. 计算总成本
        let total_cost =
            input_price + output_price + cache_write_price + cache_read_price + reasoning_price;

        Ok(CostBreakdown {
            input_price,
            output_price,
            cache_write_price,
            cache_read_price,
            reasoning_price,
            total_cost,
            template_id: template.id.clone(),
        })
    }

    /// 解析模型价格（支持别名、继承、倍率）
    fn resolve_model_price(&self, template: &PricingTemplate, model: &str) -> Result<ModelPrice> {
        // 1. 优先查找自定义模型（直接匹配）
        if let Some(price) = template.custom_models.get(model) {
            return Ok(price.clone());
        }

        // 2. 别名匹配自定义模型
        for price in template.custom_models.values() {
            if price.aliases.contains(&model.to_string()) {
                return Ok(price.clone());
            }
        }

        // 3. 查找继承配置（支持别名匹配）
        for inherited in &template.inherited_models {
            // 加载源模板并获取基础价格（包括别名信息）
            if let Ok(source_template) = self.get_template(&inherited.source_template_id) {
                if let Ok(base_price) =
                    self.resolve_model_price(&source_template, &inherited.model_name)
                {
                    // 检查请求的模型名是否匹配模型名或别名
                    if inherited.model_name == model
                        || base_price.aliases.contains(&model.to_string())
                    {
                        // 应用倍率
                        return Ok(ModelPrice {
                            provider: base_price.provider,
                            input_price_per_1m: base_price.input_price_per_1m
                                * inherited.multiplier,
                            output_price_per_1m: base_price.output_price_per_1m
                                * inherited.multiplier,
                            cache_write_price_per_1m: base_price
                                .cache_write_price_per_1m
                                .map(|p| p * inherited.multiplier),
                            cache_write_1h_price_per_1m: base_price
                                .cache_write_1h_price_per_1m
                                .map(|p| p * inherited.multiplier),
                            cache_read_price_per_1m: base_price
                                .cache_read_price_per_1m
                                .map(|p| p * inherited.multiplier),
                            reasoning_output_price_per_1m: base_price
                                .reasoning_output_price_per_1m
                                .map(|p| p * inherited.multiplier),
                            currency: base_price.currency,
                            aliases: base_price.aliases,
                        });
                    }
                }
            }
        }

        Err(anyhow!(
            "Model {} not found in template {}",
            model,
            template.id
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, TempDir};

    fn create_test_manager() -> (PricingManager, TempDir) {
        let dir = tempdir().unwrap();
        let data_manager = Arc::new(DataManager::new());
        let manager = PricingManager::new_with_manager(dir.path().to_path_buf(), data_manager);
        manager.initialize().unwrap();
        (manager, dir)
    }

    #[test]
    fn test_initialize() {
        let (manager, _dir) = create_test_manager();

        // 验证目录创建
        assert!(manager.pricing_dir.exists());
        assert!(manager.templates_dir.exists());
        assert!(manager.default_templates_path.exists());

        // 验证内置模板存在
        let template = manager.get_template("builtin_claude").unwrap();
        assert_eq!(template.id, "builtin_claude");
        assert!(template.is_default_preset);
    }

    #[test]
    fn test_resolve_model_price_with_alias() {
        let (manager, _dir) = create_test_manager();
        let template = manager.get_template("builtin_claude").unwrap();

        // 测试直接匹配
        let price1 = manager
            .resolve_model_price(&template, "claude-sonnet-4.5")
            .unwrap();
        assert_eq!(price1.input_price_per_1m, 3.0);

        // 测试别名匹配
        let price2 = manager
            .resolve_model_price(&template, "claude-sonnet-4-5")
            .unwrap();
        assert_eq!(price2.input_price_per_1m, 3.0);

        let price3 = manager
            .resolve_model_price(&template, "claude-sonnet-4-5-20250929")
            .unwrap();
        assert_eq!(price3.input_price_per_1m, 3.0);
    }

    #[test]
    fn test_calculate_cost_breakdown() {
        let (manager, _dir) = create_test_manager();

        let breakdown = manager
            .calculate_cost(
                Some("builtin_claude"),
                None, // 工具 ID（已指定模板则可选）
                "claude-sonnet-4.5",
                1000, // input
                500,  // output
                100,  // cache write (total 5m + 1h)
                0,    // cache_creation_1h_tokens
                200,  // cache read
                0,    // reasoning_tokens
            )
            .unwrap();

        // 验证各部分价格
        // input: 1000 * 3.0 / 1_000_000 = 0.003
        assert_eq!(breakdown.input_price, 0.003);

        // output: 500 * 15.0 / 1_000_000 = 0.0075
        assert_eq!(breakdown.output_price, 0.0075);

        // cache write: 100 * 3.75 / 1_000_000 = 0.000375 (全部为 5m)
        assert_eq!(breakdown.cache_write_price, 0.000375);

        // cache read: 200 * 0.3 / 1_000_000 = 0.00006
        assert_eq!(breakdown.cache_read_price, 0.00006);

        // total: 0.003 + 0.0075 + 0.000375 + 0.00006 = 0.011235
        // 使用 assert_eq! 直接比较，因为各部分价格已验证精确
        let expected_total = breakdown.input_price
            + breakdown.output_price
            + breakdown.cache_write_price
            + breakdown.cache_read_price;
        assert_eq!(breakdown.total_cost, expected_total);

        assert_eq!(breakdown.template_id, "builtin_claude");
    }

    #[test]
    fn test_multi_source_inheritance() {
        let (manager, _dir) = create_test_manager();

        // 创建一个使用多源继承的模板
        let template = PricingTemplate::new(
            "test_multi_source".to_string(),
            "Test Multi Source".to_string(),
            "Test".to_string(),
            "1.0".to_string(),
            vec![
                InheritedModel::new(
                    "claude-sonnet-4.5".to_string(),
                    "builtin_claude".to_string(),
                    1.1,
                ),
                InheritedModel::new(
                    "claude-opus-4.5".to_string(),
                    "builtin_claude".to_string(),
                    1.5,
                ),
            ],
            Default::default(),
            vec![],
            false,
        );

        manager.save_template(&template).unwrap();

        // 测试 Sonnet 4.5 的继承（1.1 倍率）
        let price1 = manager
            .resolve_model_price(&template, "claude-sonnet-4.5")
            .unwrap();
        assert_eq!(price1.input_price_per_1m, 3.0 * 1.1);
        assert_eq!(price1.output_price_per_1m, 15.0 * 1.1);

        // 测试 Opus 4.5 的继承（1.5 倍率）
        let price2 = manager
            .resolve_model_price(&template, "claude-opus-4.5")
            .unwrap();
        assert_eq!(price2.input_price_per_1m, 5.0 * 1.5);
        assert_eq!(price2.output_price_per_1m, 25.0 * 1.5);
    }

    #[test]
    fn test_default_template_fallback() {
        let (manager, _dir) = create_test_manager();

        // 不指定模板 ID，应使用默认模板
        let breakdown = manager
            .calculate_cost(
                None,
                Some("claude-code"),
                "claude-sonnet-4.5",
                1000,
                500,
                0,
                0, // cache_creation_1h_tokens
                0,
                0,
            )
            .unwrap();

        assert_eq!(breakdown.template_id, "builtin_claude");
        assert_eq!(breakdown.input_price, 0.003);
        assert_eq!(breakdown.output_price, 0.0075);
    }

    #[test]
    fn test_set_and_get_default_template() {
        let (manager, _dir) = create_test_manager();

        // 设置默认模板
        manager
            .set_default_template("test-tool", "builtin_claude")
            .unwrap();

        // 获取默认模板
        let template = manager.get_default_template("test-tool").unwrap();
        assert_eq!(template.id, "builtin_claude");
    }

    #[test]
    fn test_delete_template() {
        let (manager, _dir) = create_test_manager();

        // 创建测试模板
        let template = PricingTemplate::new(
            "test_delete".to_string(),
            "Test Delete".to_string(),
            "Test".to_string(),
            "1.0".to_string(),
            vec![],
            Default::default(),
            vec![],
            false,
        );

        manager.save_template(&template).unwrap();
        assert!(manager.get_template("test_delete").is_ok());

        // 删除模板
        manager.delete_template("test_delete").unwrap();
        assert!(manager.get_template("test_delete").is_err());
    }

    #[test]
    fn test_cannot_delete_builtin_template() {
        let (manager, _dir) = create_test_manager();

        // 尝试删除内置模板应该失败
        let result = manager.delete_template("builtin_claude");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot delete built-in preset template"));
    }
}
