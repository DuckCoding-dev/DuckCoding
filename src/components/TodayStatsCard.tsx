import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Calendar, TrendingUp } from 'lucide-react';
import { useMemo, useState } from 'react';
import { format } from 'date-fns';
import { zhCN } from 'date-fns/locale';
import type { UsageStatsResult } from '@/lib/tauri-commands';
import { TrendChartDialog } from './TrendChartDialog';

interface TodayStatsCardProps {
  stats: UsageStatsResult | null;
  loading: boolean;
}

export function TodayStatsCard({ stats, loading }: TodayStatsCardProps) {
  const [showTrendDialog, setShowTrendDialog] = useState(false);

  // 计算今日数据
  const todayStats = useMemo(() => {
    if (!stats?.data || stats.data.length === 0) {
      return { requests: 0, quota: 0 };
    }

    // 获取今日开始时间戳（北京时间）
    const now = new Date();
    const todayStart = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    const todayStartTimestamp = Math.floor(todayStart.getTime() / 1000);

    // 过滤今日数据
    const todayData = stats.data.filter((item) => item.created_at >= todayStartTimestamp);

    // 聚合今日数据
    return {
      requests: todayData.reduce((sum, item) => sum + item.count, 0),
      quota: todayData.reduce((sum, item) => sum + item.quota, 0) / 500000,
    };
  }, [stats?.data]);

  const formatQuota = (value: number): string => {
    if (value < 0.01) {
      return `¥${value.toFixed(6)}`;
    } else if (value < 1) {
      return `¥${value.toFixed(4)}`;
    } else {
      return `¥${value.toFixed(2)}`;
    }
  };

  if (loading) {
    return (
      <Card className="shadow-sm border">
        <CardHeader className="pb-3">
          <CardTitle className="text-base font-semibold flex items-center gap-2">
            <Calendar className="h-4 w-4" />
            今日用量
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            <div className="h-6 bg-slate-200 dark:bg-slate-700 rounded animate-pulse"></div>
            <div className="h-6 bg-slate-200 dark:bg-slate-700 rounded animate-pulse"></div>
          </div>
        </CardContent>
      </Card>
    );
  }

  if (!stats || !stats.success) {
    return (
      <Card className="shadow-sm border">
        <CardHeader className="pb-3">
          <CardTitle className="text-base font-semibold flex items-center gap-2">
            <Calendar className="h-4 w-4" />
            今日用量
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-center py-8 text-sm text-muted-foreground">
            <Calendar className="h-8 w-8 mx-auto mb-2 opacity-30" />
            <p>暂无数据</p>
          </div>
        </CardContent>
      </Card>
    );
  }

  const today = format(new Date(), 'yyyy年MM月dd日', { locale: zhCN });

  return (
    <>
      <Card className="shadow-sm border">
        <CardHeader className="pb-3">
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="text-base font-semibold flex items-center gap-2">
                <Calendar className="h-4 w-4" />
                今日用量
              </CardTitle>
              <p className="text-xs text-muted-foreground mt-1">{today}</p>
            </div>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setShowTrendDialog(true)}
              className="gap-1.5"
            >
              <TrendingUp className="h-4 w-4" />
              查看趋势
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">请求次数</span>
              <span className="text-2xl font-semibold text-blue-600">
                {todayStats.requests.toLocaleString()}
              </span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">消费额度</span>
              <span className="text-2xl font-semibold text-green-600">
                {formatQuota(todayStats.quota)}
              </span>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* 趋势图表弹窗 */}
      <TrendChartDialog
        open={showTrendDialog}
        onOpenChange={setShowTrendDialog}
        stats={stats}
        loading={loading}
      />
    </>
  );
}
