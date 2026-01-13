// 全局统计 Tab 组件（重构版 v2）
// 使用统一的 useAnalyticsStats Hook，数据一致性保证

import { useMemo } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Loader2, Activity, DollarSign, AlertTriangle, Clock, ChevronDown } from 'lucide-react';
import { useAnalyticsStats } from '@/hooks/useAnalyticsStats';
import { TrendsChart, type DataKey } from '@/pages/TokenStatisticsPage/components/TrendsChart';
import { CustomTimeRangeDialog } from '@/components/dialogs/CustomTimeRangeDialog';
import type { TimeRange, TimeGranularity } from '@/types/analytics';
import type { ToolId } from '../../types/proxy-history';

interface GlobalStatsTabProps {
  toolId: ToolId;
}

/**
 * 统计卡片组件
 */
function StatCard({
  title,
  value,
  icon: Icon,
  color,
  suffix,
}: {
  title: string;
  value: string | number;
  icon: React.ElementType;
  color: string;
  suffix?: string;
}) {
  return (
    <Card>
      <CardContent className="p-6">
        <div className="flex items-center justify-between">
          <div className="flex-1">
            <p className="text-sm text-muted-foreground mb-1">{title}</p>
            <p className="text-2xl font-bold tabular-nums">
              {value}
              {suffix && <span className="text-sm text-muted-foreground ml-1">{suffix}</span>}
            </p>
          </div>
          <div className={`p-3 rounded-md ${color}`}>
            <Icon className="h-5 w-5 text-white" />
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

/**
 * 格式化 Token 数量
 */
function formatTokens(count: number): string {
  if (count >= 1000000) {
    return `${(count / 1000000).toFixed(1)}M`;
  } else if (count >= 1000) {
    return `${(count / 1000).toFixed(1)}K`;
  }
  return count.toLocaleString('zh-CN');
}

/**
 * 加载状态组件
 */
function LoadingState() {
  return (
    <div className="flex flex-col items-center justify-center py-20">
      <Loader2 className="h-12 w-12 animate-spin text-primary mb-4" />
      <p className="text-sm text-muted-foreground">加载统计数据中...</p>
    </div>
  );
}

/**
 * 空状态组件
 */
function EmptyState() {
  return (
    <div className="flex flex-col items-center justify-center py-20 text-center">
      <Activity className="h-12 w-12 text-muted-foreground mb-4" />
      <h3 className="text-lg font-semibold mb-2">暂无统计数据</h3>
      <p className="text-sm text-muted-foreground max-w-md">
        启动代理并发送请求后，统计数据将显示在此处。
      </p>
    </div>
  );
}

/**
 * 预设时间范围标签映射
 */
const PRESET_LABELS: Record<Exclude<TimeRange, 'custom'>, string> = {
  fifteen_minutes: '15分钟',
  thirty_minutes: '30分钟',
  hour: '1小时',
  twelve_hours: '12小时',
  day: '1天',
  week: '7天',
  month: '30天',
};

/**
 * 粒度标签映射
 */
const GRANULARITY_LABELS: Record<TimeGranularity, string> = {
  fifteen_minutes: '15分钟',
  thirty_minutes: '30分钟',
  hour: '小时',
  twelve_hours: '12小时',
  day: '天',
};

/**
 * 全局统计 Tab 组件（重构版 v2）
 */
export function GlobalStatsTab({ toolId }: GlobalStatsTabProps) {
  // 使用统一的 Analytics Hook
  const stats = useAnalyticsStats({ toolId });

  // 图表数据配置
  const costDataKeys: DataKey[] = useMemo(
    () => [
      { key: 'total_cost', color: '#10b981', name: '总成本', formatter: (v) => `$${v.toFixed(4)}` },
      {
        key: 'input_price',
        color: '#3b82f6',
        name: '输入成本',
        formatter: (v) => `$${v.toFixed(4)}`,
      },
      {
        key: 'output_price',
        color: '#f59e0b',
        name: '输出成本',
        formatter: (v) => `$${v.toFixed(4)}`,
      },
    ],
    [],
  );

  const tokenDataKeys: DataKey[] = useMemo(
    () => [
      {
        key: 'input_tokens',
        color: '#3b82f6',
        name: '输入 Token',
        formatter: (v) => v.toLocaleString(),
      },
      {
        key: 'output_tokens',
        color: '#10b981',
        name: '输出 Token',
        formatter: (v) => v.toLocaleString(),
      },
      {
        key: 'cache_creation_tokens',
        color: '#f59e0b',
        name: '缓存写入',
        formatter: (v) => v.toLocaleString(),
      },
      {
        key: 'cache_read_tokens',
        color: '#8b5cf6',
        name: '缓存读取',
        formatter: (v) => v.toLocaleString(),
      },
    ],
    [],
  );

  const responseTimeDataKeys: DataKey[] = useMemo(
    () => [
      {
        key: 'avg_response_time',
        color: '#6366f1',
        name: '平均响应时间',
        formatter: (v) => `${(v / 1000).toFixed(2)}s`,
      },
    ],
    [],
  );

  // 加载状态
  if (stats.loading && !stats.summary) {
    return <LoadingState />;
  }

  // 空状态
  if (!stats.summary || stats.summary.total_requests === 0) {
    return <EmptyState />;
  }

  // 错误状态
  if (stats.error) {
    return (
      <div className="flex flex-col items-center justify-center py-20 text-center">
        <AlertTriangle className="h-12 w-12 text-destructive mb-4" />
        <h3 className="text-lg font-semibold mb-2">加载失败</h3>
        <p className="text-sm text-muted-foreground max-w-md mb-4">{stats.error}</p>
        <Button onClick={stats.refresh}>重试</Button>
      </div>
    );
  }

  return (
    <div className="space-y-6 mt-4">
      {/* 时间范围和粒度控制 - Tab 版本 */}
      <div className="space-y-3">
        {/* 时间范围选择 */}
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-muted-foreground">时间范围</span>
          <div className="flex items-center gap-2">
            <Tabs
              value={stats.timeControl.mode === 'preset' ? stats.timeControl.presetRange : 'custom'}
              onValueChange={(val) => {
                if (val === 'custom') {
                  stats.timeControl.openCustomDialog();
                } else {
                  stats.timeControl.setPresetRange(val as Exclude<TimeRange, 'custom'>);
                }
              }}
              className="flex-1"
            >
              <TabsList>
                <TabsTrigger value="day">1天</TabsTrigger>
                <TabsTrigger value="week">7天</TabsTrigger>
                <TabsTrigger value="month">30天</TabsTrigger>

                {/* 更多预设范围 */}
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button
                      variant={
                        stats.timeControl.mode === 'preset' &&
                        !['day', 'week', 'month', 'custom'].includes(stats.timeControl.presetRange)
                          ? 'default'
                          : 'ghost'
                      }
                      size="sm"
                      className="h-9 px-3"
                    >
                      更多
                      <ChevronDown className="ml-1 h-4 w-4" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="start">
                    <DropdownMenuItem
                      onClick={() => stats.timeControl.setPresetRange('fifteen_minutes')}
                    >
                      {PRESET_LABELS['fifteen_minutes']}
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      onClick={() => stats.timeControl.setPresetRange('thirty_minutes')}
                    >
                      {PRESET_LABELS['thirty_minutes']}
                    </DropdownMenuItem>
                    <DropdownMenuItem onClick={() => stats.timeControl.setPresetRange('hour')}>
                      {PRESET_LABELS['hour']}
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      onClick={() => stats.timeControl.setPresetRange('twelve_hours')}
                    >
                      {PRESET_LABELS['twelve_hours']}
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>

                <TabsTrigger value="custom">自定义</TabsTrigger>
              </TabsList>
            </Tabs>

            {/* 刷新按钮 */}
            <Button variant="ghost" size="sm" onClick={stats.refresh} disabled={stats.loading}>
              <Loader2 className={`h-4 w-4 ${stats.loading ? 'animate-spin' : ''}`} />
            </Button>
          </div>
        </div>

        {/* 粒度选择 */}
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-muted-foreground">粒度</span>
          <Tabs
            value={stats.timeControl.granularity}
            onValueChange={(val) => stats.timeControl.setGranularity(val as TimeGranularity)}
          >
            <TabsList>
              {stats.timeControl.allowedGranularities.map((g) => (
                <TabsTrigger key={g} value={g}>
                  {GRANULARITY_LABELS[g]}
                </TabsTrigger>
              ))}
            </TabsList>
          </Tabs>
        </div>
      </div>

      {/* Dashboard：4个统计卡片 */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          title="总成本"
          value={`$${stats.summary.total_cost.toFixed(4)}`}
          icon={DollarSign}
          color="bg-green-500"
        />
        <StatCard
          title="总请求次数"
          value={stats.summary.total_requests.toLocaleString('zh-CN')}
          icon={Activity}
          color="bg-blue-500"
          suffix="次"
        />
        <StatCard
          title="平均响应时间"
          value={
            stats.summary.avg_response_time
              ? (stats.summary.avg_response_time / 1000).toFixed(2)
              : 'N/A'
          }
          icon={Clock}
          color="bg-indigo-500"
          suffix={stats.summary.avg_response_time ? 's' : ''}
        />
        <StatCard
          title="失败率"
          value={
            stats.summary.total_requests > 0
              ? ((stats.summary.failed_requests / stats.summary.total_requests) * 100).toFixed(1)
              : '0.0'
          }
          icon={AlertTriangle}
          color={
            stats.summary.total_requests > 0 &&
            stats.summary.failed_requests / stats.summary.total_requests > 0.05
              ? 'bg-red-500'
              : 'bg-orange-500'
          }
          suffix="%"
        />
      </div>

      {/* 成本趋势图 */}
      <TrendsChart data={stats.trends} title="成本趋势" dataKeys={costDataKeys} height={300} />

      {/* Token 使用趋势图 */}
      <TrendsChart
        data={stats.trends}
        title="Token 使用趋势"
        dataKeys={tokenDataKeys}
        height={300}
      />

      {/* 响应时间趋势图 */}
      <TrendsChart
        data={stats.trends}
        title="响应时间趋势"
        dataKeys={responseTimeDataKeys}
        height={300}
      />

      {/* Token 分布卡片 */}
      {stats.totalTokens > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Token 分布详情</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <div className="w-3 h-3 rounded-full bg-blue-500" />
                  <span className="text-sm text-muted-foreground">输入 Token</span>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium">
                    {formatTokens(stats.tokenBreakdown.input)}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    ({((stats.tokenBreakdown.input / stats.totalTokens) * 100).toFixed(1)}%)
                  </span>
                </div>
              </div>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <div className="w-3 h-3 rounded-full bg-green-500" />
                  <span className="text-sm text-muted-foreground">输出 Token</span>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium">
                    {formatTokens(stats.tokenBreakdown.output)}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    ({((stats.tokenBreakdown.output / stats.totalTokens) * 100).toFixed(1)}%)
                  </span>
                </div>
              </div>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <div className="w-3 h-3 rounded-full bg-yellow-500" />
                  <span className="text-sm text-muted-foreground">缓存创建 Token</span>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium">
                    {formatTokens(stats.tokenBreakdown.cacheCreation)}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    ({((stats.tokenBreakdown.cacheCreation / stats.totalTokens) * 100).toFixed(1)}
                    %)
                  </span>
                </div>
              </div>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <div className="w-3 h-3 rounded-full bg-purple-500" />
                  <span className="text-sm text-muted-foreground">缓存读取 Token</span>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium">
                    {formatTokens(stats.tokenBreakdown.cacheRead)}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    ({((stats.tokenBreakdown.cacheRead / stats.totalTokens) * 100).toFixed(1)}%)
                  </span>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* 请求统计卡片 */}
      <Card>
        <CardHeader>
          <CardTitle>请求概览</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">总请求数</span>
              <span className="text-sm font-medium">{stats.summary.total_requests} 次</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">成功请求</span>
              <span className="text-sm font-medium text-green-600">
                {stats.summary.successful_requests} 次 (
                {stats.summary.total_requests > 0
                  ? (
                      (stats.summary.successful_requests / stats.summary.total_requests) *
                      100
                    ).toFixed(1)
                  : '0.0'}
                %)
              </span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">失败请求</span>
              <span className="text-sm font-medium text-red-600">
                {stats.summary.failed_requests} 次 (
                {stats.summary.total_requests > 0
                  ? ((stats.summary.failed_requests / stats.summary.total_requests) * 100).toFixed(
                      1,
                    )
                  : '0.0'}
                %)
              </span>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* 提示信息 */}
      <p className="text-xs text-muted-foreground text-center">
        {stats.timeControl.mode === 'preset'
          ? `统计范围：${PRESET_LABELS[stats.timeControl.presetRange]}`
          : `统计范围：${new Date(stats.timeControl.startTimeMs).toLocaleString('zh-CN')} ~ ${new Date(stats.timeControl.endTimeMs).toLocaleString('zh-CN')}`}
        ，粒度：{GRANULARITY_LABELS[stats.timeControl.granularity]}
      </p>

      {/* 自定义时间范围对话框 */}
      <CustomTimeRangeDialog
        open={stats.timeControl.showCustomDialog}
        onOpenChange={stats.timeControl.closeCustomDialog}
        startTime={stats.timeControl.customStartTime}
        endTime={stats.timeControl.customEndTime}
        onStartTimeChange={stats.timeControl.setCustomStartTime}
        onEndTimeChange={stats.timeControl.setCustomEndTime}
        onConfirm={stats.timeControl.confirmCustomTime}
      />
    </div>
  );
}
