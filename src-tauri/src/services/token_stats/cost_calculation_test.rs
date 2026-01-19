//! 成本计算集成测试
//!
//! 验证成本计算逻辑是否正确工作

#[cfg(test)]
mod tests {
    use crate::services::pricing::PRICING_MANAGER;
    use crate::services::token_stats::processor::create_processor;
    use serde_json::json;

    #[test]
    fn test_cost_calculation_with_claude_3_5_sonnet() {
        // 测试 Claude 3.5 Sonnet 20241022 版本的成本计算
        let model = "claude-sonnet-4-5-20250929";
        let input_tokens = 100;
        let output_tokens = 50;
        let cache_creation_tokens = 10;
        let cache_read_tokens = 20;

        // 使用默认模板计算成本
        let result = PRICING_MANAGER.calculate_cost(
            None,                // 使用默认模板
            Some("claude-code"), // 工具 ID
            model,
            input_tokens,
            output_tokens,
            cache_creation_tokens,
            cache_read_tokens,
            0, // reasoning_tokens
        );

        // 验证计算成功
        assert!(result.is_ok(), "成本计算应该成功: {:?}", result.err());

        let breakdown = result.unwrap();

        // 验证使用了正确的模板
        assert_eq!(breakdown.template_id, "builtin_claude");

        // 验证成本计算正确（Claude 3.5 Sonnet: $3/1M input, $15/1M output）
        // input: 100 * 3.0 / 1,000,000 = 0.0003
        // output: 50 * 15.0 / 1,000,000 = 0.00075
        // cache_write: 10 * 3.75 / 1,000,000 = 0.0000375
        // cache_read: 20 * 0.3 / 1,000,000 = 0.000006
        // total: 0.0003 + 0.00075 + 0.0000375 + 0.000006 = 0.0010935

        println!("实际计算结果:");
        println!("  输入价格: {:.10}", breakdown.input_price);
        println!("  输出价格: {:.10}", breakdown.output_price);
        println!("  缓存写入价格: {:.10}", breakdown.cache_write_price);
        println!("  缓存读取价格: {:.10}", breakdown.cache_read_price);
        println!("  总成本: {:.10}", breakdown.total_cost);

        assert!((breakdown.input_price - 0.0003).abs() < 1e-9);
        assert!((breakdown.output_price - 0.00075).abs() < 1e-9);
        assert!((breakdown.cache_write_price - 0.0000375).abs() < 1e-9);
        assert!((breakdown.cache_read_price - 0.000006).abs() < 1e-9);
        assert!(
            (breakdown.total_cost - 0.0010935).abs() < 1e-7,
            "expected 0.0010935, got {}",
            breakdown.total_cost
        );

        println!("✅ 成本计算测试通过:");
        println!("  模型: {}", model);
        println!("  输入价格: {:.6}", breakdown.input_price);
        println!("  输出价格: {:.6}", breakdown.output_price);
        println!("  缓存写入价格: {:.6}", breakdown.cache_write_price);
        println!("  缓存读取价格: {:.6}", breakdown.cache_read_price);
        println!("  总成本: {:.6}", breakdown.total_cost);
    }

    #[test]
    fn test_cost_calculation_with_different_models() {
        // 测试不同模型的成本计算

        // Claude Opus 4.5: $5 input / $25 output
        let opus_result = PRICING_MANAGER.calculate_cost(
            None,
            Some("claude-code"),
            "claude-opus-4.5",
            1000,
            500,
            0,
            0,
            0,
        );
        assert!(opus_result.is_ok());
        let opus_breakdown = opus_result.unwrap();
        assert!((opus_breakdown.input_price - 0.005).abs() < 1e-9); // 1000 * 5 / 1M
        assert!((opus_breakdown.output_price - 0.0125).abs() < 1e-9); // 500 * 25 / 1M

        // Claude Sonnet 4.5: $3 input / $15 output
        let sonnet_result = PRICING_MANAGER.calculate_cost(
            None,
            Some("claude-code"),
            "claude-sonnet-4.5",
            1000,
            500,
            0,
            0,
            0,
        );
        assert!(sonnet_result.is_ok());
        let sonnet_breakdown = sonnet_result.unwrap();
        assert!((sonnet_breakdown.input_price - 0.003).abs() < 1e-9); // 1000 * 3 / 1M
        assert!((sonnet_breakdown.output_price - 0.0075).abs() < 1e-9); // 500 * 15 / 1M

        // Claude Haiku 3.5: $0.8 input / $4 output
        let haiku_result = PRICING_MANAGER.calculate_cost(
            None,
            Some("claude-code"),
            "claude-haiku-3.5",
            1000,
            500,
            0,
            0,
            0,
        );
        assert!(haiku_result.is_ok());
        let haiku_breakdown = haiku_result.unwrap();
        assert!((haiku_breakdown.input_price - 0.0008).abs() < 1e-9); // 1000 * 0.8 / 1M
        assert!((haiku_breakdown.output_price - 0.002).abs() < 1e-9); // 500 * 4 / 1M

        println!("✅ 多模型成本计算测试通过");
    }

