import { useCallback, useEffect, useState } from 'react';
import type { BalanceConfig } from '@/pages/BalancePage/types';
import {
  loadBalanceConfigs,
  saveBalanceConfig,
  updateBalanceConfig,
  deleteBalanceConfig,
  migrateBalanceFromLocalstorage,
} from '@/lib/tauri-commands';

const STORAGE_KEY = 'duckcoding.balance.configs';

/**
 * localStorage 存储格式（旧版本）
 */
interface LegacyStoragePayload {
  version: number;
  configs: BalanceConfig[];
}

/**
 * 从 localStorage 读取旧配置（用于迁移）
 */
function readLegacyConfigs(): BalanceConfig[] | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;

    const payload: LegacyStoragePayload = JSON.parse(raw);
    if (!Array.isArray(payload.configs)) return null;

    return payload.configs;
  } catch (error) {
    console.error('读取 localStorage 配置失败:', error);
    return null;
  }
}

/**
 * 清理 localStorage 数据（迁移完成后）
 */
function clearLegacyConfigs(): void {
  try {
    localStorage.removeItem(STORAGE_KEY);
    console.log('已清理 localStorage 旧数据');
  } catch (error) {
    console.error('清理 localStorage 失败:', error);
  }
}

export function useBalanceConfigs() {
  const [configs, setConfigs] = useState<BalanceConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [migrating, setMigrating] = useState(false);

  /**
   * 从后端加载配置
   */
  const loadConfigs = useCallback(async () => {
    try {
      setLoading(true);
      const store = await loadBalanceConfigs();
      setConfigs(store.configs);
    } catch (error) {
      console.error('加载配置失败:', error);
      // 如果后端加载失败，尝试使用空数组避免页面崩溃
      setConfigs([]);
    } finally {
      setLoading(false);
    }
  }, []);

  /**
   * 自动迁移逻辑
   * 在首次加载时检测 localStorage 并自动迁移
   */
  const autoMigrate = useCallback(async () => {
    try {
      // 1. 先尝试加载后端数据
      const store = await loadBalanceConfigs();

      // 2. 如果后端有数据，说明已经迁移过或是新用户，直接返回
      if (store.configs.length > 0) {
        setConfigs(store.configs);
        // 静默清理 localStorage（可能遗留）
        clearLegacyConfigs();
        return;
      }

      // 3. 后端无数据，检查 localStorage 是否有旧数据
      const legacyConfigs = readLegacyConfigs();
      if (!legacyConfigs || legacyConfigs.length === 0) {
        // 完全的新用户，无需迁移
        setConfigs([]);
        return;
      }

      // 4. 执行迁移
      console.log(`检测到 ${legacyConfigs.length} 个旧配置，开始迁移...`);
      setMigrating(true);

      // 处理旧配置：添加新字段
      const migratedConfigs = legacyConfigs.map((config) => ({
        ...config,
        save_api_key: true, // 默认勾选保存
        api_key: undefined, // 旧版本的 API Key 在内存中，不迁移
      }));

      // 调用后端迁移命令
      const count = await migrateBalanceFromLocalstorage(migratedConfigs);
      console.log(`迁移成功！已保存 ${count} 个配置到 balance.json`);

      // 重新加载
      const newStore = await loadBalanceConfigs();
      setConfigs(newStore.configs);

      // 清理 localStorage
      clearLegacyConfigs();

      // 可选：提示用户
      if (count > 0) {
        console.info(`✅ 已自动迁移 ${count} 个余额监控配置到新存储`);
      }
    } catch (error) {
      console.error('自动迁移失败:', error);
      // 迁移失败时，仍尝试显示后端数据
      try {
        const store = await loadBalanceConfigs();
        setConfigs(store.configs);
      } catch {
        setConfigs([]);
      }
    } finally {
      setMigrating(false);
    }
  }, []);

  /**
   * 添加配置
   */
  const addConfig = useCallback(
    async (config: BalanceConfig) => {
      try {
        await saveBalanceConfig(config);
        await loadConfigs(); // 重新加载以获取最新数据
      } catch (error) {
        console.error('添加配置失败:', error);
        throw error;
      }
    },
    [loadConfigs],
  );

  /**
   * 更新配置
   */
  const updateConfig = useCallback(
    async (config: BalanceConfig) => {
      try {
        await updateBalanceConfig(config);
        await loadConfigs(); // 重新加载
      } catch (error) {
        console.error('更新配置失败:', error);
        throw error;
      }
    },
    [loadConfigs],
  );

  /**
   * 删除配置
   */
  const deleteConfig = useCallback(
    async (id: string) => {
      try {
        await deleteBalanceConfig(id);
        await loadConfigs(); // 重新加载
      } catch (error) {
        console.error('删除配置失败:', error);
        throw error;
      }
    },
    [loadConfigs],
  );

  // 初始化：自动迁移 + 加载配置
  useEffect(() => {
    autoMigrate().finally(() => setLoading(false));
  }, [autoMigrate]);

  return {
    configs,
    loading,
    migrating,
    addConfig,
    updateConfig,
    deleteConfig,
    loadConfigs,
  };
}
