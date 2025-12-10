import { useState, useCallback, useEffect } from 'react';
import { ApiKeyMap } from '../types';
import type { BalanceConfig } from '../types';

/**
 * useApiKeys Hook
 *
 * 管理余额监控配置的 API Key：
 * - 从配置中加载已保存的 API Key
 * - 支持内存存储（默认）和持久化存储（可选）
 */
export function useApiKeys(configs: BalanceConfig[]) {
  const [apiKeys, setApiKeys] = useState<ApiKeyMap>({});

  /**
   * 初始化：从配置中加载已保存的 API Key
   * 注意：只添加配置中有的 API Key，不覆盖内存中已有的 Key
   */
  useEffect(() => {
    setApiKeys((prev) => {
      const next = { ...prev }; // 保留现有的内存 Key

      configs.forEach((config) => {
        if (config.apiKey) {
          // 配置中有 API Key，更新到内存
          next[config.id] = config.apiKey;
        }
        // 配置中没有 API Key，保留内存中的（如果有）
      });

      // 清理已删除配置的 API Key
      const configIds = new Set(configs.map((c) => c.id));
      Object.keys(next).forEach((id) => {
        if (!configIds.has(id)) {
          delete next[id];
        }
      });

      return next;
    });
  }, [configs]);

  /**
   * 设置 API Key（仅内存存储）
   * 注意：不会自动持久化，如需持久化请使用 setApiKeyWithSave
   */
  const setApiKey = useCallback((id: string, key: string) => {
    setApiKeys((prev) => ({ ...prev, [id]: key }));
  }, []);

  /**
   * 移除 API Key
   */
  const removeApiKey = useCallback((id: string) => {
    setApiKeys((prev) => {
      const next = { ...prev };
      delete next[id];
      return next;
    });
  }, []);

  /**
   * 获取 API Key
   */
  const getApiKey = useCallback((id: string) => apiKeys[id], [apiKeys]);

  return { apiKeys, setApiKey, removeApiKey, getApiKey };
}
