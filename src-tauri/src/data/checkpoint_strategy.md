# SQLite WAL Checkpoint 策略设计

## 背景

SQLite WAL (Write-Ahead Logging) 模式通过先写入 WAL 文件再定期合并到主文件来提升并发性能。但如果不及时 checkpoint，WAL 文件会无限增长。

## Checkpoint 模式

| 模式     | 行为               | 性能影响 | 使用场景          |
| -------- | ------------------ | -------- | ----------------- |
| PASSIVE  | 尝试回写但不阻塞   | 最小     | 高频操作后        |
| FULL     | 等待读者完成后回写 | 中等     | 定期维护          |
| RESTART  | FULL + 重置WAL     | 中等     | 同FULL            |
| TRUNCATE | 强制清空WAL        | 最大     | 手动触发/应用关闭 |

## Token 统计数据库策略

### 写入机制

- **批量写入**：收集写入事件到缓冲区
- **触发条件**：10条记录或100ms间隔
- **异步执行**：不阻塞代理响应

### Checkpoint 分层

1. **批量写入后**：PASSIVE checkpoint（每次批量写入）
2. **定期维护**：每5分钟执行TRUNCATE（后台任务）
3. **应用关闭**：强制TRUNCATE（刷盘缓冲区+清空WAL）
4. **手动触发**：提供命令执行TRUNCATE

## 会话记录数据库策略

### 已有机制

- 批量写入：10条或100ms触发
- 定期清理：每小时清理旧会话

### Checkpoint 优化

1. **批量写入后**：PASSIVE checkpoint
2. **清理任务后**：TRUNCATE checkpoint
3. **低频操作**：删除/更新使用PASSIVE
4. **应用关闭**：TRUNCATE checkpoint

## 实现要点

### 性能考虑

- PASSIVE不阻塞，适合高频场景
- TRUNCATE完全清空，适合低频场景
- 避免在写入循环内执行TRUNCATE

### 数据完整性

- 应用关闭时强制刷盘
- 定期TRUNCATE防止WAL过大
- 批量写入减少磁盘IO次数

### 监控指标

- WAL文件大小
- Checkpoint执行频率
- 写入延迟统计
