use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 单个模型的价格定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPrice {
    /// 提供商（如：anthropic、openai）
    pub provider: String,

    /// 输入价格（USD/百万 Token）
    pub input_price_per_1m: f64,

    /// 输出价格（USD/百万 Token）
    pub output_price_per_1m: f64,

    /// 缓存写入价格（USD/百万 Token，可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write_price_per_1m: Option<f64>,

    /// 缓存读取价格（USD/百万 Token，可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_price_per_1m: Option<f64>,

    /// 推理输出价格（USD/百万 Token，可选，如 OpenAI o1 系列）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_output_price_per_1m: Option<f64>,

    /// 货币类型（默认：USD）
    #[serde(default = "default_currency")]
    pub currency: String,

    /// 模型别名列表（支持多种 ID 格式）
    #[serde(default)]
    pub aliases: Vec<String>,
}

impl ModelPrice {
    /// 创建新的模型价格定义
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        provider: String,
        input_price_per_1m: f64,
        output_price_per_1m: f64,
        cache_write_price_per_1m: Option<f64>,
        cache_read_price_per_1m: Option<f64>,
        reasoning_output_price_per_1m: Option<f64>,
        aliases: Vec<String>,
    ) -> Self {
        Self {
            provider,
            input_price_per_1m,
            output_price_per_1m,
            cache_write_price_per_1m,
            cache_read_price_per_1m,
            reasoning_output_price_per_1m,
            currency: default_currency(),
            aliases,
        }
    }
}

/// 单个模型的继承配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InheritedModel {
    /// 模型名称（如："claude-sonnet-4.5"）
    pub model_name: String,

    /// 从哪个模板继承（如："claude_official_2025_01"）
    pub source_template_id: String,

    /// 倍率（应用到继承的价格上）
    pub multiplier: f64,
}

impl InheritedModel {
    /// 创建新的继承模型配置
    pub fn new(model_name: String, source_template_id: String, multiplier: f64) -> Self {
        Self {
            model_name,
            source_template_id,
            multiplier,
        }
    }
}

/// 价格模板（统一结构，支持三种模式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingTemplate {
    /// 模板ID（唯一标识）
    pub id: String,

    /// 模板名称
    pub name: String,

    /// 模板描述
    pub description: String,

    /// 模板版本
    pub version: String,

    /// 创建时间（Unix 时间戳，毫秒）
    pub created_at: i64,

    /// 更新时间（Unix 时间戳，毫秒）
    pub updated_at: i64,

    /// 继承配置（每个模型独立配置，可从不同模板继承）
    #[serde(default)]
    pub inherited_models: Vec<InheritedModel>,

    /// 自定义模型（直接定义价格）
    #[serde(default)]
    pub custom_models: HashMap<String, ModelPrice>,

    /// 标签列表（用于分类和搜索）
    #[serde(default)]
    pub tags: Vec<String>,

    /// 是否为内置预设模板
    #[serde(default)]
    pub is_default_preset: bool,
}

impl PricingTemplate {
    /// 创建新的价格模板
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        name: String,
        description: String,
        version: String,
        inherited_models: Vec<InheritedModel>,
        custom_models: HashMap<String, ModelPrice>,
        tags: Vec<String>,
        is_default_preset: bool,
    ) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id,
            name,
            description,
            version,
            created_at: now,
            updated_at: now,
            inherited_models,
            custom_models,
            tags,
            is_default_preset,
        }
    }

    /// 判断是否为完全自定义模式
    ///
    /// 完全自定义：inherited_models 为空，custom_models 包含所有模型及其价格
    pub fn is_full_custom(&self) -> bool {
        self.inherited_models.is_empty() && !self.custom_models.is_empty()
    }

    /// 判断是否为纯继承模式
    ///
    /// 纯继承：inherited_models 包含多个模型，custom_models 为空
    pub fn is_pure_inheritance(&self) -> bool {
        !self.inherited_models.is_empty() && self.custom_models.is_empty()
    }

    /// 判断是否为混合模式
    ///
    /// 混合模式：inherited_models 和 custom_models 同时存在
    pub fn is_mixed(&self) -> bool {
        !self.inherited_models.is_empty() && !self.custom_models.is_empty()
    }
}

