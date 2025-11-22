// 透明代理控制 Hook
// 封装代理启停控制逻辑，复用 Tauri 命令

import { useState, useEffect, useCallback } from 'react';
import {
  startToolProxy,
  stopToolProxy,
  getAllProxyStatus,
  type AllProxyStatus,
} from '@/lib/tauri-commands';
import type { ToolId } from '../types/proxy-history';

/**
 * 代理控制 Hook
 *
 * 功能：
 * - 启动/停止指定工具的透明代理
 * - 获取所有工具的代理运行状态
 * - 管理 loading 状态（启动中/停止中）
 */
export function useProxyControl() {
  // 所有工具的代理状态
  const [proxyStatus, setProxyStatus] = useState<AllProxyStatus>({});

  // 加载状态映射表（工具ID -> 是否加载中）
  const [loadingStates, setLoadingStates] = useState<Record<string, boolean>>({});

  /**
   * 刷新所有代理状态
   */
  const refreshProxyStatus = useCallback(async () => {
    try {
      const status = await getAllProxyStatus();
      setProxyStatus(status);
    } catch (error) {
      console.error('获取代理状态失败:', error);
    }
  }, []);

  /**
   * 启动代理
   */
  const startProxy = useCallback(
    async (toolId: ToolId): Promise<{ success: boolean; message: string }> => {
      setLoadingStates((prev) => ({ ...prev, [toolId]: true }));

      try {
        const message = await startToolProxy(toolId);
        await refreshProxyStatus(); // 刷新状态
        return { success: true, message };
      } catch (error: any) {
        return {
          success: false,
          message: error?.message || String(error),
        };
      } finally {
        setLoadingStates((prev) => ({ ...prev, [toolId]: false }));
      }
    },
    [refreshProxyStatus],
  );

  /**
   * 停止代理
   */
  const stopProxy = useCallback(
    async (toolId: ToolId): Promise<{ success: boolean; message: string }> => {
      setLoadingStates((prev) => ({ ...prev, [toolId]: true }));

      try {
        const message = await stopToolProxy(toolId);
        await refreshProxyStatus(); // 刷新状态
        return { success: true, message };
      } catch (error: any) {
        return {
          success: false,
          message: error?.message || String(error),
        };
      } finally {
        setLoadingStates((prev) => ({ ...prev, [toolId]: false }));
      }
    },
    [refreshProxyStatus],
  );

  /**
   * 检查指定工具是否加载中
   */
  const isLoading = useCallback(
    (toolId: ToolId): boolean => {
      return loadingStates[toolId] || false;
    },
    [loadingStates],
  );

  /**
   * 检查指定工具的代理是否运行中
   */
  const isRunning = useCallback(
    (toolId: ToolId): boolean => {
      return proxyStatus[toolId]?.running || false;
    },
    [proxyStatus],
  );

  /**
   * 获取指定工具的代理端口
   */
  const getPort = useCallback(
    (toolId: ToolId): number | null => {
      return proxyStatus[toolId]?.port || null;
    },
    [proxyStatus],
  );

  // 初始加载代理状态
  useEffect(() => {
    refreshProxyStatus();
  }, [refreshProxyStatus]);

  return {
    proxyStatus,
    startProxy,
    stopProxy,
    refreshProxyStatus,
    isLoading,
    isRunning,
    getPort,
  };
}
