// Token 统计页面
// 整合实时统计和历史日志展示

import { useEffect, useState } from 'react';
import { emit } from '@tauri-apps/api/event';
import { Button } from '@/components/ui/button';
import { ArrowLeft, Database, Trash2, AlertCircle } from 'lucide-react';
import { useToast } from '@/hooks/use-toast';
import { RealtimeStats } from '../TransparentProxyPage/components/RealtimeStats';
import { LogsTable } from '../TransparentProxyPage/components/LogsTable';
import { cleanupTokenLogs, getTokenStatsSummary, getTokenStatsConfig } from '@/lib/tauri-commands';
import type { DatabaseSummary, TokenStatsConfig, ToolType } from '@/types/token-stats';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from '@/components/ui/alert-dialog';

interface TokenStatisticsPageProps {
  /** 会话ID（从导航传入，用于筛选日志） */
  sessionId?: string;
  /** 工具类型（从导航传入，用于筛选日志） */
  toolType?: ToolType;
}

/**
 * Token 统计页面组件
 */
export default function TokenStatisticsPage({
  sessionId: propsSessionId,
  toolType: propsToolType,
}: TokenStatisticsPageProps = {}) {
  const { toast } = useToast();

  // 使用传入的参数或默认值
  const sessionId = propsSessionId;
  const toolType = propsToolType;

  // 返回透明代理页面
  const handleGoBack = async () => {
    try {
      await emit('app-navigate', { tab: 'transparent-proxy' });
    } catch (error) {
      console.error('导航失败:', error);
      toast({
        title: '导航失败',
        description: '无法返回透明代理页面',
        variant: 'destructive',
      });
    }
  };

  // 数据库摘要
  const [summary, setSummary] = useState<DatabaseSummary | null>(null);
  const [config, setConfig] = useState<TokenStatsConfig | null>(null);
  const [isCleaningUp, setIsCleaningUp] = useState(false);

  // 加载数据库摘要和配置
  useEffect(() => {
    const loadData = async () => {
      try {
        const [summaryData, configData] = await Promise.all([
          getTokenStatsSummary(),
          getTokenStatsConfig(),
        ]);
        setSummary(summaryData);
        setConfig(configData);
      } catch (error) {
        console.error('Failed to load statistics data:', error);
      }
    };

    loadData();
  }, []);

  // 手动清理日志
  const handleCleanup = async () => {
    if (!config) return;

    setIsCleaningUp(true);
    try {
      const deletedCount = await cleanupTokenLogs(config.retention_days, config.max_log_count);
      toast({
        title: '清理成功',
        description: `已清理 ${deletedCount} 条旧日志`,
      });

      // 重新加载摘要
      const newSummary = await getTokenStatsSummary();
      setSummary(newSummary);
    } catch (error) {
      console.error('Failed to cleanup logs:', error);
      toast({
        title: '清理失败',
        description: String(error),
        variant: 'destructive',
      });
    } finally {
      setIsCleaningUp(false);
    }
  };

  // 格式化日期
  const formatDate = (timestamp?: number) => {
    if (!timestamp) return '无';
    return new Date(timestamp).toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  return (
    <div className="container mx-auto p-6 space-y-6">
      {/* 页头 */}
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <Button variant="ghost" size="sm" onClick={handleGoBack}>
              <ArrowLeft className="h-4 w-4" />
            </Button>
            <h1 className="text-2xl font-bold">Token 统计</h1>
          </div>
          <p className="text-sm text-muted-foreground">查看透明代理的 Token 使用情况和请求历史</p>
        </div>

        {/* 操作按钮 */}
        <div className="flex items-center gap-2">
          {/* 数据库信息 */}
          {summary && (
            <div className="flex items-center gap-4 px-4 py-2 rounded-md bg-muted/50 text-sm">
              <div className="flex items-center gap-2">
                <Database className="h-4 w-4 text-muted-foreground" />
                <span className="text-muted-foreground">总记录:</span>
                <span className="font-medium">{summary.total_logs.toLocaleString('zh-CN')}</span>
              </div>
              <div className="h-4 w-px bg-border" />
              <div className="text-muted-foreground">
                {summary.oldest_timestamp && summary.newest_timestamp && (
                  <span>
                    {formatDate(summary.oldest_timestamp)} - {formatDate(summary.newest_timestamp)}
                  </span>
                )}
              </div>
            </div>
          )}

          {/* 清理按钮 */}
          <AlertDialog>
            <AlertDialogTrigger asChild>
              <Button variant="outline" size="sm" disabled={!config}>
                <Trash2 className="h-4 w-4" />
                清理旧日志
              </Button>
            </AlertDialogTrigger>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle className="flex items-center gap-2">
                  <AlertCircle className="h-5 w-5 text-orange-500" />
                  确认清理日志
                </AlertDialogTitle>
                <AlertDialogDescription className="space-y-2">
                  <p>此操作将根据当前配置清理旧日志：</p>
                  {config && (
                    <ul className="list-disc list-inside space-y-1 text-sm">
                      {config.retention_days && <li>保留最近 {config.retention_days} 天的日志</li>}
                      {config.max_log_count && (
                        <li>最多保留 {config.max_log_count.toLocaleString('zh-CN')} 条记录</li>
                      )}
                    </ul>
                  )}
                  <p className="text-destructive font-medium mt-2">此操作不可撤销！</p>
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>取消</AlertDialogCancel>
                <AlertDialogAction onClick={handleCleanup} disabled={isCleaningUp}>
                  {isCleaningUp ? '清理中...' : '确认清理'}
                </AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
        </div>
      </div>

      {/* 实时统计（如果提供了 sessionId 和 toolType） */}
      {sessionId && toolType && <RealtimeStats sessionId={sessionId} toolType={toolType} />}

      {/* 历史日志表格 */}
      <LogsTable initialToolType={toolType} initialSessionId={sessionId} />

      {/* 配置提示 */}
      {config && config.auto_cleanup_enabled && (
        <div className="flex items-start gap-2 p-4 rounded-lg border border-blue-200 bg-blue-50 dark:border-blue-900 dark:bg-blue-950/20">
          <AlertCircle className="h-5 w-5 text-blue-600 dark:text-blue-400 mt-0.5" />
          <div className="flex-1 text-sm">
            <p className="font-medium text-blue-900 dark:text-blue-100">自动清理已启用</p>
            <p className="text-blue-700 dark:text-blue-300 mt-1">
              系统将自动清理
              {config.retention_days && ` ${config.retention_days} 天前的日志`}
              {config.retention_days && config.max_log_count && '，并'}
              {config.max_log_count &&
                ` 保留最多 ${config.max_log_count.toLocaleString('zh-CN')} 条记录`}
              。可在设置页面修改配置。
            </p>
          </div>
        </div>
      )}
    </div>
  );
}
