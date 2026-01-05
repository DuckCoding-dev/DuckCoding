import { useState, useEffect } from 'react';
import { Coins } from 'lucide-react';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type { Provider } from '@/lib/tauri-commands';
import { RemoteTokenManagement } from './RemoteTokenManagement';

interface TokenManagementTabProps {
  providers: Provider[];
  selectedProviderId: string | null;
  onProviderChange: (providerId: string | null) => void;
}

/**
 * 令牌管理 Tab
 * 独立的供应商选择器 + 令牌列表
 */
export function TokenManagementTab({
  providers,
  selectedProviderId,
  onProviderChange,
}: TokenManagementTabProps) {
  const [localSelectedId, setLocalSelectedId] = useState<string | null>(selectedProviderId);

  // 默认选择 "duckcoding" 供应商（如果存在且未选择）
  useEffect(() => {
    if (!selectedProviderId && providers.length > 0) {
      console.log('[TokenManagementTab] 尝试自动选择供应商');
      console.log('[TokenManagementTab] 供应商列表:', providers);

      // 优先匹配 id，其次匹配 name（大小写不敏感）
      const duckcodingProvider = providers.find(
        (p) =>
          p.id.toLowerCase() === 'duckcoding' || p.name.toLowerCase() === 'duckcoding',
      );

      console.log('[TokenManagementTab] 找到的供应商:', duckcodingProvider);

      if (duckcodingProvider) {
        console.log('[TokenManagementTab] 自动选择供应商:', duckcodingProvider.id);
        onProviderChange(duckcodingProvider.id);
      } else {
        console.log('[TokenManagementTab] 未找到 duckcoding 供应商');
      }
    }
  }, [providers, selectedProviderId, onProviderChange]);

  // 同步外部 selectedProviderId 变化
  useEffect(() => {
    setLocalSelectedId(selectedProviderId);
  }, [selectedProviderId]);

  const handleProviderSelect = (value: string) => {
    const newId = value === 'none' ? null : value;
    setLocalSelectedId(newId);
    onProviderChange(newId);
  };

  const selectedProvider = providers.find((p) => p.id === localSelectedId);

  return (
    <div className="space-y-4">
      {/* 供应商选择器 */}
      <div className="flex items-center gap-4">
        <label htmlFor="provider-select" className="text-sm font-medium">
          选择供应商：
        </label>
        <Select value={localSelectedId || 'none'} onValueChange={handleProviderSelect}>
          <SelectTrigger className="w-[300px]" id="provider-select">
            <SelectValue placeholder="请选择供应商" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="none" disabled>
              请选择供应商
            </SelectItem>
            {providers.map((provider) => (
              <SelectItem key={provider.id} value={provider.id}>
                {provider.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {/* 令牌管理内容 */}
      {selectedProvider ? (
        <RemoteTokenManagement provider={selectedProvider} />
      ) : (
        <div className="text-center py-12 text-muted-foreground">
          <Coins className="h-16 w-16 mx-auto mb-4 opacity-20" />
          <p className="text-sm">请先选择一个供应商以查看和管理其令牌</p>
        </div>
      )}
    </div>
  );
}