    #[test]
    fn test_token_extraction_from_response() {
        // 测试从响应中提取 Token 信息
        let processor = create_processor("claude-code").unwrap();

        let request_body = json!({
            "model": "claude-sonnet-4-5-20250929",
            "messages": []
        });

        let response_json = json!({
            "id": "msg_test_123",
            "model": "claude-sonnet-4-5-20250929",
            "usage": {
                "input_tokens": 100,
                "output_tokens": 50,
                "cache_creation_input_tokens": 10,
                "cache_read_input_tokens": 20
            }
        });

        let token_info = processor
            .process_json_response(
                &serde_json::to_vec(&request_body).unwrap(),
                &response_json,
            )
            .unwrap();

        assert_eq!(token_info.input_tokens, 100);
        assert_eq!(token_info.output_tokens, 50);
        assert_eq!(token_info.cache_creation_tokens, 10);
        assert_eq!(token_info.cache_read_tokens, 20);
        assert_eq!(token_info.message_id, "msg_test_123");

        println!("✅ Token 提取测试通过");
    }

    #[test]
    fn test_end_to_end_cost_calculation() {
        // 端到端测试：从响应提取 Token -> 计算成本
        let processor = create_processor("claude-code").unwrap();

        let request_body = json!({
            "model": "claude-sonnet-4-5-20250929",
            "messages": []
        });

        let response_json = json!({
            "id": "msg_end_to_end",
            "model": "claude-sonnet-4-5-20250929",
            "usage": {
                "input_tokens": 1000,
                "output_tokens": 500,
                "cache_creation_input_tokens": 100,
                "cache_read_input_tokens": 200
            }
        });

        // 步骤1: 提取 Token
        let token_info = processor
            .process_json_response(
                &serde_json::to_vec(&request_body).unwrap(),
                &response_json,
            )
            .unwrap();

        // 步骤2: 计算成本
        let result = PRICING_MANAGER.calculate_cost(
            None,
            Some("claude-code"), // 工具 ID
            "claude-sonnet-4-5-20250929",
            token_info.input_tokens,
            token_info.output_tokens,
            token_info.cache_creation_tokens,
            token_info.cache_read_tokens,
            0, // reasoning_tokens
        );

        assert!(result.is_ok());
        let breakdown = result.unwrap();

        // 验证总成本不为 0
        assert!(breakdown.total_cost > 0.0, "总成本应该大于 0");

        // 预期成本计算:
        // input: 1000 * 3.0 / 1,000,000 = 0.003
        // output: 500 * 15.0 / 1,000,000 = 0.0075
        // cache_write: 100 * 3.75 / 1,000,000 = 0.000375
        // cache_read: 200 * 0.3 / 1,000,000 = 0.00006
        // total: 0.003 + 0.0075 + 0.000375 + 0.00006 = 0.010935

        println!("端到端实际计算结果:");
        println!("  输入价格: {:.10}", breakdown.input_price);
        println!("  输出价格: {:.10}", breakdown.output_price);
        println!("  缓存写入价格: {:.10}", breakdown.cache_write_price);
        println!("  缓存读取价格: {:.10}", breakdown.cache_read_price);
        println!("  总成本: {:.10}", breakdown.total_cost);

        assert!(
            (breakdown.total_cost - 0.010935).abs() < 1e-6,
            "expected 0.010935, got {}",
            breakdown.total_cost
        );

        println!("✅ 端到端成本计算测试通过");
        println!(
            "  输入: {} tokens -> ${:.6}",
            token_info.input_tokens, breakdown.input_price
        );
        println!(
            "  输出: {} tokens -> ${:.6}",
            token_info.output_tokens, breakdown.output_price
        );
        println!(
            "  缓存写入: {} tokens -> ${:.6}",
            token_info.cache_creation_tokens, breakdown.cache_write_price
        );
        println!(
            "  缓存读取: {} tokens -> ${:.6}",
            token_info.cache_read_tokens, breakdown.cache_read_price
        );
        println!("  总成本: ${:.6}", breakdown.total_cost);
    }

