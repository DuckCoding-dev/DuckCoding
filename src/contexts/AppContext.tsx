import React, { createContext, useContext, useState, useCallback, useEffect } from 'react';
import { checkInstallations, getGlobalConfig, checkForAppUpdates } from '@/lib/tauri-commands';
import type { ToolStatus, GlobalConfig, UpdateInfo } from '@/lib/tauri-commands';
import type { ToolType } from '@/types/token-stats';

export type TabType =
  | 'dashboard'
  | 'tool-management'
  | 'install'
  | 'profile-management'
  | 'balance'
  | 'transparent-proxy'
  | 'token-statistics'
  | 'provider-management'
  | 'settings'
  | 'help'
  | 'about';

interface AppContextType {
  // Navigation
  activeTab: TabType;
  setActiveTab: (tab: TabType) => void;
  selectedProxyToolId: string | undefined;
  setSelectedProxyToolId: (id: string | undefined) => void;
  settingsInitialTab: string;
  setSettingsInitialTab: (tab: string) => void;
  settingsRestrictToTab: string | undefined;
  setSettingsRestrictToTab: (tab: string | undefined) => void;
  restrictedPage: string | undefined;
  setRestrictedPage: (page: string | undefined) => void;
  tokenStatsParams: { sessionId?: string; toolType?: ToolType };
  setTokenStatsParams: (params: { sessionId?: string; toolType?: ToolType }) => void;

  // Global Data
  tools: ToolStatus[];
  toolsLoading: boolean;
  refreshTools: () => Promise<void>;
  globalConfig: GlobalConfig | null;
  configLoading: boolean;
  refreshGlobalConfig: () => Promise<void>;

  // Updates
  updateInfo: UpdateInfo | null;
  setUpdateInfo: (info: UpdateInfo | null) => void;
  isUpdateDialogOpen: boolean;
  setIsUpdateDialogOpen: (open: boolean) => void;
  checkAppUpdates: (force?: boolean) => Promise<void>;
}

const AppContext = createContext<AppContextType | undefined>(undefined);

export function AppProvider({ children }: { children: React.ReactNode }) {
  // Navigation State
  const [activeTab, setActiveTab] = useState<TabType>('dashboard');
  const [selectedProxyToolId, setSelectedProxyToolId] = useState<string | undefined>(undefined);
  const [settingsInitialTab, setSettingsInitialTab] = useState<string>('basic');
  const [settingsRestrictToTab, setSettingsRestrictToTab] = useState<string | undefined>(undefined);
  const [restrictedPage, setRestrictedPage] = useState<string | undefined>(undefined);
  const [tokenStatsParams, setTokenStatsParams] = useState<{
    sessionId?: string;
    toolType?: ToolType;
  }>({});

  // Data State
  const [tools, setTools] = useState<ToolStatus[]>([]);
  const [toolsLoading, setToolsLoading] = useState(true);
  const [globalConfig, setGlobalConfig] = useState<GlobalConfig | null>(null);
  const [configLoading, setConfigLoading] = useState(false);

  // Update State
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [isUpdateDialogOpen, setIsUpdateDialogOpen] = useState(false);
  const [updateCheckDone, setUpdateCheckDone] = useState(false);

  const refreshTools = useCallback(async () => {
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

  const refreshGlobalConfig = useCallback(async () => {
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

  const checkAppUpdates = useCallback(
    async (force = false) => {
      if (updateCheckDone && !force) return;

      try {
        console.log('Checking for app updates...');
        const update = await checkForAppUpdates();
        setUpdateInfo(update);

        if (update.has_update) {
          setIsUpdateDialogOpen(true);
        }
      } catch (error) {
        console.error('Failed to check for updates:', error);
      } finally {
        setUpdateCheckDone(true);
      }
    },
    [updateCheckDone],
  );

  // Initial Load
  useEffect(() => {
    refreshTools();
    refreshGlobalConfig();
    
    // Initial update check delay
    const timer = setTimeout(() => {
      checkAppUpdates();
    }, 1000);
    return () => clearTimeout(timer);
  }, [refreshTools, refreshGlobalConfig, checkAppUpdates]);

  const value = {
    activeTab,
    setActiveTab,
    selectedProxyToolId,
    setSelectedProxyToolId,
    settingsInitialTab,
    setSettingsInitialTab,
    settingsRestrictToTab,
    setSettingsRestrictToTab,
    restrictedPage,
    setRestrictedPage,
    tokenStatsParams,
    setTokenStatsParams,
    tools,
    toolsLoading,
    refreshTools,
    globalConfig,
    configLoading,
    refreshGlobalConfig,
    updateInfo,
    setUpdateInfo,
    isUpdateDialogOpen,
    setIsUpdateDialogOpen,
    checkAppUpdates,
  };

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
}

export function useAppContext() {
  const context = useContext(AppContext);
  if (context === undefined) {
    throw new Error('useAppContext must be used within an AppProvider');
  }
  return context;
}