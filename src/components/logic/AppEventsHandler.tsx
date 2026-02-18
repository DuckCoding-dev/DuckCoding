import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useAppContext } from '@/hooks/useAppContext';
import { useToast } from '@/hooks/use-toast';
import { useAppEvents } from '@/hooks/useAppEvents';
import { useCloseAction } from '@/hooks/useCloseAction';
import { CloseActionDialog } from '@/components/dialogs/CloseActionDialog';
import type { UpdateInfo, CloseAction } from '@/lib/tauri-commands';
import type { ToolType } from '@/types/token-stats';
import type { TabType } from '@/contexts/AppContext.types';

export function AppEventsHandler() {
  const {
    setActiveTab,
    setSettingsInitialTab,
    setSettingsRestrictToTab,
    setSelectedProxyToolId,
    setTokenStatsParams,
    setUpdateInfo,
    setIsUpdateDialogOpen,
    refreshTools,
  } = useAppContext();

  const { toast } = useToast();

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

  useAppEvents({
    onCloseRequest: openCloseDialog,
    onSingleInstance: (message: string) => {
      toast({
        title: 'DuckCoding 已在运行',
        description: message,
      });
    },
    onNavigateToInstall: () => setActiveTab('install'),
    onNavigateToList: () => setActiveTab('tool-management'),
    onNavigateToConfig: (_detail) => {
      setActiveTab('profile-management');
    },
    onNavigateToSettings: (detail) => {
      setSettingsInitialTab(detail?.tab ?? 'basic');
      setActiveTab('settings');
    },
    onNavigateToTransparentProxy: (detail) => {
      setActiveTab('transparent-proxy');
      if (detail?.toolId) {
        setSelectedProxyToolId(detail.toolId);
      }
    },
    onRefreshTools: refreshTools,
    executeCloseAction,
  });

  // Additional Event Listeners
  useEffect(() => {
    const unlistenUpdateAvailable = listen<UpdateInfo>('update-available', (event) => {
      setUpdateInfo(event.payload);
      setIsUpdateDialogOpen(true);
    });

    const unlistenRequestCheck = listen('request-check-update', () => {
      setUpdateInfo(null);
      setIsUpdateDialogOpen(true);
    });

    const unlistenNotFound = listen('update-not-found', () => {
      toast({
        title: '已是最新版本',
        description: '当前无可用更新',
      });
    });

    const unlistenOpenSettings = listen<{ tab?: string; restrictToTab?: boolean }>(
      'open-settings',
      (event) => {
        const tab = event.payload?.tab || 'basic';
        const restrictToTab = event.payload?.restrictToTab || false;
        setSettingsInitialTab(tab);
        if (restrictToTab) {
          setSettingsRestrictToTab(tab);
        } else {
          setSettingsRestrictToTab(undefined);
        }
        setActiveTab('settings');
      },
    );

    // TODO: Add Onboarding Navigation Logic here or in OnboardingManager

    const unlistenAppNavigate = listen<{
      tab: TabType;
      params?: { sessionId?: string; toolType?: ToolType };
    }>('app-navigate', (event) => {
      const { tab, params } = event.payload || {};
      if (tab) {
        setActiveTab(tab);
        if (tab === 'token-statistics' && params) {
          setTokenStatsParams(params);
        } else if (tab !== 'token-statistics') {
          setTokenStatsParams({});
        }
      }
    });

    // 监听从菜单栏激活 Profile 的事件
    const unlistenProfileActivated = listen<{ tool_id: string; profile_name: string }>(
      'profile-activated-from-menu',
      (event) => {
        const { tool_id, profile_name } = event.payload;
        toast({
          title: '配置已切换',
          description: `${tool_id} 已切换到 "${profile_name}"`,
        });
      },
    );

    // 监听菜单栏导航事件
    const unlistenNavigateTo = listen<string>('navigate-to', (event) => {
      const path = event.payload;
      if (path === '/profile') {
        setActiveTab('profile-management');
      } else if (path === '/settings') {
        setActiveTab('settings');
      } else if (path.startsWith('/transparent-proxy/')) {
        const toolId = path.replace('/transparent-proxy/', '').trim();
        if (toolId === 'claude-code' || toolId === 'codex' || toolId === 'gemini-cli') {
          setSelectedProxyToolId(toolId);
        }
        setActiveTab('transparent-proxy');
      }
    });

    return () => {
      unlistenUpdateAvailable.then((fn) => fn());
      unlistenRequestCheck.then((fn) => fn());
      unlistenNotFound.then((fn) => fn());
      unlistenOpenSettings.then((fn) => fn());
      unlistenAppNavigate.then((fn) => fn());
      unlistenProfileActivated.then((fn) => fn());
      unlistenNavigateTo.then((fn) => fn());
    };
  }, [
    setActiveTab,
    setSettingsInitialTab,
    setSettingsRestrictToTab,
    setSelectedProxyToolId,
    setIsUpdateDialogOpen,
    setUpdateInfo,
    setTokenStatsParams,
    toast,
  ]);

  return (
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
  );
}
