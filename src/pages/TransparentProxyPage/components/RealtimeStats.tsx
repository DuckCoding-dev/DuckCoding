// 实时 Token 统计组件
// 展示当前会话的 Token 消耗情况，自动刷新

import { useState, useEffect, useCallback } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Loader2, Activity, TrendingUp, Database, Zap } from 'lucide-react';
import { getSessionStats } from '@/lib/tauri-commands';
import type { SessionStats } from '@/types/token-stats';
import { TOOL_TYPE_NAMES, type ToolType } from '@/types/token-stats';

interface RealtimeStatsProps {
  /** 会话 ID */
  sessionId: string;
  /** 工具类型 */
  toolType: ToolType;
  /** 刷新间隔（毫秒），默认 3000ms */
  refreshInterval?: number;
  /** 是否自动刷新，默认 true */
  autoRefresh?: boolean;
}

/**
 * 格式化 Token 数量，添加千位分隔符
 */
function formatTokens(count: number): string {
  return count.toLocaleString('zh-CN');
}

/**
 * Token 统计卡片组件
 */
function StatCard({
  title,
  value,
  icon: Icon,
  color,
}: {
  title: string;
  value: string;
  icon: React.ElementType;
  color: string;
}) {
  return (
    <div className="flex items-center gap-3 p-3 rounded-lg border border-border/50 bg-muted/30">
      <div className={`p-2 rounded-md ${color}`}>
        <Icon className="h-4 w-4 text-white" />
      </div>
      <div className="flex-1 min-w-0">
        <p className="text-xs text-muted-foreground truncate">{title}</p>
        <p className="text-lg font-semibold tabular-nums">{value}</p>
      </div>
    </div>
  );
}

/**
 * 实时 Token 统计组件
 */
export function RealtimeStats({
  sessionId,
  toolType,
  refreshInterval = 3000,
  autoRefresh = true,
}: RealtimeStatsProps) {
  const [stats, setStats] = useState<SessionStats | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);

  // 获取统计数据
  const fetchStats = useCallback(async () => {
    try {
      const data = await getSessionStats(toolType, sessionId);
      setStats(data);
      setError(null);
      setLastUpdated(new Date());
    } catch (err) {
      console.error('Failed to fetch session stats:', err);
      setError(err instanceof Error ? err.message : '加载统计数据失败');
    } finally {
      setIsLoading(false);
    }
  }, [sessionId, toolType]);

  // 初始加载和定时刷新
  useEffect(() => {
    fetchStats();

    if (autoRefresh && refreshInterval > 0) {
      const timer = setInterval(fetchStats, refreshInterval);
      return () => clearInterval(timer);
    }
  }, [fetchStats, autoRefresh, refreshInterval]);

  // 计算总 Token 数
  const totalTokens =
    (stats?.total_input ?? 0) + (stats?.total_output ?? 0) + (stats?.total_cache_creation ?? 0);

  // 加载状态
  if (isLoading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Activity className="h-5 w-5" />
            实时统计
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-center py-8">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            <span className="ml-2 text-sm text-muted-foreground">加载中...</span>
          </div>
        </CardContent>
      </Card>
    );
  }

  // 错误状态
  if (error) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Activity className="h-5 w-5" />
            实时统计
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-center py-8 text-destructive">
            <p className="text-sm">{error}</p>
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="flex items-center gap-2">
            <Activity className="h-5 w-5" />
            实时统计
          </CardTitle>
          <div className="flex items-center gap-2">
            <Badge variant="outline">{TOOL_TYPE_NAMES[toolType]}</Badge>
            {lastUpdated && (
              <span className="text-xs text-muted-foreground">
                更新于 {lastUpdated.toLocaleTimeString('zh-CN')}
              </span>
            )}
          </div>
        </div>
      </CardHeader>
      <CardContent>
        {stats && (
          <div className="space-y-4">
            {/* Token 统计网格 */}
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
              <StatCard
                title="输入 Tokens"
                value={formatTokens(stats.total_input)}
                icon={TrendingUp}
                color="bg-blue-500"
              />
              <StatCard
                title="输出 Tokens"
                value={formatTokens(stats.total_output)}
                icon={TrendingUp}
                color="bg-green-500"
              />
              <StatCard
                title="缓存创建 Tokens"
                value={formatTokens(stats.total_cache_creation)}
                icon={Database}
                color="bg-yellow-500"
              />
              <StatCard
                title="缓存读取 Tokens"
                value={formatTokens(stats.total_cache_read)}
                icon={Zap}
                color="bg-purple-500"
              />
              <StatCard
                title="总计 Tokens"
                value={formatTokens(totalTokens)}
                icon={Activity}
                color="bg-orange-500"
              />
              <StatCard
                title="请求次数"
                value={formatTokens(stats.request_count)}
                icon={Activity}
                color="bg-gray-500"
              />
            </div>

            {/* 额外信息 */}
            {stats.request_count > 0 && (
              <div className="pt-3 border-t border-border/50">
                <div className="flex items-center justify-between text-xs text-muted-foreground">
                  <span>平均每次请求消耗</span>
                  <span className="font-medium tabular-nums">
                    {Math.round(totalTokens / stats.request_count)} Tokens
                  </span>
                </div>
              </div>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
