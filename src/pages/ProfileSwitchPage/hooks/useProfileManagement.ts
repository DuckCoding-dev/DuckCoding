import { useState, useCallback, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import {
  switchProfile,
  deleteProfile,
  getActiveConfig,
  getGlobalConfig,
  startToolProxy,
  stopToolProxy,
  getAllProxyStatus,
  saveGlobalConfig,
  getExternalChanges,
  ackExternalChange,
  listProfileDescriptors,
  getMigrationReport,
  importNativeChange,
  cleanLegacyBackups,
  type ToolStatus,
  type GlobalConfig,
  type AllProxyStatus,
  type ExternalConfigChange,
  type ProfileDescriptor,
  type MigrationRecord,
  type ImportExternalChangeResult,
  type LegacyCleanupResult,
} from '@/lib/tauri-commands';
import { useProfileLoader } from '@/hooks/useProfileLoader';

export function useProfileManagement(
  tools: ToolStatus[],
  applySavedOrder: (toolId: string, profiles: string[]) => string[],
) {
  const [switching, setSwitching] = useState(false);
  const [deletingProfiles, setDeletingProfiles] = useState<Record<string, boolean>>({});
  const [globalConfig, setGlobalConfig] = useState<GlobalConfig | null>(null);
  const [allProxyStatus, setAllProxyStatus] = useState<AllProxyStatus>({});
  const [loadingTools, setLoadingTools] = useState<Set<string>>(new Set());
  const [externalChanges, setExternalChanges] = useState<ExternalConfigChange[]>([]);
  const [profileDescriptors, setProfileDescriptors] = useState<ProfileDescriptor[]>([]);
  const [migrationRecords, setMigrationRecords] = useState<MigrationRecord[]>([]);
  const [loadingInsights, setLoadingInsights] = useState(false);
  const [cleaningLegacy, setCleaningLegacy] = useState(false);
  const [cleanupResults, setCleanupResults] = useState<LegacyCleanupResult[]>([]);
  const [notifyEnabled, setNotifyEnabled] = useState(true);
  const [listenerError, setListenerError] = useState<string | null>(null);
  const [pollIntervalMs, setPollIntervalMs] = useState(5000);

  // 使用共享配置加载 Hook，传入排序转换函数
  const { profiles, setProfiles, activeConfigs, setActiveConfigs, loadAllProfiles } =
    useProfileLoader(tools, applySavedOrder);

  // 加载全局配置
  const loadGlobalConfig = useCallback(async () => {
    try {
      const config = await getGlobalConfig();
      setGlobalConfig(config);
      if (config?.external_watch_enabled !== undefined) {
        setNotifyEnabled(config.external_watch_enabled);
      }
      if (config?.external_poll_interval_ms !== undefined) {
        setPollIntervalMs(config.external_poll_interval_ms);
      }
    } catch (error) {
      console.error('Failed to load global config:', error);
    }
  }, []);

  // 加载所有工具的代理状态
  const loadAllProxyStatus = useCallback(async () => {
    try {
      const status = await getAllProxyStatus();
      setAllProxyStatus(status);
    } catch (error) {
      console.error('Failed to load all proxy status:', error);
    }
  }, []);

  const loadExternalChanges = useCallback(async () => {
    setLoadingInsights(true);
    try {
      const [changes, descriptors, records] = await Promise.all([
        getExternalChanges().catch((error) => {
          console.error('Failed to load external changes:', error);
          return [];
        }),
        listProfileDescriptors().catch((error) => {
          console.error('Failed to load profile descriptors:', error);
          return [];
        }),
        getMigrationReport().catch((error) => {
          console.error('Failed to load migration report:', error);
          return [];
        }),
      ]);
      setExternalChanges(changes);
      setProfileDescriptors(descriptors);
      setMigrationRecords(records);
    } finally {
      setLoadingInsights(false);
    }
  }, []);

  const acknowledgeChange = useCallback(
    async (toolId: string) => {
      try {
        await ackExternalChange(toolId);
        await loadExternalChanges();
      } catch (error) {
        console.error('Failed to acknowledge external change:', error);
      }
    },
    [loadExternalChanges],
  );

  // 监听后端事件，实时追加外部改动
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    const setup = async () => {
      try {
        unlisten = await listen<ExternalConfigChange>('external-config-changed', (event) => {
          if (!notifyEnabled) return;
          const payload = event.payload;
          setExternalChanges((prev) => {
            const filtered = prev.filter(
              (c) => !(c.tool_id === payload.tool_id && c.path === payload.path),
            );
            return [...filtered, payload];
          });
        });
      } catch (error) {
        console.error('Failed to listen external-config-changed:', error);
        setListenerError(String(error));
      }
    };

    void setup();
    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [notifyEnabled]);

  // 轮询补偿：按配置间隔刷新外部改动
  useEffect(() => {
    if (!notifyEnabled || pollIntervalMs <= 0) return;
    const timer = setInterval(() => {
      void loadExternalChanges();
    }, pollIntervalMs);
    return () => clearInterval(timer);
  }, [notifyEnabled, pollIntervalMs, loadExternalChanges]);

  // 初次加载/重新开启监听时，立即拉取一次外部改动
  useEffect(() => {
    if (!notifyEnabled) return;
    void loadExternalChanges();
  }, [notifyEnabled, loadExternalChanges]);

  const importExternalChange = useCallback(
    async (
      toolId: string,
      profileName: string,
      asNew: boolean,
    ): Promise<{
      success: boolean;
      message: string;
      result?: ImportExternalChangeResult;
    }> => {
      const trimmedName = profileName.trim();
      if (!trimmedName) {
        return { success: false, message: 'Profile 名称不能为空' };
      }

      setLoadingInsights(true);
      try {
        const result = await importNativeChange(toolId, trimmedName, asNew);
        await Promise.all([loadExternalChanges(), loadAllProfiles()]);

        try {
          const active = await getActiveConfig(toolId);
          setActiveConfigs((prev) => ({ ...prev, [toolId]: active }));
        } catch (error) {
          console.error('Failed to refresh active config after import:', error);
        }

        return {
          success: true,
          message: asNew
            ? `已将外部改动导入为 ${result.profileName}`
            : `已覆盖 profile：${result.profileName}`,
          result,
        };
      } catch (error) {
        console.error('Failed to import external change:', error);
        return { success: false, message: String(error) };
      } finally {
        setLoadingInsights(false);
      }
    },
    [loadExternalChanges, loadAllProfiles, setActiveConfigs],
  );

  const cleanupLegacyBackups = useCallback(async (): Promise<{
    success: boolean;
    message: string;
    results?: LegacyCleanupResult[];
  }> => {
    setCleaningLegacy(true);
    try {
      const results = await cleanLegacyBackups();
      await loadExternalChanges();
      setCleanupResults(results);
      const removed = results.reduce((sum, r) => sum + r.removed.length, 0);
      const failed = results.reduce((sum, r) => sum + r.failed.length, 0);
      return {
        success: true,
        message:
          failed > 0
            ? `已清理旧版备份：成功 ${removed}，失败 ${failed}`
            : `已清理旧版备份：成功 ${removed}`,
        results,
      };
    } catch (error) {
      console.error('Failed to clean legacy backups:', error);
      return { success: false, message: String(error) };
    } finally {
      setCleaningLegacy(false);
    }
  }, [loadExternalChanges]);

  const persistWatchSettings = useCallback(
    async (enabled: boolean, intervalMs: number) => {
      if (!globalConfig) return;
      const next: GlobalConfig = {
        ...globalConfig,
        external_watch_enabled: enabled,
        external_poll_interval_ms: intervalMs,
      };
      await saveGlobalConfig(next);
      setGlobalConfig(next);
      setNotifyEnabled(enabled);
      setPollIntervalMs(intervalMs);
    },
    [globalConfig],
  );

  // 获取指定工具的代理是否启用
  const isToolProxyEnabled = useCallback(
    (toolId: string): boolean => {
      return globalConfig?.proxy_configs?.[toolId]?.enabled || false;
    },
    [globalConfig],
  );

  // 获取指定工具的代理是否运行中
  const isToolProxyRunning = useCallback(
    (toolId: string): boolean => {
      return allProxyStatus[toolId]?.running || false;
    },
    [allProxyStatus],
  );

  // 检查工具是否正在加载
  const isToolLoading = useCallback(
    (toolId: string): boolean => {
      return loadingTools.has(toolId);
    },
    [loadingTools],
  );

  // 切换配置
  const handleSwitchProfile = useCallback(
    async (
      toolId: string,
      profile: string,
    ): Promise<{
      success: boolean;
      message: string;
      isProxyEnabled?: boolean;
    }> => {
      try {
        setSwitching(true);

        // 检查是否启用了透明代理
        const isProxyEnabled = isToolProxyEnabled(toolId) && isToolProxyRunning(toolId);

        // 切换配置（后端会自动处理透明代理更新）
        await switchProfile(toolId, profile);

        // 重新加载当前生效的配置
        try {
          const activeConfig = await getActiveConfig(toolId);
          setActiveConfigs((prev) => ({ ...prev, [toolId]: activeConfig }));
        } catch (error) {
          console.error('Failed to reload active config', error);
        }

        // 刷新配置确保UI显示正确
        await loadGlobalConfig();

        if (isProxyEnabled) {
          return {
            success: true,
            message: '✅ 配置已切换\n✅ 透明代理已自动更新\n无需重启终端',
            isProxyEnabled: true,
          };
        } else {
          return {
            success: true,
            message: '配置切换成功！\n请重启相关 CLI 工具以使新配置生效。',
            isProxyEnabled: false,
          };
        }
      } catch (error) {
        console.error('Failed to switch profile:', error);
        return {
          success: false,
          message: String(error),
        };
      } finally {
        setSwitching(false);
      }
    },
    [isToolProxyEnabled, isToolProxyRunning, loadGlobalConfig, setActiveConfigs],
  );

  // 删除配置
  const handleDeleteProfile = useCallback(
    async (
      toolId: string,
      profile: string,
    ): Promise<{
      success: boolean;
      message: string;
    }> => {
      const profileKey = `${toolId}-${profile}`;

      try {
        setDeletingProfiles((prev) => ({ ...prev, [profileKey]: true }));

        // 后端删除
        await deleteProfile(toolId, profile);

        // 立即本地更新（乐观更新）
        const currentProfiles = profiles[toolId] || [];
        const updatedProfiles = currentProfiles.filter((p) => p !== profile);

        setProfiles((prev) => ({
          ...prev,
          [toolId]: updatedProfiles,
        }));

        // 尝试重新加载所有配置，确保与后端同步
        try {
          const latest = await loadAllProfiles();
          const deletedWasActive =
            latest.activeConfigs[toolId]?.profile_name === profile ||
            activeConfigs[toolId]?.profile_name === profile;

          // 如果删除的是当前正在使用的配置，确保UI展示的生效配置同步更新
          if (deletedWasActive) {
            try {
              const newActiveConfig =
                latest.activeConfigs[toolId] ?? (await getActiveConfig(toolId));
              setActiveConfigs((prev) => ({ ...prev, [toolId]: newActiveConfig }));
            } catch (error) {
              console.error('Failed to reload active config', error);
            }
          }
        } catch (reloadError) {
          console.error('Failed to reload profiles after delete:', reloadError);
        }

        return {
          success: true,
          message: '配置删除成功！',
        };
      } catch (error) {
        console.error('Failed to delete profile:', error);
        return {
          success: false,
          message: String(error),
        };
      } finally {
        setDeletingProfiles((prev) => {
          const updated = { ...prev };
          delete updated[profileKey];
          return updated;
        });
      }
    },
    [profiles, activeConfigs, loadAllProfiles, setProfiles, setActiveConfigs],
  );

  // 启动指定工具的透明代理
  const handleStartToolProxy = useCallback(
    async (
      toolId: string,
    ): Promise<{
      success: boolean;
      message: string;
    }> => {
      try {
        setLoadingTools((prev) => new Set(prev).add(toolId));
        const result = await startToolProxy(toolId);
        // 重新加载状态
        await loadAllProxyStatus();
        return {
          success: true,
          message: result,
        };
      } catch (error) {
        console.error('Failed to start tool proxy:', error);
        return {
          success: false,
          message: String(error),
        };
      } finally {
        setLoadingTools((prev) => {
          const next = new Set(prev);
          next.delete(toolId);
          return next;
        });
      }
    },
    [loadAllProxyStatus],
  );

  // 停止指定工具的透明代理
  const handleStopToolProxy = useCallback(
    async (
      toolId: string,
    ): Promise<{
      success: boolean;
      message: string;
    }> => {
      try {
        setLoadingTools((prev) => new Set(prev).add(toolId));
        const result = await stopToolProxy(toolId);
        // 重新加载状态
        await loadAllProxyStatus();

        // 刷新当前生效配置（确保UI显示正确更新）
        try {
          const activeConfig = await getActiveConfig(toolId);
          setActiveConfigs((prev) => ({ ...prev, [toolId]: activeConfig }));
        } catch (error) {
          console.error('Failed to reload active config after stopping proxy:', error);
        }

        return {
          success: true,
          message: result,
        };
      } catch (error) {
        console.error('Failed to stop tool proxy:', error);
        return {
          success: false,
          message: String(error),
        };
      } finally {
        setLoadingTools((prev) => {
          const next = new Set(prev);
          next.delete(toolId);
          return next;
        });
      }
    },
    [loadAllProxyStatus, setActiveConfigs],
  );

  return {
    // State
    switching,
    deletingProfiles,
    profiles,
    setProfiles,
    activeConfigs,
    globalConfig,
    allProxyStatus,
    externalChanges,
    profileDescriptors,
    migrationRecords,
    loadingInsights,
    cleaningLegacy,
    cleanupResults,
    notifyEnabled,
    listenerError,
    pollIntervalMs,

    // Actions
    loadGlobalConfig,
    loadAllProxyStatus,
    loadExternalChanges,
    acknowledgeChange,
    importExternalChange,
    cleanupLegacyBackups,
    persistWatchSettings,
    setPollIntervalMs,
    loadAllProfiles,
    handleSwitchProfile,
    handleDeleteProfile,
    handleStartToolProxy,
    handleStopToolProxy,

    // Helpers
    isToolProxyEnabled,
    isToolProxyRunning,
    isToolLoading,
  };
}