/// 工具默认模板配置（存储在 default_templates.json）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultTemplatesConfig {
    /// 工具 -> 默认模板 ID 的映射
    ///
    /// 例如：
    /// ```json
    /// {
    ///   "claude-code": "claude_official_2025_01",
    ///   "codex": "claude_official_2025_01",
    ///   "gemini-cli": "claude_official_2025_01"
    /// }
    /// ```
    #[serde(flatten)]
    pub tool_defaults: HashMap<String, String>,
}

impl DefaultTemplatesConfig {
    /// 创建新的默认模板配置
    pub fn new() -> Self {
        Self {
            tool_defaults: HashMap::new(),
        }
    }

    /// 获取工具的默认模板 ID
    pub fn get_default(&self, tool_id: &str) -> Option<&String> {
        self.tool_defaults.get(tool_id)
    }

    /// 设置工具的默认模板 ID
    pub fn set_default(&mut self, tool_id: String, template_id: String) {
        self.tool_defaults.insert(tool_id, template_id);
    }
}

impl Default for DefaultTemplatesConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// 默认货币类型
fn default_currency() -> String {
    "USD".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_price_creation() {
        let price = ModelPrice::new(
            "anthropic".to_string(),
            3.0,
            15.0,
            Some(3.75),
            Some(0.3),
            None,  // No reasoning price
            vec![
                "claude-sonnet-4.5".to_string(),
                "claude-sonnet-4-5".to_string(),
            ],
        );

        assert_eq!(price.provider, "anthropic");
        assert_eq!(price.input_price_per_1m, 3.0);
        assert_eq!(price.output_price_per_1m, 15.0);
        assert_eq!(price.cache_write_price_per_1m, Some(3.75));
        assert_eq!(price.cache_read_price_per_1m, Some(0.3));
        assert_eq!(price.currency, "USD");
        assert_eq!(price.aliases.len(), 2);
    }

    #[test]
    fn test_inherited_model_creation() {
        let inherited = InheritedModel::new(
            "claude-sonnet-4.5".to_string(),
            "builtin_claude".to_string(),
            1.1,
        );

        assert_eq!(inherited.model_name, "claude-sonnet-4.5");
        assert_eq!(inherited.source_template_id, "builtin_claude");
        assert_eq!(inherited.multiplier, 1.1);
    }

    #[test]
    fn test_pricing_template_modes() {
        // 完全自定义模式
        let mut custom_models = HashMap::new();
        custom_models.insert(
            "model1".to_string(),
            ModelPrice::new("provider1".to_string(), 1.0, 2.0, None, None, None, vec![]),
        );

        let full_custom = PricingTemplate::new(
            "template1".to_string(),
            "Full Custom".to_string(),
            "Description".to_string(),
            "1.0".to_string(),
            vec![],
            custom_models.clone(),
            vec![],
            false,
        );

        assert!(full_custom.is_full_custom());
        assert!(!full_custom.is_pure_inheritance());
        assert!(!full_custom.is_mixed());

        // 纯继承模式
        let inherited_models = vec![InheritedModel::new(
            "model1".to_string(),
            "source_template".to_string(),
            1.0,
        )];

        let pure_inheritance = PricingTemplate::new(
            "template2".to_string(),
            "Pure Inheritance".to_string(),
            "Description".to_string(),
            "1.0".to_string(),
            inherited_models.clone(),
            HashMap::new(),
            vec![],
            false,
        );

        assert!(!pure_inheritance.is_full_custom());
        assert!(pure_inheritance.is_pure_inheritance());
        assert!(!pure_inheritance.is_mixed());

        // 混合模式
        let mixed = PricingTemplate::new(
            "template3".to_string(),
            "Mixed".to_string(),
            "Description".to_string(),
            "1.0".to_string(),
            inherited_models,
            custom_models,
            vec![],
            false,
        );

        assert!(!mixed.is_full_custom());
        assert!(!mixed.is_pure_inheritance());
        assert!(mixed.is_mixed());
    }

    #[test]
    fn test_default_templates_config() {
        let mut config = DefaultTemplatesConfig::new();

        config.set_default("claude-code".to_string(), "template1".to_string());
        config.set_default("codex".to_string(), "template2".to_string());

        assert_eq!(
            config.get_default("claude-code"),
            Some(&"template1".to_string())
        );
        assert_eq!(config.get_default("codex"), Some(&"template2".to_string()));
        assert_eq!(config.get_default("gemini-cli"), None);
    }
}
