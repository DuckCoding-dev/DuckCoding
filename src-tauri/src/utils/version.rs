/// 版本号解析和处理工具
///
/// 提供统一的版本号解析逻辑，支持多种常见格式
use once_cell::sync::Lazy;
use regex::Regex;

/// 版本号正则表达式（支持语义化版本）
static VERSION_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"v?(\d+\.\d+\.\d+(?:-[\w.]+)?)").expect("版本正则表达式无效"));

/// 解析版本号字符串，处理多种常见格式
///
/// 支持格式：
/// - "2.0.61" -> "2.0.61"
/// - "v1.2.3" -> "1.2.3"
/// - "2.0.61 (Claude Code)" -> "2.0.61"
/// - "codex-cli 0.65.0" -> "0.65.0"
/// - "1.2.3-beta.1" -> "1.2.3-beta.1"
///
/// # 实现策略
/// 1. 使用正则表达式提取标准语义化版本号（优先）
/// 2. 回退到手动解析特殊格式
///
/// # Examples
///
/// ```
/// use duckcoding::utils::version::parse_version_string;
///
/// assert_eq!(parse_version_string("2.0.61"), "2.0.61");
/// assert_eq!(parse_version_string("v1.2.3"), "1.2.3");
/// assert_eq!(parse_version_string("2.0.61 (Claude Code)"), "2.0.61");
/// assert_eq!(parse_version_string("codex-cli 0.65.0"), "0.65.0");
/// ```
pub fn parse_version_string(raw: &str) -> String {
    let trimmed = raw.trim();

    // 策略 1: 使用正则表达式提取标准版本号（推荐）
    if let Some(captures) = VERSION_REGEX.captures(trimmed) {
        if let Some(version) = captures.get(1) {
            return version.as_str().to_string();
        }
    }

    // 策略 2: 处理括号格式（兼容旧实现）
    // 格式：2.0.61 (Claude Code) -> 2.0.61
    if let Some(idx) = trimmed.find('(') {
        let before_bracket = trimmed[..idx].trim();
        if !before_bracket.is_empty() {
            return before_bracket.to_string();
        }
    }

    // 策略 3: 处理空格分隔格式（兼容旧实现）
    // 格式：codex-cli 0.65.0 -> 0.65.0
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.len() > 1 {
        // 查找第一个以数字开头的部分
        for part in parts {
            if part.chars().next().is_some_and(|c| c.is_numeric()) {
                return part.trim_start_matches('v').to_string();
            }
        }
    }

    // 策略 4: 移除 'v' 前缀作为最后的回退
    trimmed.trim_start_matches('v').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_standard_version() {
        assert_eq!(parse_version_string("2.0.61"), "2.0.61");
        assert_eq!(parse_version_string("1.2.3"), "1.2.3");
    }

    #[test]
    fn test_parse_version_with_v_prefix() {
        assert_eq!(parse_version_string("v1.2.3"), "1.2.3");
        assert_eq!(parse_version_string("v2.0.61"), "2.0.61");
    }

    #[test]
    fn test_parse_version_with_bracket() {
        assert_eq!(parse_version_string("2.0.61 (Claude Code)"), "2.0.61");
        assert_eq!(parse_version_string("1.0.0 (beta)"), "1.0.0");
    }

    #[test]
    fn test_parse_version_with_prefix() {
        assert_eq!(parse_version_string("codex-cli 0.65.0"), "0.65.0");
        assert_eq!(parse_version_string("tool 1.2.3"), "1.2.3");
    }

    #[test]
    fn test_parse_version_with_prerelease() {
        assert_eq!(parse_version_string("1.2.3-beta.1"), "1.2.3-beta.1");
        assert_eq!(parse_version_string("2.0.0-rc.2"), "2.0.0-rc.2");
    }

    #[test]
    fn test_parse_complex_version() {
        assert_eq!(
            parse_version_string("v1.2.3-alpha.4 (test build)"),
            "1.2.3-alpha.4"
        );
    }
}
