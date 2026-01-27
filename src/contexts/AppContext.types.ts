import { createContext } from 'react';
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

export interface AppContextType {
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

export const AppContext = createContext<AppContextType | undefined>(undefined);
