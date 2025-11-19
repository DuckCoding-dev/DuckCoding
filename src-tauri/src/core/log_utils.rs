use std::time::Instant;

/// 计时器（用于性能分析）
///
/// # 示例
/// ```
/// let timer = Timer::new("install_tool");
/// // 执行安装逻辑
/// drop(timer); // 自动记录耗时
/// ```
pub struct Timer {
    name: String,
    start: Instant,
}

impl Timer {
    /// 创建新的计时器
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        tracing::info!(timer = %name, "计时器启动");

        Self {
            name,
            start: Instant::now(),
        }
    }

    /// 记录中间点
    pub fn checkpoint(&self, label: &str) {
        let elapsed = self.start.elapsed();
        tracing::info!(
            timer = %self.name,
            checkpoint = label,
            elapsed_ms = elapsed.as_millis(),
            "计时检查点"
        );
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        tracing::info!(
            timer = %self.name,
            elapsed_ms = elapsed.as_millis(),
            "操作完成"
        );
    }
}

/// 日志上下文构建器
///
/// # 示例
/// ```
/// LogContext::new("install_tool")
///     .field("tool", "claude-code")
///     .field("version", "1.0.0")
///     .info("开始安装");
/// ```
pub struct LogContext {
    fields: Vec<(String, String)>,
}

impl LogContext {
    pub fn new(operation: impl Into<String>) -> Self {
        let mut ctx = Self { fields: Vec::new() };
        ctx.fields.push(("operation".to_string(), operation.into()));
        ctx
    }

    pub fn field(mut self, key: impl Into<String>, value: impl ToString) -> Self {
        self.fields.push((key.into(), value.to_string()));
        self
    }

    pub fn info(self, message: &str) {
        tracing::info!(fields = ?self.fields, "{}", message);
    }

    pub fn warn(self, message: &str) {
        tracing::warn!(fields = ?self.fields, "{}", message);
    }

    pub fn error(self, message: &str) {
        tracing::error!(fields = ?self.fields, "{}", message);
    }
}
