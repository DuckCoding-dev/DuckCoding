// Claude Code 代理内容组件
// 显示代理请求历史记录表格

import { Button } from '@/components/ui/button';
import { FileText, Trash2 } from 'lucide-react';
import type { ProxySessionRecord } from '../../types/proxy-history';

interface ClaudeContentProps {
  /** 会话历史记录（暂时为空数组，等待后续实现） */
  sessions?: ProxySessionRecord[];
}

/**
 * Claude Code 代理内容组件
 *
 * 功能：
 * - 展示代理请求历史记录表格
 * - 列：会话标识符 | 会话启动时间 | 目前使用配置 | 操作
 *
 * 当前状态：预留表格结构，使用空状态占位
 */
export function ClaudeContent({ sessions = [] }: ClaudeContentProps) {
  // 空状态展示
  if (sessions.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16 text-center">
        <FileText className="h-12 w-12 text-muted-foreground mb-4" />
        <h3 className="text-lg font-semibold mb-2">暂无代理会话记录</h3>
        <p className="text-sm text-muted-foreground max-w-md">
          启动代理后，Claude Code 的请求会话记录将显示在此处。
        </p>
      </div>
    );
  }

  // 表格展示（使用简单的 HTML table + Tailwind CSS）
  return (
    <div className="rounded-lg border overflow-hidden">
      <table className="w-full">
        <thead className="bg-muted/50">
          <tr>
            <th className="px-4 py-3 text-left text-sm font-medium text-muted-foreground w-[200px]">
              会话标识符
            </th>
            <th className="px-4 py-3 text-left text-sm font-medium text-muted-foreground w-[180px]">
              会话启动时间
            </th>
            <th className="px-4 py-3 text-left text-sm font-medium text-muted-foreground">
              目前使用配置
            </th>
            <th className="px-4 py-3 text-right text-sm font-medium text-muted-foreground w-[100px]">
              操作
            </th>
          </tr>
        </thead>
        <tbody>
          {sessions.map((session) => (
            <tr key={session.sessionId} className="border-t hover:bg-muted/30 transition-colors">
              <td className="px-4 py-3 font-mono text-sm">{session.sessionId}</td>
              <td className="px-4 py-3 text-sm text-muted-foreground">
                {new Date(session.startTime).toLocaleString('zh-CN')}
              </td>
              <td className="px-4 py-3 text-sm">{session.configUsed}</td>
              <td className="px-4 py-3 text-right">
                <Button variant="ghost" size="sm" className="h-8">
                  <Trash2 className="h-3 w-3" />
                </Button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
