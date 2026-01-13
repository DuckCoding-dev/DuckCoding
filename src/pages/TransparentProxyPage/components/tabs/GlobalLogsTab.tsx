// 全局日志 Tab 组件
// 复用 LogsTable 组件展示所有会话的日志

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Info } from 'lucide-react';
import { LogsTable } from '../LogsTable';
import type { ToolId } from '../../types/proxy-history';

interface GlobalLogsTabProps {
  toolId: ToolId;
}

/**
 * 全局日志 Tab 组件
 */
export function GlobalLogsTab({ toolId }: GlobalLogsTabProps) {
  return (
    <div className="space-y-4 mt-4">
      {/* 说明文本 */}
      <Alert>
        <Info className="h-4 w-4" />
        <AlertTitle>全局日志</AlertTitle>
        <AlertDescription>
          显示所有会话的请求日志，支持按会话ID、时间范围、状态、配置过滤。点击展开按钮查看详细信息。
        </AlertDescription>
      </Alert>

      {/* 复用 LogsTable 组件 */}
      <LogsTable initialToolType={toolId} />
    </div>
  );
}
