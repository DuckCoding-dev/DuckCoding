import { useCallback, useEffect, useRef, useState } from 'react';
import { BalanceConfig, BalanceResult, BalanceStateMap } from '../types';
import { fetchApi } from '@/lib/tauri-commands';
import { executeExtractor } from '../utils/extractor';

export function useBalanceMonitor(
  configs: BalanceConfig[],
  getApiKey: (id: string) => string | undefined,
  enabled: boolean,
) {
  const [stateMap, setStateMap] = useState<BalanceStateMap>({});
  const timers = useRef<Record<string, ReturnType<typeof setInterval>>>({});

  const refreshOne = useCallback(
    async (id: string) => {
      const config = configs.find((c) => c.id === id);
      if (!config) return;

      const apiKey = getApiKey(id);

      setStateMap((prev) => ({
        ...prev,
        [id]: { ...(prev[id] ?? {}), loading: true, error: null },
      }));

      try {
        // 构建 headers（合并静态和动态）
        const headers: Record<string, string> = {
          ...config.staticHeaders,
        };

        // 如果有 API Key，添加到 Authorization header
        if (apiKey) {
          headers['Authorization'] = `Bearer ${apiKey}`;
        }

        // 调用 Rust 后端获取原始响应
        const response = await fetchApi(config.endpoint, config.method, headers, config.timeoutMs);

        // 执行 extractor 提取余额信息
        const result: BalanceResult = executeExtractor(response, config.extractorScript);

        setStateMap((prev) => ({
          ...prev,
          [id]: {
            loading: false,
            error: null,
            lastResult: result,
            lastFetchedAt: Date.now(),
          },
        }));
      } catch (error) {
        const message = error instanceof Error ? error.message : '查询失败';
        setStateMap((prev) => ({
          ...prev,
          [id]: { ...(prev[id] ?? {}), loading: false, error: message },
        }));
      }
    },
    [configs, getApiKey],
  );

  const refreshAll = useCallback(
    () => Promise.all(configs.map((c) => refreshOne(c.id))),
    [configs, refreshOne],
  );

  // 自动刷新管理
  useEffect(() => {
    Object.values(timers.current).forEach(clearInterval);
    timers.current = {};

    if (!enabled) return;

    configs.forEach((config) => {
      const apiKey = getApiKey(config.id);
      if (config.intervalSec && config.intervalSec > 0 && apiKey) {
        // 立即执行一次查询
        refreshOne(config.id);
        // 启动定时器
        timers.current[config.id] = setInterval(() => {
          refreshOne(config.id);
        }, config.intervalSec * 1000);
      }
    });

    return () => {
      Object.values(timers.current).forEach(clearInterval);
      timers.current = {};
    };
  }, [configs, enabled, refreshOne, getApiKey]);

  // 清理已删除配置的状态
  useEffect(() => {
    setStateMap((prev) => {
      const next: BalanceStateMap = {};
      configs.forEach((c) => {
        next[c.id] = prev[c.id] ?? { loading: false };
      });
      return next;
    });
  }, [configs]);

  return { stateMap, refreshOne, refreshAll };
}
