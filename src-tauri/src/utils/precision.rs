//! 数值精度工具模块
//!
//! 提供价格等需要精确小数位数的序列化/反序列化支持

use serde::{Deserialize, Deserializer, Serializer};

/// 价格字段精度（小数点后6位）
///
/// 用于 serde 的 serialize_with 和 deserialize_with 属性
pub mod price_precision {
    use super::*;

    /// 序列化 f64 为固定 6 位小数
    ///
    /// 注意：对于非常小的数（< 0.0001），JSON可能使用科学计数法
    pub fn serialize<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 四舍五入到 6 位小数
        let multiplier = 1_000_000.0; // 10^6
        let rounded = (value * multiplier).round() / multiplier;
        serializer.serialize_f64(rounded)
    }

    /// 反序列化保持原有精度
    pub fn deserialize<'de, D>(deserializer: D) -> Result<f64, D::Error>
    where
        D: Deserializer<'de>,
    {
        f64::deserialize(deserializer)
    }
}

/// 可选价格字段精度（Option<f64>）
pub mod option_price_precision {
    use super::*;

    /// 序列化 Option<f64> 为固定 6 位小数
    pub fn serialize<S>(value: &Option<f64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(v) => {
                // 四舍五入到 6 位小数
                let multiplier = 1_000_000.0; // 10^6
                let rounded = (v * multiplier).round() / multiplier;
                serializer.serialize_some(&rounded)
            }
            None => serializer.serialize_none(),
        }
    }

    /// 反序列化保持原有精度
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<f64>::deserialize(deserializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestStruct {
        #[serde(with = "price_precision")]
        price: f64,
        #[serde(with = "option_price_precision")]
        optional_price: Option<f64>,
    }

    #[test]
    fn test_price_precision_serialization() {
        let test = TestStruct {
            price: 0.099904123456789,
            optional_price: Some(1.234567890123),
        };

        let json = serde_json::to_string(&test).unwrap();
        println!("Serialized: {}", json);

        // 反序列化验证精度
        let deserialized: TestStruct = serde_json::from_str(&json).unwrap();
        assert!((deserialized.price - 0.099904).abs() < 1e-9);
        assert!((deserialized.optional_price.unwrap() - 1.234568).abs() < 1e-9);
    }

    #[test]
    fn test_price_precision_deserialization() {
        let json = r#"{"price":0.099904,"optional_price":1.234568}"#;
        let test: TestStruct = serde_json::from_str(json).unwrap();

        assert!((test.price - 0.099904).abs() < 1e-9);
        assert!((test.optional_price.unwrap() - 1.234568).abs() < 1e-9);
    }

    #[test]
    fn test_option_price_none() {
        let test = TestStruct {
            price: 0.5,
            optional_price: None,
        };

        let json = serde_json::to_string(&test).unwrap();
        assert!(json.contains("\"optional_price\":null"));

        let deserialized: TestStruct = serde_json::from_str(&json).unwrap();
        assert!((deserialized.price - 0.5).abs() < 1e-9);
        assert!(deserialized.optional_price.is_none());
    }

    #[test]
    fn test_very_small_price() {
        let test = TestStruct {
            price: 0.000001234567,
            optional_price: Some(0.000009876543),
        };

        let json = serde_json::to_string(&test).unwrap();
        println!("Small price serialized: {}", json);

        // 反序列化验证精度（四舍五入到 6 位小数）
        let deserialized: TestStruct = serde_json::from_str(&json).unwrap();
        assert!((deserialized.price - 0.000001).abs() < 1e-9); // 0.000001234567 -> 0.000001
        assert!((deserialized.optional_price.unwrap() - 0.00001).abs() < 1e-9); // 0.000009876543 -> 0.00001
    }

    #[test]
    fn test_typical_api_costs() {
        // 测试典型的 API 成本（例如 Claude API）
        let test = TestStruct {
            price: 0.001234,
            optional_price: Some(0.056789),
        };

        let json = serde_json::to_string(&test).unwrap();
        println!("Typical API cost serialized: {}", json);

        let deserialized: TestStruct = serde_json::from_str(&json).unwrap();
        assert!((deserialized.price - 0.001234).abs() < 1e-9);
        assert!((deserialized.optional_price.unwrap() - 0.056789).abs() < 1e-9);
    }

    #[test]
    fn test_rounding_behavior() {
        // 测试四舍五入行为
        let test = TestStruct {
            price: 0.0000015,                // 应该四舍五入到 0.000002
            optional_price: Some(0.0000014), // 应该四舍五入到 0.000001
        };

        let json = serde_json::to_string(&test).unwrap();
        let deserialized: TestStruct = serde_json::from_str(&json).unwrap();

        assert!((deserialized.price - 0.000002).abs() < 1e-9);
        assert!((deserialized.optional_price.unwrap() - 0.000001).abs() < 1e-9);
    }
}
