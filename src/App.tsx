import { useState, useEffect, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import { AppSidebar } from '@/components/layout/AppSidebar';
import { CloseActionDialog } from '@/components/dialogs/CloseActionDialog';
import { UpdateDialog } from '@/components/dialogs/UpdateDialog';
import { StatisticsPage } from '@/pages/StatisticsPage';
import { BalancePage } from '@/pages/BalancePage';
import { InstallationPage } from '@/pages/InstallationPage';
import { DashboardPage } from '@/pages/DashboardPage';
import { ConfigurationPage } from '@/pages/ConfigurationPage';
import { ProfileSwitchPage } from '@/pages/ProfileSwitchPage';
import { SettingsPage } from '@/pages/SettingsPage';
import { TransparentProxyPage } from '@/pages/TransparentProxyPage';
import { useToast } from '@/hooks/use-toast';
import { useAppEvents } from '@/hooks/useAppEvents';
import { useCloseAction } from '@/hooks/useCloseAction';
import { Toaster } from '@/components/ui/toaster';
import {
  checkInstallations,
  checkForAppUpdates,
  getGlobalConfig,
  getUserQuota,
  getUsageStats,
  type CloseAction,
  type ToolStatus,
  type GlobalConfig,
  type UserQuotaResult,
  type UsageStatsResult,
  type UpdateInfo,
} from '@/lib/tauri-commands';

type TabType =
  | 'dashboard'
  | 'install'
  | 'config'
  | 'switch'
  | 'statistics'
  | 'balance'
  | 'transparent-proxy'
  | 'settings';

function App() {
  const { toast } = useToast();
  const [activeTab, setActiveTab] = useState<TabType>('dashboard');
  const [selectedProxyToolId, setSelectedProxyToolId] = useState<string | undefined>(undefined);

  // 全局工具状态缓存
  const [tools, setTools] = useState<ToolStatus[]>([]);
  const [toolsLoading, setToolsLoading] = useState(true);

  // 全局配置缓存（供 StatisticsPage 和 SettingsPage 共享）
  const [globalConfig, setGlobalConfig] = useState<GlobalConfig | null>(null);
  const [configLoading, setConfigLoading] = useState(false);

  // 统计数据缓存
  const [usageStats, setUsageStats] = useState<UsageStatsResult | null>(null);
  const [userQuota, setUserQuota] = useState<UserQuotaResult | null>(null);
  const [statsLoading, setStatsLoading] = useState(false);
  const [statsLoadFailed, setStatsLoadFailed] = useState(false); // 新增：记录加载失败状态
  const [statsError, setStatsError] = useState<string | null>(null);

  // 更新检查状态
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [updateCheckDone, setUpdateCheckDone] = useState(false);
  const [isUpdateDialogOpen, setIsUpdateDialogOpen] = useState(false);

  // 加载工具状态（全局缓存）
  const loadTools = useCallback(async () => {
    try {
      setToolsLoading(true);
      const result = await checkInstallations();
      setTools(result);
    } catch (error) {
      console.error('Failed to load tools:', error);
    } finally {
      setToolsLoading(false);
    }
  }, []);

  // 加载全局配置（供多处使用）
  const loadGlobalConfig = useCallback(async () => {
    try {
      setConfigLoading(true);
      const config = await getGlobalConfig();
      setGlobalConfig(config);
    } catch (error) {
      console.error('Failed to load global config:', error);
    } finally {
      setConfigLoading(false);
    }
  }, []);

  // 加载统计数据（仅在需要时调用）
  const loadStatistics = useCallback(async () => {
    if (!globalConfig?.user_id || !globalConfig?.system_token) {
      return;
    }

    try {
      setStatsLoading(true);
      setStatsLoadFailed(false);
      setStatsError(null);
      const [quota, stats] = await Promise.all([getUserQuota(), getUsageStats()]);
      setUserQuota(quota);
      setUsageStats(stats);
      setStatsLoadFailed(false);
    } catch (error) {
      console.error('Failed to load statistics:', error);
      setStatsLoadFailed(true);
      const message = error instanceof Error ? error.message : '请检查网络连接后重试';
      setStatsError(message);

      toast({
        title: '加载统计数据失败',
        description: message,
        variant: 'destructive',
        duration: 5000,
      });
    } finally {
      setStatsLoading(false);
    }
  }, [globalConfig?.user_id, globalConfig?.system_token, toast]);

  // 检查应用更新
  const checkAppUpdates = useCallback(
    async (force = false) => {
      // 避免重复检查，除非强制检查
      if (updateCheckDone && !force) {
        return;
      }

      try {
        console.log('Checking for app updates...');
        const update = await checkForAppUpdates();
        setUpdateInfo(update);

        // 如果有可用更新，直接打开更新弹窗
        if (update.has_update) {
          setIsUpdateDialogOpen(true);
        }
      } catch (error) {
        console.error('Failed to check for updates:', error);
        // 静默失败，不显示错误通知给用户
      } finally {
        setUpdateCheckDone(true);
      }
    },
    [updateCheckDone],
  );

  // 初始化加载工具和全局配置
  useEffect(() => {
    loadTools();
    loadGlobalConfig();
  }, [loadTools, loadGlobalConfig]);

  // 应用启动时检查更新（延迟1秒，避免影响启动速度）
  useEffect(() => {
    const timer = setTimeout(() => {
      checkAppUpdates();
    }, 1000); // 1秒后检查更新

    return () => clearTimeout(timer);
  }, [checkAppUpdates]);

  // 当凭证变更时，重置统计状态以便重新加载
  useEffect(() => {
    setStatsLoadFailed(false);
    setStatsError(null);
  }, [globalConfig?.user_id, globalConfig?.system_token]);

  // 监听后端推送的更新事件
  useEffect(() => {
    // 监听后端主动推送的更新可用事件
    const unlistenUpdateAvailable = listen<UpdateInfo>('update-available', (event) => {
      const updateInfo = event.payload;
      setUpdateInfo(updateInfo);

      // 直接打开更新弹窗
      setIsUpdateDialogOpen(true);
    });

    // 监听托盘菜单触发的检查更新请求
    const unlistenRequestCheck = listen('request-check-update', () => {
      // 清空旧的更新信息，打开弹窗，触发重新检查
      setUpdateInfo(null);
      setIsUpdateDialogOpen(true);
    });

    // 监听未发现更新事件（用于托盘触发后的反馈）
    const unlistenNotFound = listen('update-not-found', () => {
      toast({
        title: '已是最新版本',
        description: '当前无可用更新',
      });
    });

    return () => {
      unlistenUpdateAvailable.then((fn) => fn());
      unlistenRequestCheck.then((fn) => fn());
      unlistenNotFound.then((fn) => fn());
    };
  }, [toast]);

  // 智能预加载：只要有凭证就立即预加载统计数据
  useEffect(() => {
    // 条件：配置已加载 + 有凭证 + 还没有统计数据 + 不在加载中 + 没有失败过
    if (
      globalConfig?.user_id &&
      globalConfig?.system_token &&
      !usageStats &&
      !statsLoading &&
      !statsLoadFailed
    ) {
      loadStatistics();
    }
  }, [
    globalConfig?.user_id,
    globalConfig?.system_token,
    usageStats,
    statsLoading,
    statsLoadFailed,
    loadStatistics,
  ]);

  // 使用关闭动作 Hook
  const {
    closeDialogOpen,
    rememberCloseChoice,
    closeActionLoading,
    setRememberCloseChoice,
    executeCloseAction,
    openCloseDialog,
    closeDialog,
  } = useCloseAction((message: string) => {
    toast({
      variant: 'destructive',
      title: '窗口操作失败',
      description: message,
    });
  });

  // 使用应用事件 Hook
  useAppEvents({
    onCloseRequest: openCloseDialog,
    onSingleInstance: (message: string) => {
      toast({
        title: 'DuckCoding 已在运行',
        description: message,
      });
    },
    onNavigateToConfig: (detail) => {
      setActiveTab('config');
      console.log('Navigate to config:', detail);
    },
    onNavigateToInstall: () => setActiveTab('install'),
    onNavigateToSettings: () => setActiveTab('settings'),
    onNavigateToTransparentProxy: (detail) => {
      setActiveTab('transparent-proxy');
      if (detail?.toolId) {
        setSelectedProxyToolId(detail.toolId);
      }
    },
    onRefreshTools: loadTools,
    executeCloseAction,
  });

  return (
    <div className="flex h-screen bg-gradient-to-br from-slate-50 to-slate-100 dark:from-slate-900 dark:to-slate-800">
      {/* 侧边栏 */}
      <AppSidebar activeTab={activeTab} onTabChange={(tab) => setActiveTab(tab as TabType)} />

      {/* 主内容区域 */}
      <main className="flex-1 overflow-auto">
        {activeTab === 'dashboard' && <DashboardPage tools={tools} loading={toolsLoading} />}
        {activeTab === 'install' && <InstallationPage tools={tools} loading={toolsLoading} />}
        {activeTab === 'config' && <ConfigurationPage tools={tools} loading={toolsLoading} />}
        {activeTab === 'switch' && <ProfileSwitchPage tools={tools} loading={toolsLoading} />}
        {activeTab === 'statistics' && (
          <StatisticsPage
            globalConfig={globalConfig}
            usageStats={usageStats}
            userQuota={userQuota}
            statsLoading={statsLoading}
            statsLoadFailed={statsLoadFailed}
            statsError={statsError}
            onLoadStatistics={loadStatistics}
          />
        )}
        {activeTab === 'balance' && <BalancePage />}
        {activeTab === 'transparent-proxy' && (
          <TransparentProxyPage selectedToolId={selectedProxyToolId} />
        )}
        {activeTab === 'settings' && (
          <SettingsPage
            globalConfig={globalConfig}
            configLoading={configLoading}
            onConfigChange={loadGlobalConfig}
            updateInfo={updateInfo}
            onUpdateCheck={() => {
              // 清空旧的更新信息，打开弹窗，触发重新检查
              setUpdateInfo(null);
              setIsUpdateDialogOpen(true);
            }}
          />
        )}
      </main>

      {/* 更新对话框 */}
      <UpdateDialog
        open={isUpdateDialogOpen}
        onOpenChange={setIsUpdateDialogOpen}
        updateInfo={updateInfo}
        onCheckForUpdate={() => {
          // 清空旧信息，触发重新检查
          setUpdateInfo(null);
          checkAppUpdates(true);
        }}
      />

      {/* 关闭动作选择对话框 */}
      <CloseActionDialog
        open={closeDialogOpen}
        closeActionLoading={closeActionLoading}
        rememberCloseChoice={rememberCloseChoice}
        onClose={closeDialog}
        onRememberChange={setRememberCloseChoice}
        onExecuteAction={(action: CloseAction, remember: boolean) =>
          executeCloseAction(action, remember, false)
        }
      />

      {/* Toast 通知 */}
      <Toaster />
    </div>
  );
}

export default App;
