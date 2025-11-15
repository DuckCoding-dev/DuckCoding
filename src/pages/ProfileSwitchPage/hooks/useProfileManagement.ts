import { useState, useCallback } from 'react';
import {
  switchProfile,
  deleteProfile,
  getActiveConfig,
  getGlobalConfig,
  startTransparentProxy,
  stopTransparentProxy,
  getTransparentProxyStatus,
  type ToolStatus,
  type GlobalConfig,
  type TransparentProxyStatus,
} from '@/lib/tauri-commands';
import { useProfileLoader } from '@/hooks/useProfileLoader';

export function useProfileManagement(
  tools: ToolStatus[],
  applySavedOrder: (toolId: string, profiles: string[]) => string[],
) {
  const [switching, setSwitching] = useState(false);
  const [deletingProfiles, setDeletingProfiles] = useState<Record<string, boolean>>({});
  const [selectedProfile, setSelectedProfile] = useState<Record<string, string>>({});
  const [globalConfig, setGlobalConfig] = useState<GlobalConfig | null>(null);
  const [transparentProxyStatus, setTransparentProxyStatus] =
    useState<TransparentProxyStatus | null>(null);
  const [startingProxy, setStartingProxy] = useState(false);
  const [stoppingProxy, setStoppingProxy] = useState(false);

  // 使用共享配置加载 Hook，传入排序转换函数
  const { profiles, setProfiles, activeConfigs, setActiveConfigs, loadAllProfiles } =
    useProfileLoader(tools, applySavedOrder);

  // 加载全局配置
  const loadGlobalConfig = useCallback(async () => {
    try {
      const config = await getGlobalConfig();
      setGlobalConfig(config);
    } catch (error) {
      console.error('Failed to load global config:', error);
    }
  }, []);

  // 加载透明代理状态
  const loadTransparentProxyStatus = useCallback(async () => {
    try {
      const status = await getTransparentProxyStatus();
      setTransparentProxyStatus(status);
    } catch (error) {
      console.error('Failed to load transparent proxy status:', error);
    }
  }, []);

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
        const isProxyEnabled =
          globalConfig?.transparent_proxy_enabled && transparentProxyStatus?.running;

        // 切换配置（后端会自动处理透明代理更新）
        await switchProfile(toolId, profile);
        setSelectedProfile((prev) => ({ ...prev, [toolId]: profile }));

        // 重新加载当前生效的配置
        try {
          const activeConfig = await getActiveConfig(toolId);
          setActiveConfigs((prev) => ({ ...prev, [toolId]: activeConfig }));
        } catch (error) {
          console.error('Failed to reload active config', error);
        }

        // 如果是 ClaudeCode，总是刷新配置确保UI显示正确
        if (toolId === 'claude-code') {
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
        } else {
          return {
            success: true,
            message: '配置切换成功！\n请重启相关 CLI 工具以使新配置生效。',
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
    [globalConfig, transparentProxyStatus, loadGlobalConfig, setActiveConfigs],
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

        // 清理相关状态
        setSelectedProfile((prev) => {
          const updated = { ...prev };
          if (updated[toolId] === profile) {
            delete updated[toolId];
          }
          return updated;
        });

        // 尝试重新加载所有配置，确保与后端同步
        try {
          await loadAllProfiles();

          // 如果删除的是当前正在使用的配置，重新获取当前配置
          if (activeConfigs[toolId]?.profile_name === profile) {
            try {
              const newActiveConfig = await getActiveConfig(toolId);
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

  // 启动透明代理
  const handleStartTransparentProxy = useCallback(async (): Promise<{
    success: boolean;
    message: string;
  }> => {
    try {
      setStartingProxy(true);
      const result = await startTransparentProxy();
      // 重新加载状态
      const status = await getTransparentProxyStatus();
      setTransparentProxyStatus(status);
      return {
        success: true,
        message: result,
      };
    } catch (error) {
      console.error('Failed to start transparent proxy:', error);
      return {
        success: false,
        message: String(error),
      };
    } finally {
      setStartingProxy(false);
    }
  }, []);

  // 停止透明代理
  const handleStopTransparentProxy = useCallback(async (): Promise<{
    success: boolean;
    message: string;
  }> => {
    try {
      setStoppingProxy(true);
      const result = await stopTransparentProxy();
      // 重新加载状态
      const status = await getTransparentProxyStatus();
      setTransparentProxyStatus(status);

      // 刷新当前生效配置（确保UI显示正确更新）
      try {
        const activeConfig = await getActiveConfig('claude-code');
        setActiveConfigs((prev) => ({ ...prev, 'claude-code': activeConfig }));
      } catch (error) {
        console.error('Failed to reload active config after stopping proxy:', error);
      }

      return {
        success: true,
        message: result,
      };
    } catch (error) {
      console.error('Failed to stop transparent proxy:', error);
      return {
        success: false,
        message: String(error),
      };
    } finally {
      setStoppingProxy(false);
    }
  }, [setActiveConfigs]);

  return {
    // State
    switching,
    deletingProfiles,
    selectedProfile,
    profiles,
    setProfiles,
    activeConfigs,
    globalConfig,
    transparentProxyStatus,
    startingProxy,
    stoppingProxy,

    // Actions
    loadGlobalConfig,
    loadTransparentProxyStatus,
    loadAllProfiles,
    handleSwitchProfile,
    handleDeleteProfile,
    handleStartTransparentProxy,
    handleStopTransparentProxy,
  };
}
