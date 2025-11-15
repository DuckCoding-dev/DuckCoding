import { useState, useCallback } from 'react';
import {
  startTransparentProxy,
  stopTransparentProxy,
  getTransparentProxyStatus,
  type TransparentProxyStatus,
} from '@/lib/tauri-commands';

export function useTransparentProxy() {
  const [transparentProxyStatus, setTransparentProxyStatus] =
    useState<TransparentProxyStatus | null>(null);
  const [startingProxy, setStartingProxy] = useState(false);
  const [stoppingProxy, setStoppingProxy] = useState(false);

  // 加载透明代理状态
  const loadTransparentProxyStatus = useCallback(async () => {
    try {
      const status = await getTransparentProxyStatus();
      setTransparentProxyStatus(status);
    } catch (error) {
      console.error('Failed to load transparent proxy status:', error);
      throw error;
    }
  }, []);

  // 启动透明代理
  const handleStartProxy = useCallback(async (): Promise<string> => {
    setStartingProxy(true);
    try {
      const result = await startTransparentProxy();
      await loadTransparentProxyStatus();
      return result;
    } finally {
      setStartingProxy(false);
    }
  }, [loadTransparentProxyStatus]);

  // 停止透明代理
  const handleStopProxy = useCallback(async (): Promise<string> => {
    setStoppingProxy(true);
    try {
      const result = await stopTransparentProxy();
      await loadTransparentProxyStatus();
      return result;
    } finally {
      setStoppingProxy(false);
    }
  }, [loadTransparentProxyStatus]);

  return {
    transparentProxyStatus,
    startingProxy,
    stoppingProxy,
    loadTransparentProxyStatus,
    handleStartProxy,
    handleStopProxy,
  };
}
