use crate::models::pricing::{ModelPrice, PricingTemplate};
use std::collections::HashMap;

/// 生成内置 OpenAI/Codex 价格模板
///
/// 包含 OpenAI/Codex 模型的定价
pub fn builtin_openai_official_template() -> PricingTemplate {
    let mut custom_models = HashMap::new();

    // GPT-5.2 Codex: $3 input / $12 output
    // 注意：OpenAI 的缓存机制不同，没有 cache_creation，只有 cached_tokens（读取）
    custom_models.insert(
        "gpt-5.2-codex".to_string(),
        ModelPrice::new(
            "openai".to_string(),
            3.0,
            12.0,
            None,      // OpenAI 不收费缓存创建
            Some(1.5), // Cache read: input * 0.5 (OpenAI 标准)
            None,      // 标准模型无推理 tokens
            vec![
                "gpt-5.2-codex".to_string(),
                "gpt-5.2".to_string(),
                "gpt-5-2-codex".to_string(),
            ],
        ),
    );

    PricingTemplate::new(
        "builtin_openai".to_string(),
        "内置OpenAI价格".to_string(),
        "OpenAI 官方定价，包含 GPT/Codex 模型".to_string(),
        "1.0".to_string(),
        vec![], // 内置模板不使用继承
        custom_models,
        vec![
            "official".to_string(),
            "openai".to_string(),
            "codex".to_string(),
        ],
        true, // 标记为内置预设模板
    )
}

/// 生成内置 Claude 价格模板
///
/// 包含 8 个 Claude 模型的官方定价
pub fn builtin_claude_official_template() -> PricingTemplate {
    let mut custom_models = HashMap::new();

    // Claude Opus 4.5: $5 input / $25 output
    custom_models.insert(
        "claude-opus-4.5".to_string(),
        ModelPrice::new(
            "anthropic".to_string(),
            5.0,
            25.0,
            Some(6.25), // Cache write: 5.0 * 1.25
            Some(0.5),  // Cache read: 5.0 * 0.1
            None,       // No reasoning tokens
            vec![
                "claude-opus-4.5".to_string(),
                "claude-opus-4-5".to_string(),
                "opus-4.5".to_string(),
                "claude-opus-4-5-20251101".to_string(),
            ],
        ),
    );

    // Claude Opus 4.1: $15 input / $75 output
    custom_models.insert(
        "claude-opus-4.1".to_string(),
        ModelPrice::new(
            "anthropic".to_string(),
            15.0,
            75.0,
            Some(18.75), // Cache write: 15.0 * 1.25
            Some(1.5),   // Cache read: 15.0 * 0.1
            None,        // No reasoning tokens
            vec![
                "claude-opus-4.1".to_string(),
                "claude-opus-4-1".to_string(),
                "claude-opus-4-1-20250805".to_string(),
            ],
        ),
    );

    // Claude Opus 4: $15 input / $75 output
    custom_models.insert(
        "claude-opus-4".to_string(),
        ModelPrice::new(
            "anthropic".to_string(),
            15.0,
            75.0,
            Some(18.75), // Cache write: 15.0 * 1.25
            Some(1.5),   // Cache read: 15.0 * 0.1
            None,        // No reasoning tokens
            vec![
                "claude-opus-4".to_string(),
                "claude-opus-4-20250514".to_string(),
            ],
        ),
    );

    // Claude Sonnet 4.5: $3 input / $15 output
    custom_models.insert(
        "claude-sonnet-4.5".to_string(),
        ModelPrice::new(
            "anthropic".to_string(),
            3.0,
            15.0,
            Some(3.75), // Cache write: 3.0 * 1.25
            Some(0.3),  // Cache read: 3.0 * 0.1
            None,       // No reasoning tokens
            vec![
                "claude-sonnet-4.5".to_string(),
                "claude-sonnet-4-5".to_string(),
                "claude-sonnet-4-5-20250929".to_string(),
            ],
        ),
    );

    // Claude Sonnet 4: $3 input / $15 output
    custom_models.insert(
        "claude-sonnet-4".to_string(),
        ModelPrice::new(
            "anthropic".to_string(),
            3.0,
            15.0,
            Some(3.75), // Cache write: 3.0 * 1.25
            Some(0.3),  // Cache read: 3.0 * 0.1
            None,       // No reasoning tokens
            vec![
                "claude-sonnet-4".to_string(),
                "claude-sonnet-4-20250514".to_string(),
            ],
        ),
    );

    // claude-3-7-sonnet : $3 input / $15 output
    custom_models.insert(
        "claude-3-7-sonnet".to_string(),
        ModelPrice::new(
            "anthropic".to_string(),
            3.0,
            15.0,
            Some(3.75), // Cache write: 3.0 * 1.25
            Some(0.3),  // Cache read: 3.0 * 0.1
            None,       // No reasoning tokens
            vec![
                "claude-3-7-sonnet".to_string(),
                "claude-3-7-sonnet-20250219".to_string(),
                "claude-3-sonnet-3-7".to_string(),
                "sonnet-3.7".to_string(),
            ],
        ),
    );

    // Claude Haiku 4.5: $1 input / $5 output
    custom_models.insert(
        "claude-haiku-4.5".to_string(),
        ModelPrice::new(
            "anthropic".to_string(),
            1.0,
            5.0,
            Some(1.25), // Cache write: 1.0 * 1.25
            Some(0.1),  // Cache read: 1.0 * 0.1
            None,       // No reasoning tokens
            vec![
                "claude-haiku-4.5".to_string(),
                "claude-haiku-4-5".to_string(),
                "claude-haiku-4-5-20251001".to_string(),
            ],
        ),
    );

    // Claude Haiku 3.5: $0.8 input / $4 output
    custom_models.insert(
        "claude-haiku-3.5".to_string(),
        ModelPrice::new(
            "anthropic".to_string(),
            0.8,
            4.0,
            Some(1.0),  // Cache write: 0.8 * 1.25
            Some(0.08), // Cache read: 0.8 * 0.1
            None,       // No reasoning tokens
            vec![
                "claude-haiku-3.5".to_string(),
                "claude-haiku-3-5".to_string(),
                "claude-3-5-haiku-20241022".to_string(),
            ],
        ),
    );

    PricingTemplate::new(
        "builtin_claude".to_string(),
        "内置Claude价格".to_string(),
        "Anthropic 官方定价，包含 8 个 Claude 模型".to_string(),
        "1.0".to_string(),
        vec![], // 内置模板不使用继承
        custom_models,
        vec!["official".to_string(), "claude".to_string()],
        true, // 标记为内置预设模板
    )
}

