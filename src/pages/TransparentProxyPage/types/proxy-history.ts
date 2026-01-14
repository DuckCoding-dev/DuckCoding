// 透明代理相关类型定义

/**
 * 代理会话记录
 * 用于展示代理请求的历史记录
 */
export interface ProxySessionRecord {
  /** 会话唯一标识符 */
  sessionId: string;
  /** 会话启动时间（ISO 8601 格式） */
  startTime: string;
  /** 当前使用的配置名称 */
  configUsed: string;
}

/**
 * 工具 ID 类型（严格限制为四个支持的工具）
 */
export type ToolId = 'claude-code' | 'codex' | 'gemini-cli' | 'amp-code';

/**
 * 工具元数据（用于 UI 展示）
 */
export interface ToolMetadata {
  id: ToolId;
  name: string;
  icon?: string;
}
