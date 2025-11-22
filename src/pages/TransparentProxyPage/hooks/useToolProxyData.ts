// 工具代理数据管理 Hook（工厂数据层）
// 统一管理三个工具的配置和状态数据

import { useState, useEffect, useCallback } from 'react';
import { getGlobalConfig, type GlobalConfig, type ToolProxyConfig } from '@/lib/tauri-commands';
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
 * - 从 GlobalConfig.proxy_configs 读取配置
 * - 从代理状态中读取运行信息
 * - 提供统一的数据访问接口（工厂模式）
 */
export function useToolProxyData() {
  const [globalConfig, setGlobalConfig] = useState<GlobalConfig | null>(null);
  const [configLoading, setConfigLoading] = useState(true);

  // 使用代理控制 Hook
  const { proxyStatus, isRunning, getPort, refreshProxyStatus } = useProxyControl();

  /**
   * 加载全局配置
   */
  const loadGlobalConfig = useCallback(async () => {
    setConfigLoading(true);
    try {
      const config = await getGlobalConfig();
      setGlobalConfig(config);
    } catch (error) {
      console.error('加载全局配置失败:', error);
    } finally {
      setConfigLoading(false);
    }
  }, []);

  /**
   * 刷新数据（配置 + 状态）
   */
  const refreshData = useCallback(async () => {
    await Promise.all([loadGlobalConfig(), refreshProxyStatus()]);
  }, [loadGlobalConfig, refreshProxyStatus]);

  /**
   * 获取指定工具的完整数据（工厂方法）
   */
  const getToolData = useCallback(
    (toolId: ToolId): ToolData => {
      const config = globalConfig?.proxy_configs?.[toolId] || null;
      const running = isRunning(toolId);
      const port = getPort(toolId);

      return {
        toolId,
        config,
        isRunning: running,
        port,
      };
    },
    [globalConfig, isRunning, getPort],
  );

  /**
   * 获取所有工具的数据
   */
  const getAllToolsData = useCallback((): ToolData[] => {
    const toolIds: ToolId[] = ['claude-code', 'codex', 'gemini-cli'];
    return toolIds.map((toolId) => getToolData(toolId));
  }, [getToolData]);

  // 初始加载
  useEffect(() => {
    loadGlobalConfig();
  }, [loadGlobalConfig]);

  return {
    globalConfig,
    configLoading,
    proxyStatus,
    getToolData,
    getAllToolsData,
    refreshData,
    loadGlobalConfig,
    refreshProxyStatus,
  };
}