/// 生成内置 Gemini 价格模板
///
/// 包含 Google Gemini 模型的定价
pub fn builtin_gemini_official_template() -> PricingTemplate {
    let mut custom_models = HashMap::new();

    // Gemini 2.5 Pro: $1.25 input / $10 output (≤200k tokens)
    custom_models.insert(
        "gemini-2.5-pro".to_string(),
        ModelPrice::new(
            "google".to_string(),
            1.25,
            10.0,
            None,
            Some(0.3125), // Cache read: $0.3125/1M
            Some(10.0),   // Reasoning (thinking) tokens
            vec!["gemini-2.5-pro".to_string(), "gemini-2-5-pro".to_string()],
        ),
    );

    // Gemini 2.5 Flash: $0.15 input / $0.60 output (≤200k tokens)
    custom_models.insert(
        "gemini-2.5-flash".to_string(),
        ModelPrice::new(
            "google".to_string(),
            0.15,
            0.6,
            None,
            Some(0.0375), // Cache read
            Some(3.5),    // Thinking tokens
            vec![
                "gemini-2.5-flash".to_string(),
                "gemini-2-5-flash".to_string(),
            ],
        ),
    );

    // Gemini 2.0 Flash: $0.10 input / $0.40 output
    custom_models.insert(
        "gemini-2.0-flash".to_string(),
        ModelPrice::new(
            "google".to_string(),
            0.1,
            0.4,
            None,
            Some(0.025), // Cache read
            None,
            vec![
                "gemini-2.0-flash".to_string(),
                "gemini-2-0-flash".to_string(),
            ],
        ),
    );

    PricingTemplate::new(
        "builtin_gemini".to_string(),
        "内置Gemini价格".to_string(),
        "Google Gemini 官方定价，包含主流 Gemini 模型".to_string(),
        "1.0".to_string(),
        vec![], // 内置模板不使用继承
        custom_models,
        vec![
            "official".to_string(),
            "gemini".to_string(),
            "google".to_string(),
        ],
        true, // 标记为内置预设模板
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_template() {
        let template = builtin_claude_official_template();

        // 验证基本信息
        assert_eq!(template.id, "builtin_claude");
        assert!(template.is_default_preset);
        assert!(template.is_full_custom());

        // 验证包含 8 个模型
        assert_eq!(template.custom_models.len(), 8);

        // 验证 Opus 4.5 价格
        let opus_4_5 = template.custom_models.get("claude-opus-4.5").unwrap();
        assert_eq!(opus_4_5.provider, "anthropic");
        assert_eq!(opus_4_5.input_price_per_1m, 5.0);
        assert_eq!(opus_4_5.output_price_per_1m, 25.0);
        assert_eq!(opus_4_5.cache_write_price_per_1m, Some(6.25));
        assert_eq!(opus_4_5.cache_read_price_per_1m, Some(0.5));
        assert_eq!(opus_4_5.aliases.len(), 4);

        // 验证 Sonnet 4.5 价格
        let sonnet_4_5 = template.custom_models.get("claude-sonnet-4.5").unwrap();
        assert_eq!(sonnet_4_5.input_price_per_1m, 3.0);
        assert_eq!(sonnet_4_5.output_price_per_1m, 15.0);
        assert_eq!(sonnet_4_5.cache_write_price_per_1m, Some(3.75));
        assert_eq!(sonnet_4_5.cache_read_price_per_1m, Some(0.3));

        // // 验证 Claude 3.5 Sonnet (旧版本) 价格
        // let sonnet_3_5 = template.custom_models.get("claude-3-5-sonnet").unwrap();
        // assert_eq!(sonnet_3_5.input_price_per_1m, 3.0);
        // assert_eq!(sonnet_3_5.output_price_per_1m, 15.0);
        // assert_eq!(sonnet_3_5.cache_write_price_per_1m, Some(3.75));
        // assert_eq!(sonnet_3_5.cache_read_price_per_1m, Some(0.3));
        // assert!(sonnet_3_5
        //     .aliases
        //     .contains(&"claude-sonnet-4-5-20250929".to_string()));

        // 验证 Haiku 3.5 价格
        let haiku_3_5 = template.custom_models.get("claude-haiku-3.5").unwrap();
        assert_eq!(haiku_3_5.input_price_per_1m, 0.8);
        assert_eq!(haiku_3_5.output_price_per_1m, 4.0);
        assert_eq!(haiku_3_5.cache_write_price_per_1m, Some(1.0));
        assert_eq!(haiku_3_5.cache_read_price_per_1m, Some(0.08));
    }

    #[test]
    fn test_builtin_template_aliases() {
        let template = builtin_claude_official_template();

        // 验证 Sonnet 4.5 的别名
        let sonnet_4_5 = template.custom_models.get("claude-sonnet-4.5").unwrap();
        assert!(sonnet_4_5
            .aliases
            .contains(&"claude-sonnet-4.5".to_string()));
        assert!(sonnet_4_5
            .aliases
            .contains(&"claude-sonnet-4-5".to_string()));
        assert!(sonnet_4_5
            .aliases
            .contains(&"claude-sonnet-4-5-20250929".to_string()));
    }

    #[test]
    fn test_cache_price_calculations() {
        let template = builtin_claude_official_template();

        // 验证缓存价格计算公式：write = input * 1.25, read = input * 0.1
        for (_, model_price) in template.custom_models.iter() {
            let expected_cache_write =
                (model_price.input_price_per_1m * 1.25 * 100.0).round() / 100.0;
            let expected_cache_read =
                (model_price.input_price_per_1m * 0.1 * 100.0).round() / 100.0;

            let actual_cache_write = model_price
                .cache_write_price_per_1m
                .map(|v| (v * 100.0).round() / 100.0)
                .unwrap_or(0.0);
            let actual_cache_read = model_price
                .cache_read_price_per_1m
                .map(|v| (v * 100.0).round() / 100.0)
                .unwrap_or(0.0);

            assert_eq!(
                actual_cache_write, expected_cache_write,
                "Cache write price mismatch for model with input price {}",
                model_price.input_price_per_1m
            );
            assert_eq!(
                actual_cache_read, expected_cache_read,
                "Cache read price mismatch for model with input price {}",
                model_price.input_price_per_1m
            );
        }
    }

    #[test]
    fn test_builtin_openai_template() {
        let template = builtin_openai_official_template();

        // 验证基本信息
        assert_eq!(template.id, "builtin_openai");
        assert!(template.is_default_preset);
        assert!(template.is_full_custom());

        // 验证包含 1 个模型
        assert_eq!(template.custom_models.len(), 1);

        // 验证 GPT-5.2 Codex 价格
        let gpt_5_2 = template.custom_models.get("gpt-5.2-codex").unwrap();
        assert_eq!(gpt_5_2.provider, "openai");
        assert_eq!(gpt_5_2.input_price_per_1m, 3.0);
        assert_eq!(gpt_5_2.output_price_per_1m, 12.0);
        assert_eq!(gpt_5_2.cache_write_price_per_1m, None); // OpenAI 不收费缓存创建
        assert_eq!(gpt_5_2.cache_read_price_per_1m, Some(1.5));
        assert_eq!(gpt_5_2.reasoning_output_price_per_1m, None);
        assert_eq!(gpt_5_2.aliases.len(), 3);

        // 验证别名
        assert!(gpt_5_2.aliases.contains(&"gpt-5.2-codex".to_string()));
        assert!(gpt_5_2.aliases.contains(&"gpt-5.2".to_string()));
        assert!(gpt_5_2.aliases.contains(&"gpt-5-2-codex".to_string()));
    }
}
