import { useState, useEffect, useCallback } from 'react';
import {
  listProviders,
  createProvider,
  updateProvider,
  deleteProvider,
  type Provider,
} from '@/lib/tauri-commands';

export function useProviderManagement() {
  const [providers, setProviders] = useState<Provider[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // 加载供应商列表
  const loadProviders = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const providerList = await listProviders();
      setProviders(providerList);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      setError(errorMsg);
      console.error('加载供应商失败:', err);
    } finally {
      setLoading(false);
    }
  }, []);

  // 创建供应商
  const handleCreate = useCallback(
    async (provider: Provider) => {
      try {
        await createProvider(provider);
        await loadProviders();
        return { success: true };
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        console.error('创建供应商失败:', err);
        return { success: false, error: errorMsg };
      }
    },
    [loadProviders],
  );

  // 更新供应商
  const handleUpdate = useCallback(
    async (id: string, provider: Provider) => {
      try {
        await updateProvider(id, provider);
        await loadProviders();
        return { success: true };
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        console.error('更新供应商失败:', err);
        return { success: false, error: errorMsg };
      }
    },
    [loadProviders],
  );

  // 删除供应商
  const handleDelete = useCallback(
    async (id: string) => {
      try {
        await deleteProvider(id);
        await loadProviders();
        return { success: true };
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        console.error('删除供应商失败:', err);
        return { success: false, error: errorMsg };
      }
    },
    [loadProviders],
  );

  // 初始化加载
  useEffect(() => {
    loadProviders();
  }, [loadProviders]);

  return {
    providers,
    loading,
    error,
    loadProviders,
    createProvider: handleCreate,
    updateProvider: handleUpdate,
    deleteProvider: handleDelete,
  };
}
