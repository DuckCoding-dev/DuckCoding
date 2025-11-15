import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { BarChart3, Settings as SettingsIcon, RefreshCw, Loader2 } from 'lucide-react';
import { PageContainer } from '@/components/layout/PageContainer';
import { QuotaCard } from '@/components/QuotaCard';
import { TodayStatsCard } from '@/components/TodayStatsCard';
import { UsageChart } from '@/components/UsageChart';
import type { GlobalConfig, UserQuotaResult, UsageStatsResult } from '@/lib/tauri-commands';

interface StatisticsPageProps {
  globalConfig: GlobalConfig | null;
  usageStats: UsageStatsResult | null;
  userQuota: UserQuotaResult | null;
  statsLoading: boolean;
  onLoadStatistics: () => void;
}

export function StatisticsPage({
  globalConfig,
  usageStats,
  userQuota,
  statsLoading,
  onLoadStatistics,
}: StatisticsPageProps) {
  const hasCredentials = globalConfig?.user_id && globalConfig?.system_token;

  return (
    <PageContainer>
      <div className="mb-6">
        <h2 className="text-2xl font-semibold mb-1">用量统计</h2>
        <p className="text-sm text-muted-foreground">查看您的 DuckCoding API 使用情况和消费记录</p>
      </div>

      {!hasCredentials ? (
        <Card className="shadow-sm border">
          <CardContent className="pt-6">
            <div className="text-center py-12">
              <BarChart3 className="h-16 w-16 mx-auto mb-4 text-muted-foreground opacity-30" />
              <h3 className="text-lg font-semibold mb-2">需要配置凭证</h3>
              <p className="text-sm text-muted-foreground mb-4">
                请先在全局设置中配置您的用户ID和系统访问令牌
              </p>
              <Button
                onClick={() => {
                  // 切换到设置页面的逻辑将在父组件中处理
                  // 这里可以通过回调或全局状态管理来实现
                  window.dispatchEvent(new CustomEvent('navigate-to-settings'));
                }}
                className="shadow-md hover:shadow-lg transition-all"
              >
                <SettingsIcon className="mr-2 h-4 w-4" />
                前往设置
              </Button>
            </div>
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-6">
          <div className="flex items-center justify-end">
            <Button
              variant="outline"
              size="sm"
              onClick={onLoadStatistics}
              disabled={statsLoading}
              className="shadow-sm hover:shadow-md transition-all"
            >
              {statsLoading ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  加载中...
                </>
              ) : (
                <>
                  <RefreshCw className="mr-2 h-4 w-4" />
                  刷新数据
                </>
              )}
            </Button>
          </div>

          {/* 顶部卡片网格 - 2列 */}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <QuotaCard quota={userQuota} loading={statsLoading} />
            <TodayStatsCard stats={usageStats} loading={statsLoading} />
          </div>

          {/* 用量趋势图 - 全宽 */}
          <UsageChart stats={usageStats} loading={statsLoading} />
        </div>
      )}
    </PageContainer>
  );
}
