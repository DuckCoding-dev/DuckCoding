// Token 统计页面
// 整合实时统计和历史日志展示

import { useEffect, useState } from 'react';
import { emit } from '@tauri-apps/api/event';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { PageContainer } from '@/components/layout/PageContainer';
import { ArrowLeft, Database, RefreshCw, AlertCircle, Calendar } from 'lucide-react';
import { useToast } from '@/hooks/use-toast';
import { RealtimeStats } from '../TransparentProxyPage/components/RealtimeStats';
import { LogsTable } from '../TransparentProxyPage/components/LogsTable';
import { getTokenStatsSummary, getTokenStatsConfig } from '@/lib/tauri-commands';
import { queryTokenTrends, queryCostSummary } from '@/lib/tauri-commands/analytics';
import { Dashboard } from './components/Dashboard';
import { TrendsChart } from './components/TrendsChart';
import { CustomTimeRangeDialog } from '@/components/dialogs/CustomTimeRangeDialog';
import { useTimeRangeControl } from '@/hooks/useTimeRangeControl';
import { GRANULARITY_LABELS } from '@/utils/time-range';
import type { DatabaseSummary, TokenStatsConfig, ToolType } from '@/types/token-stats';
import type { TrendDataPoint, CostSummary, TimeRange, TimeGranularity } from '@/types/analytics';

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

  // 使用统一的时间范围控制 Hook
  const timeControl = useTimeRangeControl();

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

  /**
   * 填充缺失的时间点数据（用于响应时间趋势图）
   * 将 null 值的 avg_response_time 替换为 0，确保折线连续
   * @param data - 原始趋势数据
   * @returns 填充后的趋势数据
   */
  const fillMissingTimePoints = (data: TrendDataPoint[]): TrendDataPoint[] => {
    return data.map((point) => {
      // 如果 avg_response_time 为 null，替换为 0
      if (point.avg_response_time === null) {
        return {
          ...point,
          avg_response_time: 0,
        };
      }
      return point;
    });
  };

  // 数据库摘要
  const [summary, setSummary] = useState<DatabaseSummary | null>(null);
  const [config, setConfig] = useState<TokenStatsConfig | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  // 分析数据
  const [trendsData, setTrendsData] = useState<TrendDataPoint[]>([]);
  const [responseTimeTrends, setResponseTimeTrends] = useState<TrendDataPoint[]>([]); // 填充后的响应时间趋势数据
  const [costSummary, setCostSummary] = useState<CostSummary | null>(null);
  const [analyticsLoading, setAnalyticsLoading] = useState(false);

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

  // 加载分析数据
  useEffect(() => {
    const loadAnalyticsData = async () => {
      setAnalyticsLoading(true);
      try {
        const [trends, summary] = await Promise.all([
          queryTokenTrends({
            start_time: timeControl.startTimeMs,
            end_time: timeControl.endTimeMs,
            tool_type: toolType,
            granularity: timeControl.granularity,
          }),
          queryCostSummary(timeControl.startTimeMs, timeControl.endTimeMs, toolType),
        ]);

        // 原始数据用于成本和 Token 趋势图
        setTrendsData(trends);

        // 为响应时间趋势图填充缺失的时间点（将 null 替换为 0）
        const filledTrends = fillMissingTimePoints(trends);
        setResponseTimeTrends(filledTrends);

        setCostSummary(summary);
      } catch (error) {
        console.error('Failed to load analytics data:', error);
        toast({
          title: '加载失败',
          description: '无法加载分析数据',
          variant: 'destructive',
        });
      } finally {
        setAnalyticsLoading(false);
      }
    };

    loadAnalyticsData();
  }, [timeControl.startTimeMs, timeControl.endTimeMs, timeControl.granularity, toolType, toast]);

  // 刷新数据
  const handleRefresh = async () => {
    try {
      const [summaryData, configData] = await Promise.all([
        getTokenStatsSummary(),
        getTokenStatsConfig(),
      ]);
      setSummary(summaryData);
      setConfig(configData);
      setRefreshKey((prev) => prev + 1);
      toast({
        title: '刷新成功',
        description: '数据已更新',
      });
    } catch (error) {
      console.error('刷新数据失败:', error);
      toast({
        title: '刷新失败',
        description: String(error),
        variant: 'destructive',
      });
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

  // 格式化查询时间范围显示（所有模式都显示实际查询范围）
  const formatQueryTimeRange = () => {
    return `${formatDate(timeControl.startTimeMs)} - ${formatDate(timeControl.endTimeMs)}`;
  };

  // 处理时间范围选择
  const handleTimeRangeChange = (value: string) => {
    if (value === 'custom') {
      timeControl.openCustomDialog();
    } else {
      timeControl.setPresetRange(value as Exclude<TimeRange, 'custom'>);
    }
  };

  const pageActions = (
    <div className="flex items-center gap-2">
      {/* 数据库信息 */}
      {summary && (
        <div className="hidden xl:flex items-center gap-4 px-4 py-2 rounded-md bg-muted/50 text-sm">
          <div className="flex items-center gap-2">
            <Database className="h-4 w-4 text-muted-foreground" />
            <span className="text-muted-foreground">总记录:</span>
            <span className="font-medium">{summary.total_logs.toLocaleString('zh-CN')}</span>
          </div>
          <div className="h-4 w-px bg-border" />
          <div className="text-muted-foreground">
            <span>{formatQueryTimeRange()}</span>
          </div>
        </div>
      )}

      {/* 时间范围选择器 */}
      <Select
        value={timeControl.mode === 'custom' ? 'custom' : timeControl.presetRange}
        onValueChange={handleTimeRangeChange}
      >
        <SelectTrigger className="w-36">
          <Calendar className="h-4 w-4 mr-2" />
          <SelectValue placeholder="查询范围" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="fifteen_minutes">最近15分钟</SelectItem>
          <SelectItem value="thirty_minutes">最近30分钟</SelectItem>
          <SelectItem value="hour">最近1小时</SelectItem>
          <SelectItem value="twelve_hours">最近12小时</SelectItem>
          <SelectItem value="day">最近1天</SelectItem>
          <SelectItem value="week">最近7天</SelectItem>
          <SelectItem value="month">最近30天</SelectItem>
          <SelectItem value="custom">自定义</SelectItem>
        </SelectContent>
      </Select>

      {/* 时间粒度选择器 */}
      <Select
        value={timeControl.granularity}
        onValueChange={(value) => timeControl.setGranularity(value as TimeGranularity)}
      >
        <SelectTrigger className="w-32">
          <SelectValue placeholder="数据粒度" />
        </SelectTrigger>
        <SelectContent>
          {timeControl.allowedGranularities.map((g) => (
            <SelectItem key={g} value={g}>
              {GRANULARITY_LABELS[g]}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>

      {/* 刷新按钮 */}
      <Button variant="outline" size="sm" onClick={handleRefresh}>
        <RefreshCw className="h-4 w-4" />
        刷新
      </Button>

      {/* 返回按钮 */}
      <Button variant="ghost" size="sm" onClick={handleGoBack}>
          <ArrowLeft className="h-4 w-4" />
          返回
      </Button>
    </div>
  );

  return (
    <div className="space-y-6">
      <PageContainer 
        title="Token 统计" 
        description="查看透明代理的 Token 使用情况和请求历史"
        actions={pageActions}
      >
        {/* 实时统计（如果提供了 sessionId 和 toolType） */}
        {sessionId && toolType && <RealtimeStats sessionId={sessionId} toolType={toolType} />}

        {/* 仪表盘 - 关键指标 */}
        {costSummary && <Dashboard summary={costSummary} loading={analyticsLoading} />}

        {/* 趋势图表 */}
        {trendsData.length > 0 && (
          <>
            {/* 成本趋势 */}
            <TrendsChart
              data={trendsData}
              title="成本趋势"
              dataKeys={[
                {
                  key: 'total_cost',
                  name: '总成本',
                  color: '#10b981',
                  formatter: (value) => `$${value.toFixed(4)}`,
                },
                {
                  key: 'input_price',
                  name: '输入成本',
                  color: '#3b82f6',
                  formatter: (value) => `$${value.toFixed(4)}`,
                },
                {
                  key: 'output_price',
                  name: '输出成本',
                  color: '#f59e0b',
                  formatter: (value) => `$${value.toFixed(4)}`,
                },
              ]}
              yAxisLabel="成本 (USD)"
              height={300}
            />

            {/* Token 使用趋势 */}
            <TrendsChart
              data={trendsData}
              title="Token 使用趋势"
              dataKeys={[
                {
                  key: 'input_tokens',
                  name: '输入 Tokens',
                  color: '#3b82f6',
                  formatter: (value) => value.toLocaleString(),
                },
                {
                  key: 'output_tokens',
                  name: '输出 Tokens',
                  color: '#f59e0b',
                  formatter: (value) => value.toLocaleString(),
                },
                {
                  key: 'cache_read_tokens',
                  name: '缓存读取 Tokens',
                  color: '#8b5cf6',
                  formatter: (value) => value.toLocaleString(),
                },
              ]}
              yAxisLabel="Token 数量"
              height={300}
            />

            {/* 响应时间趋势 */}
            <TrendsChart
              data={responseTimeTrends}
              title="平均响应时间趋势"
              dataKeys={[
                {
                  key: 'avg_response_time',
                  name: '平均响应时间',
                  color: '#8b5cf6',
                  formatter: (value) =>
                    value >= 1000 ? `${(value / 1000).toFixed(2)}s` : `${Math.round(value)}ms`,
                },
              ]}
              yAxisLabel="响应时间 (ms)"
              height={300}
            />
          </>
        )}

        {/* 历史日志表格 */}
        <LogsTable key={refreshKey} initialToolType={toolType} initialSessionId={sessionId} />

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
      </PageContainer>

      {/* 自定义时间范围对话框 */}
      <CustomTimeRangeDialog
        open={timeControl.showCustomDialog}
        onOpenChange={timeControl.closeCustomDialog}
        startTime={timeControl.customStartTime}
        endTime={timeControl.customEndTime}
        onStartTimeChange={timeControl.setCustomStartTime}
        onEndTimeChange={timeControl.setCustomEndTime}
        onConfirm={timeControl.confirmCustomTime}
      />
    </div>
  );
}
