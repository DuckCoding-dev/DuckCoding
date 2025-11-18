import { useState, useEffect, useCallback } from 'react';
import { AppSidebar } from '@/components/layout/AppSidebar';
import { CloseActionDialog } from '@/components/dialogs/CloseActionDialog';
import { StatisticsPage } from '@/pages/StatisticsPage';
import { InstallationPage } from '@/pages/InstallationPage';
import { DashboardPage } from '@/pages/DashboardPage';
import { ConfigurationPage } from '@/pages/ConfigurationPage';
import { ProfileSwitchPage } from '@/pages/ProfileSwitchPage';
import { SettingsPage } from '@/pages/SettingsPage';
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

type TabType = 'dashboard' | 'install' | 'config' | 'switch' | 'statistics' | 'settings';

function App() {
  const { toast } = useToast();
  const [activeTab, setActiveTab] = useState<TabType>('dashboard');

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

  // 更新检查状态
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [updateCheckDone, setUpdateCheckDone] = useState(false);

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
      const [quota, stats] = await Promise.all([getUserQuota(), getUsageStats()]);
      setUserQuota(quota);
      setUsageStats(stats);
      setStatsLoadFailed(false);
    } catch (error) {
      console.error('Failed to load statistics:', error);
      setStatsLoadFailed(true);

      toast({
        title: '加载统计数据失败',
        description: error instanceof Error ? error.message : '请检查网络连接后重试',
        variant: 'destructive',
        duration: 5000,
      });
    } finally {
      setStatsLoading(false);
    }
  }, [globalConfig?.user_id, globalConfig?.system_token, toast]);

  // 检查应用更新
  const checkAppUpdates = useCallback(async () => {
    // 避免重复检查
    if (updateCheckDone) {
      return;
    }

    try {
      console.log('Checking for app updates...');
      const update = await checkForAppUpdates();
      setUpdateInfo(update);

      // 如果有可用更新，显示通知
      if (update.has_update) {
        toast({
          title: '发现新版本',
          description: `DuckCoding ${update.latest_version} 现已可用，当前版本：${update.current_version}`,
          action: (
            <button
              onClick={() => {
                setActiveTab('settings');
                // 延迟一点时间确保页面切换后再设置子tab
                setTimeout(() => {
                  // 通过事件或者状态管理来设置更新tab
                  const event = new CustomEvent('navigate-to-update-tab');
                  window.dispatchEvent(event);
                }, 100);
              }}
              className="px-3 py-1.5 text-xs bg-primary text-primary-foreground rounded hover:bg-primary/90 transition-colors whitespace-nowrap"
            >
              立即更新
            </button>
          ),
          duration: 8000, // 显示8秒，给用户足够时间点击
        });
      }
    } catch (error) {
      console.error('Failed to check for updates:', error);
      // 静默失败，不显示错误通知给用户
    } finally {
      setUpdateCheckDone(true);
    }
  }, [updateCheckDone, toast]);

  // 初始化加载工具和全局配置
  useEffect(() => {
    loadTools();
    loadGlobalConfig();
  }, [loadTools, loadGlobalConfig]);

  // 应用启动时检查更新（延迟3秒，避免影响启动速度）
  useEffect(() => {
    const timer = setTimeout(() => {
      checkAppUpdates();
    }, 3000); // 3秒后检查更新

    return () => clearTimeout(timer);
  }, [checkAppUpdates]);

  // 智能预加载：只要有凭证就立即预加载统计数据
  useEffect(() => {
    // 条件：配置已加载 + 有凭证 + 还没有统计数据 + 不在加载中 + 没有失败过
    if (globalConfig?.user_id && globalConfig?.system_token && !usageStats && !statsLoading && !statsLoadFailed) {
      loadStatistics();
    }
  }, [globalConfig?.user_id, globalConfig?.system_token, usageStats, statsLoading, statsLoadFailed, loadStatistics]);

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
            onLoadStatistics={loadStatistics}
          />
        )}
        {activeTab === 'settings' && (
          <SettingsPage
            globalConfig={globalConfig}
            configLoading={configLoading}
            onConfigChange={loadGlobalConfig}
            updateInfo={updateInfo}
            onUpdateCheck={checkAppUpdates}
          />
        )}
      </main>

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
