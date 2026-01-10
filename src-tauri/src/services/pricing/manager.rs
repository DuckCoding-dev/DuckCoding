use crate::data::DataManager;
use crate::models::pricing::{DefaultTemplatesConfig, InheritedModel, ModelPrice, PricingTemplate};
use crate::services::pricing::builtin::builtin_claude_official_template;
use crate::utils::precision::price_precision;
use anyhow::{anyhow, Context, Result};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

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
        let builtin_template = builtin_claude_official_template();
        self.save_template(&builtin_template)?;

        // 初始化默认模板配置（如果不存在）
        if !self.default_templates_path.exists() {
            let mut config = DefaultTemplatesConfig::new();
            config.set_default(
                "claude-code".to_string(),
                "claude_official_2025_01".to_string(),
            );
            config.set_default("codex".to_string(), "claude_official_2025_01".to_string());
            config.set_default(
                "gemini-cli".to_string(),
                "claude_official_2025_01".to_string(),
            );

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

    /// 计算成本（核心方法）
    ///
    /// # 参数
    ///
    /// - `template_id`: 价格模板 ID（None 时使用工具默认模板）
    /// - `model`: 模型名称
    /// - `input_tokens`: 输入 Token 数量
    /// - `output_tokens`: 输出 Token 数量
    /// - `cache_creation_tokens`: 缓存创建 Token 数量
    /// - `cache_read_tokens`: 缓存读取 Token 数量
    ///
    /// # 返回
    ///
    /// 成本分解结果
    pub fn calculate_cost(
        &self,
        template_id: Option<&str>,
        model: &str,
        input_tokens: i64,
        output_tokens: i64,
        cache_creation_tokens: i64,
        cache_read_tokens: i64,
    ) -> Result<CostBreakdown> {
        // 1. 获取模板
        let template = if let Some(id) = template_id {
            self.get_template(id)?
        } else {
            // 使用 claude-code 的默认模板作为回退
            self.get_default_template("claude-code")?
        };

        // 2. 解析模型价格（别名 → 继承 → 倍率）
        let model_price = self.resolve_model_price(&template, model)?;

        // 3. 计算各部分价格
        let input_price = input_tokens as f64 * model_price.input_price_per_1m / 1_000_000.0;
        let output_price = output_tokens as f64 * model_price.output_price_per_1m / 1_000_000.0;
        let cache_write_price = cache_creation_tokens as f64
            * model_price.cache_write_price_per_1m.unwrap_or(0.0)
            / 1_000_000.0;
        let cache_read_price = cache_read_tokens as f64
            * model_price.cache_read_price_per_1m.unwrap_or(0.0)
            / 1_000_000.0;

        // 4. 计算总成本
        let total_cost = input_price + output_price + cache_write_price + cache_read_price;

        Ok(CostBreakdown {
            input_price,
            output_price,
            cache_write_price,
            cache_read_price,
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

        // 3. 查找继承配置（每个模型独立配置）
        for inherited in &template.inherited_models {
            if inherited.model_name == model {
                return self.resolve_inherited_price(inherited);
            }
        }

        Err(anyhow!(
            "Model {} not found in template {}",
            model,
            template.id
        ))
    }

    /// 递归解析继承价格
    fn resolve_inherited_price(&self, inherited: &InheritedModel) -> Result<ModelPrice> {
        // 1. 加载源模板
        let source_template = self.get_template(&inherited.source_template_id)?;

        // 2. 递归解析源模板中的价格
        let base_price = self.resolve_model_price(&source_template, &inherited.model_name)?;

        // 3. 应用倍率
        Ok(ModelPrice {
            provider: base_price.provider,
            input_price_per_1m: base_price.input_price_per_1m * inherited.multiplier,
            output_price_per_1m: base_price.output_price_per_1m * inherited.multiplier,
            cache_write_price_per_1m: base_price
                .cache_write_price_per_1m
                .map(|p| p * inherited.multiplier),
            cache_read_price_per_1m: base_price
                .cache_read_price_per_1m
                .map(|p| p * inherited.multiplier),
            currency: base_price.currency,
            aliases: base_price.aliases,
        })
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
        let template = manager.get_template("claude_official_2025_01").unwrap();
        assert_eq!(template.id, "claude_official_2025_01");
        assert!(template.is_default_preset);
    }

    #[test]
    fn test_resolve_model_price_with_alias() {
        let (manager, _dir) = create_test_manager();
        let template = manager.get_template("claude_official_2025_01").unwrap();

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
                Some("claude_official_2025_01"),
                "claude-sonnet-4.5",
                1000, // input
                500,  // output
                100,  // cache write
                200,  // cache read
            )
            .unwrap();

        // 验证各部分价格
        // input: 1000 * 3.0 / 1_000_000 = 0.003
        assert_eq!(breakdown.input_price, 0.003);

        // output: 500 * 15.0 / 1_000_000 = 0.0075
        assert_eq!(breakdown.output_price, 0.0075);

        // cache write: 100 * 3.75 / 1_000_000 = 0.000375
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

        assert_eq!(breakdown.template_id, "claude_official_2025_01");
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
                    "claude_official_2025_01".to_string(),
                    1.1,
                ),
                InheritedModel::new(
                    "claude-opus-4.5".to_string(),
                    "claude_official_2025_01".to_string(),
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
            .calculate_cost(None, "claude-sonnet-4.5", 1000, 500, 0, 0)
            .unwrap();

        assert_eq!(breakdown.template_id, "claude_official_2025_01");
        assert_eq!(breakdown.input_price, 0.003);
        assert_eq!(breakdown.output_price, 0.0075);
    }

    #[test]
    fn test_set_and_get_default_template() {
        let (manager, _dir) = create_test_manager();

        // 设置默认模板
        manager
            .set_default_template("test-tool", "claude_official_2025_01")
            .unwrap();

        // 获取默认模板
        let template = manager.get_default_template("test-tool").unwrap();
        assert_eq!(template.id, "claude_official_2025_01");
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
        let result = manager.delete_template("claude_official_2025_01");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot delete built-in preset template"));
    }
}
