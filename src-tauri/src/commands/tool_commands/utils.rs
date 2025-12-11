/// 解析版本号字符串，处理特殊格式
///
/// 支持格式：
/// - "2.0.61" -> "2.0.61"
/// - "2.0.61 (Claude Code)" -> "2.0.61"
/// - "codex-cli 0.65.0" -> "0.65.0"
/// - "v1.2.3" -> "1.2.3"
pub fn parse_version_string(raw: &str) -> String {
    let trimmed = raw.trim();

    // 1. 处理括号格式：2.0.61 (Claude Code) -> 2.0.61
    if let Some(idx) = trimmed.find('(') {
        return trimmed[..idx].trim().to_string();
    }

    // 2. 处理空格分隔格式：codex-cli 0.65.0 -> 0.65.0
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.len() > 1 {
        // 查找第一个以数字开头的部分
        for part in parts {
            if part.chars().next().is_some_and(|c| c.is_numeric()) {
                return part.trim_start_matches('v').to_string();
            }
        }
    }

    // 3. 移除 'v' 前缀：v1.2.3 -> 1.2.3
    trimmed.trim_start_matches('v').to_string()
}
