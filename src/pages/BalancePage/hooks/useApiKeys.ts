import { useState, useCallback } from 'react';
import { ApiKeyMap } from '../types';

export function useApiKeys() {
  const [apiKeys, setApiKeys] = useState<ApiKeyMap>({});

  const setApiKey = useCallback((id: string, key: string) => {
    setApiKeys((prev) => ({ ...prev, [id]: key }));
  }, []);

  const removeApiKey = useCallback((id: string) => {
    setApiKeys((prev) => {
      const next = { ...prev };
      delete next[id];
      return next;
    });
  }, []);

  const getApiKey = useCallback((id: string) => apiKeys[id], [apiKeys]);

  return { apiKeys, setApiKey, removeApiKey, getApiKey };
}
