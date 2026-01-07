// 工具代理数据管理 Hook（工厂数据层）
// 统一管理三个工具的配置和状态数据

import { useState, useEffect, useCallback } from 'react';
import { getProxyConfig, updateProxyConfig, type ToolProxyConfig } from '@/lib/tauri-commands';
import type { ToolId } from '../types/proxy-history';
import { useProxyControl } from './useProxyControl';

/**
 * 工具数据（配置 + 状态）
 */
export interface ToolData {
  toolId: ToolId;
  config: ToolProxyConfig | null;
  isRunning: boolean;
  port: number | null;
}

/**
 * 工具代理数据管理 Hook
 *
 * 功能：
 * - 从 ProxyConfigManager 读取配置（proxy.json）
 * - 从代理状态中读取运行信息
 * - 提供统一的数据访问接口（工厂模式）
 */
export function useToolProxyData() {
  const [configs, setConfigs] = useState<Map<ToolId, ToolProxyConfig | null>>(new Map());
  const [configLoading, setConfigLoading] = useState(true);

  // 使用代理控制 Hook
  const { proxyStatus, isRunning, getPort, refreshProxyStatus } = useProxyControl();

  /**
   * 加载指定工具的配置
   */
  const loadToolConfig = useCallback(async (toolId: ToolId) => {
    try {
      const config = await getProxyConfig(toolId);
      setConfigs((prev) => new Map(prev).set(toolId, config));
      return config;
    } catch (error) {
      console.error(`加载 ${toolId} 配置失败:`, error);
      setConfigs((prev) => new Map(prev).set(toolId, null));
      return null;
    }
  }, []);

  /**
   * 加载所有工具配置
   */
  const loadAllConfigs = useCallback(async () => {
    setConfigLoading(true);
    try {
      await Promise.all([
        loadToolConfig('claude-code'),
        loadToolConfig('codex'),
        loadToolConfig('gemini-cli'),
        loadToolConfig('amp-code'),
      ]);
    } finally {
      setConfigLoading(false);
    }
  }, [loadToolConfig]);

  /**
   * 刷新数据（配置 + 状态）
   */
  const refreshData = useCallback(async () => {
    await Promise.all([loadAllConfigs(), refreshProxyStatus()]);
  }, [loadAllConfigs, refreshProxyStatus]);

  /**
   * 获取指定工具的完整数据（工厂方法）
   */
  const getToolData = useCallback(
    (toolId: ToolId): ToolData => {
      const config = configs.get(toolId) || null;
      const running = isRunning(toolId);
      const port = getPort(toolId);

      return {
        toolId,
        config,
        isRunning: running,
        port,
      };
    },
    [configs, isRunning, getPort],
  );

  /**
   * 获取所有工具的数据
   */
  const getAllToolsData = useCallback((): ToolData[] => {
    const toolIds: ToolId[] = ['claude-code', 'codex', 'gemini-cli', 'amp-code'];
    return toolIds.map((toolId) => getToolData(toolId));
  }, [getToolData]);

  /**
   * 保存指定工具的配置
   */
  const saveToolConfig = useCallback(
    async (toolId: ToolId, updates: Partial<ToolProxyConfig>): Promise<void> => {
      const currentConfig = configs.get(toolId) || {
        enabled: false,
        port:
          toolId === 'claude-code'
            ? 8787
            : toolId === 'codex'
              ? 8788
              : toolId === 'gemini-cli'
                ? 8789
                : toolId === 'amp-code'
                  ? 8790
                  : 8791,
        local_api_key: null,
        real_api_key: null,
        real_base_url: null,
        real_model_provider: null,
        real_profile_name: null,
        allow_public: false,
        session_endpoint_config_enabled: false,
        auto_start: false,
      };

      const updatedConfig: ToolProxyConfig = {
        ...currentConfig,
        ...updates,
      };

      await updateProxyConfig(toolId, updatedConfig);
      setConfigs((prev) => new Map(prev).set(toolId, updatedConfig));
    },
    [configs],
  );

  // 初始加载
  useEffect(() => {
    loadAllConfigs();
  }, [loadAllConfigs]);

  return {
    configLoading,
    proxyStatus,
    getToolData,
    getAllToolsData,
    saveToolConfig,
    refreshData,
    loadGlobalConfig: loadAllConfigs,
    refreshProxyStatus,
  };
}
