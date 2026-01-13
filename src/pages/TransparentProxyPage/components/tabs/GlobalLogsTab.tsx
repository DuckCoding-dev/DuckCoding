// 全局日志 Tab 组件
// 复用 LogsTable 组件展示所有会话的日志

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
    <div className="mt-4">
      {/* 复用 LogsTable 组件 */}
      <LogsTable initialToolType={toolId} />
    </div>
  );
}
