import { useState, useEffect, useCallback } from 'react';
import { Button } from '@/components/ui/button';
import { RefreshCw, Loader2, Search } from 'lucide-react';
import { PageContainer } from '@/components/layout/PageContainer';
import { DashboardToolCard } from './components/DashboardToolCard';
import { UpdateCheckBanner } from './components/UpdateCheckBanner';
import { ProviderTabs } from './components/ProviderTabs';
import { useDashboard } from './hooks/useDashboard';
import { useDashboardProviders } from './hooks/useDashboardProviders';
import { getToolDisplayName } from '@/utils/constants';
import { useToast } from '@/hooks/use-toast';
import {
  getUserQuota,
  getUsageStats,
  type ToolStatus,
  refreshAllToolVersions,
  getSelectedProviderId,
  setSelectedProviderId as saveSelectedProviderId,
} from '@/lib/tauri-commands';
import type { UserQuotaResult, UsageStatsResult } from '@/lib/tauri-commands/types';

interface DashboardPageProps {
  tools: ToolStatus[];
  loading: boolean;
}

export function DashboardPage({ tools: toolsProp, loading: loadingProp }: DashboardPageProps) {
  const { toast } = useToast();
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
      // 更新成功后重新检测工具状态（而不是仅读数据库）
      await handleRefreshToolStatus();
      // 更新成功后自动检测工具更新状态，显示「最新版」标识
      await checkSingleToolUpdate(toolId);
    } else {
      toast({
        title: '更新失败',
        description: result.message,
        variant: 'destructive',
      });
    }
  };

  // 切换到配置页面
  const switchToConfig = (toolId?: string) => {
    window.dispatchEvent(new CustomEvent('navigate-to-config', { detail: { toolId } }));
  };

  // 切换到安装页面
  const switchToInstall = () => {
    window.dispatchEvent(new CustomEvent('navigate-to-install'));
  };

  // 切换到安装页面
  const switchToList = () => {
    window.dispatchEvent(new CustomEvent('navigate-to-list'));
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
    // 使用 Hook 提供的函数，直接保存 instance_id
    const result = await setInstanceSelection(toolId, instanceId);

    if (result.success) {
      // 获取实例的 label 用于提示
      const instances = getInstanceOptions(toolId);
      const selectedInstance = instances.find((opt) => opt.value === instanceId);

      toast({
        title: '实例已切换',
        description: `${getToolDisplayName(toolId)} 已切换到 ${selectedInstance?.label || instanceId}`,
      });

      // 切换成功后重新加载工具实例数据以刷新UI
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
    // 从后端数据中查找该工具
    const foundTool = tools.find((t) => t.id === toolId);
    // 如果找到则使用后端数据，否则创建占位数据
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

  return (
    <PageContainer>
      <div className="mb-6">
        <h2 className="text-2xl font-semibold mb-1">仪表板</h2>
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-20">
          <Loader2 className="h-8 w-8 animate-spin text-primary" />
          <span className="ml-3 text-muted-foreground">加载中...</span>
        </div>
      ) : (
        <>
          {/* 更新检查提示 */}
          {updateCheckMessage && <UpdateCheckBanner message={updateCheckMessage} />}

          <div className="space-y-6">
            {/* 第一段：工具卡片 + 操作按钮 */}
            <div>
              <div className="flex justify-between items-center mb-4">
                <h3 className="text-lg font-semibold">工具状态</h3>
                <div className="flex gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={handleRefreshToolStatus}
                    disabled={refreshing}
                    className="shadow-sm hover:shadow-md transition-all"
                  >
                    {refreshing ? (
                      <>
                        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                        检测中...
                      </>
                    ) : (
                      <>
                        <Search className="mr-2 h-4 w-4" />
                        检测工具状态
                      </>
                    )}
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={checkForUpdates}
                    disabled={checkingUpdates}
                    className="shadow-sm hover:shadow-md transition-all"
                  >
                    {checkingUpdates ? (
                      <>
                        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                        检查中...
                      </>
                    ) : (
                      <>
                        <RefreshCw className="mr-2 h-4 w-4" />
                        一键检查更新
                      </>
                    )}
                  </Button>
                </div>
              </div>

              {/* 工具卡片列表 */}
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

            {/* 第二段：供应商标签页 */}
            <div>
              <div className="flex justify-between items-center mb-3">
                <h3 className="text-lg font-semibold">供应商与用量统计</h3>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleRefreshProviderData}
                  disabled={quotaLoading || statsLoading}
                  className="shadow-sm hover:shadow-md transition-all"
                >
                  {quotaLoading || statsLoading ? (
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
