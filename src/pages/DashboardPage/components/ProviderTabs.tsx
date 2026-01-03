// 供应商水平标签页组件
// 用于在 Dashboard 中切换不同供应商

import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Loader2, Building2, RefreshCw } from 'lucide-react';
import type { Provider } from '@/lib/tauri-commands';
import { QuotaCard } from '@/components/QuotaCard';
import { TodayStatsCard } from '@/components/TodayStatsCard';
import type { UserQuotaResult, UsageStatsResult } from '@/lib/tauri-commands/types';

interface ProviderTabsProps {
  providers: Provider[];
  selectedProviderId: string | null;
  loading: boolean;
  quota: UserQuotaResult | null;
  quotaLoading: boolean;
  stats: UsageStatsResult | null;
  statsLoading: boolean;
  onProviderChange: (providerId: string) => void;
  onRefresh?: () => void; // 新增刷新回调
}

export function ProviderTabs({
  providers,
  selectedProviderId,
  loading,
  quota,
  quotaLoading,
  stats,
  statsLoading,
  onProviderChange,
  onRefresh,
}: ProviderTabsProps) {
  if (loading) {
    return (
      <Card className="shadow-sm border">
        <CardContent className="flex items-center justify-center py-8">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          <span className="ml-2 text-sm text-muted-foreground">加载供应商...</span>
        </CardContent>
      </Card>
    );
  }

  if (providers.length === 0) {
    return (
      <Card className="shadow-sm border">
        <CardHeader>
          <div className="flex items-center gap-2 text-muted-foreground">
            <Building2 className="h-5 w-5" />
            <span className="text-sm">暂无供应商</span>
          </div>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            请前往「全局设置 → 供应商管理」添加供应商配置
          </p>
        </CardContent>
      </Card>
    );
  }

  const currentProviderId = selectedProviderId || providers[0]?.id;
  const isRefreshing = quotaLoading || statsLoading;

  return (
    <Tabs value={currentProviderId} onValueChange={onProviderChange} className="space-y-4">
      {/* 供应商标签列表和刷新按钮 */}
      <div className="flex items-center gap-4">
        <TabsList className="h-auto p-1 bg-muted/50">
          {providers.map((provider) => {
            const isSelected = provider.id === currentProviderId;
            return (
              <TabsTrigger
                key={provider.id}
                value={provider.id}
                className="data-[state=active]:bg-background"
              >
                <div className="flex items-center gap-2">
                  <span>{provider.name}</span>
                  {isSelected && (
                    <Badge variant="default" className="text-xs px-1.5 py-0">
                      当前
                    </Badge>
                  )}
                </div>
              </TabsTrigger>
            );
          })}
        </TabsList>

        {/* 刷新按钮 */}
        {onRefresh && (
          <Button
            variant="outline"
            size="sm"
            onClick={onRefresh}
            disabled={isRefreshing}
            className="shadow-sm hover:shadow-md transition-all"
          >
            {isRefreshing ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                刷新中...
              </>
            ) : (
              <>
                <RefreshCw className="mr-2 h-4 w-4" />
                刷新
              </>
            )}
          </Button>
        )}
      </div>

      {/* 供应商内容 */}
      {providers.map((provider) => (
        <TabsContent key={provider.id} value={provider.id} className="mt-0 space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            {/* 额度卡片 */}
            <QuotaCard quota={quota} loading={quotaLoading} />

            {/* 今日统计卡片 */}
            <TodayStatsCard stats={stats} loading={statsLoading} />
          </div>
        </TabsContent>
      ))}
    </Tabs>
  );
}
