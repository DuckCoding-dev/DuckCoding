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
import { checkInstallations, type CloseAction, type ToolStatus } from '@/lib/tauri-commands';

type TabType = 'dashboard' | 'install' | 'config' | 'switch' | 'statistics' | 'settings';

function App() {
  const { toast } = useToast();
  const [activeTab, setActiveTab] = useState<TabType>('dashboard');

  // 全局工具状态缓存
  const [tools, setTools] = useState<ToolStatus[]>([]);
  const [toolsLoading, setToolsLoading] = useState(true);

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

  // 初始化加载工具
  useEffect(() => {
    loadTools();
  }, [loadTools]);

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
        {activeTab === 'statistics' && <StatisticsPage />}
        {activeTab === 'settings' && <SettingsPage />}
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
