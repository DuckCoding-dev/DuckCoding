import { useCallback, useEffect, useState } from 'react';
import { BalanceConfig, StoragePayload } from '../types';

const STORAGE_KEY = 'duckcoding.balance.configs';
const STORAGE_VERSION = 1;

function safeParse(raw: string | null): StoragePayload | null {
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw);
    const version = typeof parsed.version === 'number' ? parsed.version : 0;
    const configs = Array.isArray(parsed.configs) ? parsed.configs : [];
    return { version, configs };
  } catch {
    return null;
  }
}

export function useBalanceConfigs() {
  const [configs, setConfigs] = useState<BalanceConfig[]>([]);

  const persist = useCallback((next: BalanceConfig[]) => {
    setConfigs(next);
    const payload: StoragePayload = { version: STORAGE_VERSION, configs: next };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
  }, []);

  const loadConfigs = useCallback(() => {
    const payload = safeParse(localStorage.getItem(STORAGE_KEY));
    if (!payload) return;
    setConfigs(payload.configs);
  }, []);

  const addConfig = useCallback(
    (config: BalanceConfig) => {
      persist([...configs, config]);
    },
    [configs, persist],
  );

  const updateConfig = useCallback(
    (config: BalanceConfig) => {
      persist(configs.map((c) => (c.id === config.id ? config : c)));
    },
    [configs, persist],
  );

  const deleteConfig = useCallback(
    (id: string) => {
      persist(configs.filter((c) => c.id !== id));
    },
    [configs, persist],
  );

  useEffect(() => {
    loadConfigs();
  }, [loadConfigs]);

  return {
    configs,
    addConfig,
    updateConfig,
    deleteConfig,
    loadConfigs,
  };
}