    #[test]
    fn test_codex_end_to_end_cost_calculation() {
        // Codex 端到端测试：从响应提取 Token -> 计算成本（使用 builtin_openai 模板）
        let processor = create_processor("codex").unwrap();

        let request_body = json!({
            "model": "gpt-5.2-codex",
            "messages": []
        });

        // 模拟 Codex 响应：input_tokens 包含缓存的 token
        let response_json = json!({
            "id": "resp_codex_test",
            "model": "gpt-5.2-codex",
            "usage": {
                "input_tokens": 10000,           // 总输入（包含缓存）
                "input_tokens_details": {
                    "cached_tokens": 8000         // 缓存读取
                },
                "output_tokens": 500,
                "output_tokens_details": {
                    "reasoning_tokens": 0
                }
            }
        });

        // 步骤1: 提取 Token
        let token_info = processor
            .process_json_response(
                &serde_json::to_vec(&request_body).unwrap(),
                &response_json,
            )
            .unwrap();

        // 验证 Token 提取正确（新输入 = 总输入 - 缓存）
        assert_eq!(token_info.input_tokens, 2000); // 10000 - 8000
        assert_eq!(token_info.output_tokens, 500);
        assert_eq!(token_info.cache_creation_tokens, 0);
        assert_eq!(token_info.cache_read_tokens, 8000);

        // 步骤2: 计算成本（使用 builtin_openai 模板）
        let result = PRICING_MANAGER.calculate_cost(
            Some("builtin_openai"), // 使用 OpenAI 模板
            Some("codex"),          // 工具 ID
            "gpt-5.2-codex",
            token_info.input_tokens,
            token_info.output_tokens,
            token_info.cache_creation_tokens,
            token_info.cache_read_tokens,
            0, // reasoning_tokens
        );

        assert!(result.is_ok());
        let breakdown = result.unwrap();

        // 验证使用了正确的模板
        assert_eq!(breakdown.template_id, "builtin_openai");

        // 预期成本计算（gpt-5.2-codex: $3 input / $12 output / $1.5 cache read）
        // input: 2000 * 3.0 / 1,000,000 = 0.006
        // output: 500 * 12.0 / 1,000,000 = 0.006
        // cache_write: 0（OpenAI 不收费）
        // cache_read: 8000 * 1.5 / 1,000,000 = 0.012
        // total: 0.006 + 0.006 + 0 + 0.012 = 0.024

        println!("Codex 端到端实际计算结果:");
        println!("  输入价格: {:.10}", breakdown.input_price);
        println!("  输出价格: {:.10}", breakdown.output_price);
        println!("  缓存写入价格: {:.10}", breakdown.cache_write_price);
        println!("  缓存读取价格: {:.10}", breakdown.cache_read_price);
        println!("  总成本: {:.10}", breakdown.total_cost);

        assert!((breakdown.input_price - 0.006).abs() < 1e-9);
        assert!((breakdown.output_price - 0.006).abs() < 1e-9);
        assert!((breakdown.cache_write_price - 0.0).abs() < 1e-9); // OpenAI 不收费缓存创建
        assert!((breakdown.cache_read_price - 0.012).abs() < 1e-9);
        assert!(
            (breakdown.total_cost - 0.024).abs() < 1e-6,
            "expected 0.024, got {}",
            breakdown.total_cost
        );

        println!("✅ Codex 端到端成本计算测试通过");
        println!(
            "  新输入: {} tokens -> ${:.6}",
            token_info.input_tokens, breakdown.input_price
        );
        println!(
            "  输出: {} tokens -> ${:.6}",
            token_info.output_tokens, breakdown.output_price
        );
        println!(
            "  缓存读取: {} tokens -> ${:.6}",
            token_info.cache_read_tokens, breakdown.cache_read_price
        );
        println!("  总成本: ${:.6}", breakdown.total_cost);
    }
}
