import { useState, useEffect, useCallback } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { RefreshCw, Loader2, Search, Zap, Activity, CheckCircle2, AlertCircle } from 'lucide-react';
import { PageContainer } from '@/components/layout/PageContainer';
import { DashboardToolCard } from './components/DashboardToolCard';
import { UpdateCheckBanner } from './components/UpdateCheckBanner';
import { ProviderTabs } from './components/ProviderTabs';
import { useDashboard } from './hooks/useDashboard';
import { useDashboardProviders } from './hooks/useDashboardProviders';
import { getToolDisplayName } from '@/utils/constants';
import { useToast } from '@/hooks/use-toast';
import { useAppContext } from '@/hooks/useAppContext';
import {
  getUserQuota,
  getUsageStats,
  refreshAllToolVersions,
  getSelectedProviderId,
  setSelectedProviderId as saveSelectedProviderId,
} from '@/lib/tauri-commands';
import type { UserQuotaResult, UsageStatsResult } from '@/lib/tauri-commands/types';

export function DashboardPage() {
  const { toast } = useToast();
  const { tools: toolsProp, toolsLoading: loadingProp, setActiveTab } = useAppContext();

  const [loading, setLoading] = useState(loadingProp);
  const [refreshing, setRefreshing] = useState(false);
  const [quota, setQuota] = useState<UserQuotaResult | null>(null);
  const [quotaLoading, setQuotaLoading] = useState(false);
  const [stats, setStats] = useState<UsageStatsResult | null>(null);
  const [statsLoading, setStatsLoading] = useState(false);

  // 使用仪表板 Hook
  const {
    tools,
    updating,
    checkingUpdates,
    checkingSingleTool,
    updateCheckMessage,
    checkForUpdates,
    checkSingleToolUpdate,
    handleUpdate,
    updateTools,
  } = useDashboard(toolsProp);

  // 使用供应商管理 Hook
  const {
    providers,
    loading: providersLoading,
    instanceSelections,
    setInstanceSelection,
    getInstanceOptions,
    loadToolInstances,
    toolInstances,
  } = useDashboardProviders();

  // 选中的供应商 ID（持久化到 dashboard.json）
  const [selectedProviderId, setSelectedProviderId] = useState<string | null>(null);
  const [providerIdLoaded, setProviderIdLoaded] = useState(false);

  // 初始化时从后端加载 selectedProviderId
  useEffect(() => {
    const loadSelectedProviderId = async () => {
      try {
        const savedProviderId = await getSelectedProviderId();
        if (savedProviderId) {
          setSelectedProviderId(savedProviderId);
        }
      } catch (error) {
        console.error('加载选中的供应商 ID 失败:', error);
      } finally {
        setProviderIdLoaded(true);
      }
    };
    loadSelectedProviderId();
  }, []);

  // 初始化时选中第一个供应商（如果后端没有保存的值）
  useEffect(() => {
    if (providerIdLoaded && providers.length > 0 && !selectedProviderId) {
      setSelectedProviderId(providers[0].id);
    }
  }, [providers, selectedProviderId, providerIdLoaded]);

  // 同步外部 tools 数据
  useEffect(() => {
    updateTools(toolsProp);
    setLoading(loadingProp);
  }, [toolsProp, loadingProp, updateTools]);

  // 加载用户配额
  const loadQuota = useCallback(async (providerId: string) => {
    setQuotaLoading(true);
    try {
      const quotaData = await getUserQuota(providerId);
      setQuota(quotaData);
    } catch (error) {
      console.error('加载用户配额失败:', error);
      setQuota(null); // 清空旧数据
    } finally {
      setQuotaLoading(false);
    }
  }, []);

  // 加载用量统计
  const loadStats = useCallback(async (providerId: string) => {
    setStatsLoading(true);
    try {
      const statsData = await getUsageStats(providerId);
      setStats(statsData);
    } catch (error) {
      console.error('加载用量统计失败:', error);
      setStats(null); // 清空旧数据
    } finally {
      setStatsLoading(false);
    }
  }, []);

  // 加载用户配额和用量统计
  useEffect(() => {
    if (selectedProviderId) {
      loadQuota(selectedProviderId);
      loadStats(selectedProviderId);
    }
  }, [selectedProviderId, loadQuota, loadStats]);

  // 手动刷新工具状态（刷新数据库版本号）
  const handleRefreshToolStatus = async () => {
    setRefreshing(true);
    try {
      const newTools = await refreshAllToolVersions();
      updateTools(newTools);
      toast({
        title: '刷新完成',
        description: '工具版本号已更新',
      });
    } catch (error) {
      toast({
        title: '刷新失败',
        description: String(error),
        variant: 'destructive',
      });
    } finally {
      setRefreshing(false);
    }
  };

  // 更新工具处理
  const onUpdate = async (toolId: string) => {
    const result = await handleUpdate(toolId);

    if (result.isUpdating) {
      toast({
        title: '请稍候',
        description: result.message,
        variant: 'destructive',
      });
      return;
    }

    if (result.success) {
      toast({
        title: '更新成功',
        description: `${getToolDisplayName(toolId)} ${result.message}`,
      });
    } else {
      toast({
        title: '更新失败',
        description: result.message,
        variant: 'destructive',
      });
    }
  };

  // 切换到配置页面
  const switchToConfig = (_toolId?: string) => {
    setActiveTab('profile-management');
  };

  // 切换到安装页面
  const switchToInstall = () => {
    setActiveTab('install');
  };

  // 切换到工具列表页面
  const switchToList = () => {
    setActiveTab('tool-management');
  };

  // 处理供应商切换（持久化到后端）
  const handleProviderChange = async (providerId: string) => {
    setSelectedProviderId(providerId);
    try {
      await saveSelectedProviderId(providerId);
    } catch (error) {
      console.error('保存选中的供应商 ID 失败:', error);
    }
  };

  // 刷新当前供应商的配额和统计数据
  const handleRefreshProviderData = () => {
    if (selectedProviderId) {
      loadQuota(selectedProviderId);
      loadStats(selectedProviderId);
    }
  };

  // 处理实例选择变更
  const handleInstanceChange = async (toolId: string, instanceId: string) => {
    const result = await setInstanceSelection(toolId, instanceId);

    if (result.success) {
      const instances = getInstanceOptions(toolId);
      const selectedInstance = instances.find((opt) => opt.value === instanceId);

      toast({
        title: '实例已切换',
        description: `${getToolDisplayName(toolId)} 已切换到 ${selectedInstance?.label || instanceId}`,
      });

      try {
        await loadToolInstances();
      } catch (error) {
        console.error('更新工具实例数据失败:', error);
      }
    } else {
      toast({
        title: '切换失败',
        description: result.error,
        variant: 'destructive',
      });
    }
  };

  // 固定显示的三个工具ID
  const FIXED_TOOL_IDS = ['claude-code', 'codex', 'gemini-cli'];

  // 确保始终显示这三个工具，不论是否安装
  const displayTools = FIXED_TOOL_IDS.map((toolId) => {
    const foundTool = tools.find((t) => t.id === toolId);
    return (
      foundTool || {
        id: toolId,
        name: getToolDisplayName(toolId),
        installed: false,
        version: null,
        hasUpdate: false,
        latestVersion: null,
        mirrorIsStale: false,
        mirrorVersion: null,
      }
    );
  });

  const installedCount = displayTools.filter((t) => t.installed).length;
  const updateCount = displayTools.filter((t) => t.hasUpdate).length;

  const pageActions = (
    <div className="flex gap-2">
      <Button variant="outline" size="sm" onClick={handleRefreshToolStatus} disabled={refreshing}>
        {refreshing ? (
          <Loader2 className="mr-2 h-4 w-4 animate-spin" />
        ) : (
          <Search className="mr-2 h-4 w-4" />
        )}
        检测状态
      </Button>
      <Button variant="outline" size="sm" onClick={checkForUpdates} disabled={checkingUpdates}>
        {checkingUpdates ? (
          <Loader2 className="mr-2 h-4 w-4 animate-spin" />
        ) : (
          <RefreshCw className="mr-2 h-4 w-4" />
        )}
        检查更新
      </Button>
    </div>
  );

  return (
    <PageContainer title="仪表板" description="概览系统状态与工具健康度" actions={pageActions}>
      {loading ? (
        <div className="flex items-center justify-center py-20">
          <Loader2 className="h-8 w-8 animate-spin text-primary" />
          <span className="ml-3 text-muted-foreground">加载中...</span>
        </div>
      ) : (
        <>
          {updateCheckMessage && <UpdateCheckBanner message={updateCheckMessage} />}

          {/* 状态概览卡片 */}
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">已安装工具</CardTitle>
                <Zap className="h-4 w-4 text-muted-foreground" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{installedCount}</div>
                <p className="text-xs text-muted-foreground">
                  共 {FIXED_TOOL_IDS.length} 个支持工具
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">可用更新</CardTitle>
                <AlertCircle
                  className={`h-4 w-4 ${updateCount > 0 ? 'text-amber-500' : 'text-muted-foreground'}`}
                />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{updateCount}</div>
                <p className="text-xs text-muted-foreground">建议及时更新以获取新特性</p>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">供应商状态</CardTitle>
                <Activity className="h-4 w-4 text-muted-foreground" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{quota ? '正常' : '-'}</div>
                <p className="text-xs text-muted-foreground">
                  {providers.length > 0 ? `已配置 ${providers.length} 个供应商` : '暂无供应商配置'}
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">系统状态</CardTitle>
                <CheckCircle2 className="h-4 w-4 text-green-500" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">运行中</div>
                <p className="text-xs text-muted-foreground">所有服务正常运行</p>
              </CardContent>
            </Card>
          </div>

          <div className="space-y-6">
            {/* 工具状态 */}
            <div className="space-y-3">
              <h3 className="text-lg font-semibold tracking-tight">工具管理</h3>
              <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                {displayTools.map((tool) => (
                  <DashboardToolCard
                    key={tool.id}
                    tool={tool}
                    updating={updating === tool.id}
                    checking={checkingSingleTool === tool.id}
                    checkingAll={checkingUpdates}
                    instanceSelection={instanceSelections[tool.id]}
                    instanceOptions={getInstanceOptions(tool.id)}
                    toolInstances={toolInstances[tool.id] || []}
                    onUpdate={() => onUpdate(tool.id)}
                    onCheckUpdates={() => checkSingleToolUpdate(tool.id)}
                    onConfigure={() => switchToConfig(tool.id)}
                    onInstanceChange={(instanceId) => handleInstanceChange(tool.id, instanceId)}
                    onInstall={switchToInstall}
                    onAdd={switchToList}
                  />
                ))}
              </div>
            </div>

            {/* 供应商与用量 */}
            <div className="space-y-3">
              <div className="flex justify-between items-center">
                <h3 className="text-lg font-semibold tracking-tight">供应商与用量统计</h3>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={handleRefreshProviderData}
                  disabled={quotaLoading || statsLoading}
                >
                  {quotaLoading || statsLoading ? (
                    <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" />
                  ) : (
                    <RefreshCw className="mr-2 h-3.5 w-3.5" />
                  )}
                  刷新数据
                </Button>
              </div>
              <ProviderTabs
                providers={providers}
                selectedProviderId={selectedProviderId}
                loading={providersLoading}
                quota={quota}
                quotaLoading={quotaLoading}
                stats={stats}
                statsLoading={statsLoading}
                onProviderChange={handleProviderChange}
              />
            </div>
          </div>
        </>
      )}
    </PageContainer>
  );
}
