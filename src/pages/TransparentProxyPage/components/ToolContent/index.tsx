// ToolContent 工厂组件
// 根据 toolId 动态渲染对应的内容组件

import type { ToolId, ProxySessionRecord } from '../../types/proxy-history';
import { ClaudeContent } from './ClaudeContent';
import { CodexContent } from './CodexContent';
import { GeminiContent } from './GeminiContent';

interface ToolContentProps {
  /** 工具 ID，用于决定渲染哪个内容组件 */
  toolId: ToolId;
  /** 会话历史记录（可选，仅 Claude Code 使用） */
  sessions?: ProxySessionRecord[];
}

/**
 * ToolContent 工厂组件
 *
 * 功能：
 * - 根据 toolId 动态渲染对应的内容组件
 * - 支持类型安全的工具切换
 * - 符合开放封闭原则（OCP）：新增工具只需添加 case 分支
 *
 * 扩展方式：
 * 1. 在 types/proxy-history.ts 添加新工具到 ToolId 类型
 * 2. 实现新工具的内容组件（如 NewToolContent.tsx）
 * 3. 在此处添加新的 case 分支
 */
export function ToolContent({ toolId, sessions }: ToolContentProps) {
  switch (toolId) {
    case 'claude-code':
      return <ClaudeContent sessions={sessions} />;
    case 'codex':
      return <CodexContent />;
    case 'gemini-cli':
      return <GeminiContent />;
  }
}
