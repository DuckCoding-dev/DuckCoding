import { useMemo, useState } from 'react';
import { Button } from '@/components/ui/button';
import { PageContainer } from '@/components/layout/PageContainer';
import { Separator } from '@/components/ui/separator';
import { Loader2, Plus, RefreshCw } from 'lucide-react';
import { useBalanceConfigs } from './hooks/useBalanceConfigs';
import { useApiKeys } from './hooks/useApiKeys';
import { useBalanceMonitor } from './hooks/useBalanceMonitor';
import { BalanceConfig, BalanceFormValues } from './types';
import { EmptyState } from './components/EmptyState';
import { ConfigCard } from './components/ConfigCard';
import { ConfigFormDialog } from './components/ConfigFormDialog';

function createId() {
  return typeof crypto !== 'undefined' && crypto.randomUUID
    ? crypto.randomUUID()
    : `cfg_${Date.now()}`;
}

export function BalancePage() {
  const { configs, addConfig, updateConfig, deleteConfig } = useBalanceConfigs();
  const { setApiKey, removeApiKey, getApiKey } = useApiKeys();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingConfig, setEditingConfig] = useState<BalanceConfig | null>(null);
  const [refreshingAll, setRefreshingAll] = useState(false);

  const { stateMap, refreshOne, refreshAll } = useBalanceMonitor(configs, getApiKey, true);

  const sortedConfigs = useMemo(
    () => [...configs].sort((a, b) => b.updatedAt - a.updatedAt),
    [configs],
  );

  const handleSubmit = (values: BalanceFormValues) => {
    const now = Date.now();

    // 解析 staticHeaders
    let staticHeaders: Record<string, string> | undefined;
    if (values.staticHeaders?.trim()) {
      try {
        staticHeaders = JSON.parse(values.staticHeaders);
      } catch {
        // 忽略 JSON 解析错误，使用空对象
        staticHeaders = {};
      }
    }

    if (editingConfig) {
      const next: BalanceConfig = {
        ...editingConfig,
        name: values.name,
        endpoint: values.endpoint,
        method: values.method,
        staticHeaders,
        extractorScript: values.extractorScript,
        intervalSec: values.intervalSec ?? 0,
        timeoutMs: values.timeoutMs,
        updatedAt: now,
      };
      updateConfig(next);
      if (values.apiKey?.trim()) {
        setApiKey(editingConfig.id, values.apiKey.trim());
      } else {
        removeApiKey(editingConfig.id);
      }
    } else {
      const id = createId();
      const config: BalanceConfig = {
        id,
        name: values.name,
        endpoint: values.endpoint,
        method: values.method,
        staticHeaders,
        extractorScript: values.extractorScript,
        intervalSec: values.intervalSec ?? 0,
        timeoutMs: values.timeoutMs,
        createdAt: now,
        updatedAt: now,
      };
      addConfig(config);
      if (values.apiKey?.trim()) {
        setApiKey(id, values.apiKey.trim());
      }
    }

    setDialogOpen(false);
    setEditingConfig(null);
  };

  const handleEdit = (config: BalanceConfig) => {
    setEditingConfig(config);
    setDialogOpen(true);
  };

  const handleDelete = (id: string) => {
    deleteConfig(id);
    removeApiKey(id);
  };

  const handleRefreshAll = async () => {
    setRefreshingAll(true);
    try {
      await refreshAll();
    } finally {
      setRefreshingAll(false);
    }
  };

  const renderContent = () => {
    if (!sortedConfigs.length) {
      return <EmptyState onAdd={() => setDialogOpen(true)} />;
    }

    return (
      <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-4">
        {sortedConfigs.map((config) => (
          <ConfigCard
            key={config.id}
            config={config}
            state={stateMap[config.id] ?? { loading: false }}
            onRefresh={refreshOne}
            onEdit={handleEdit}
            onDelete={handleDelete}
          />
        ))}
      </div>
    );
  };

  return (
    <PageContainer>
      <div className="flex flex-col gap-4">
        <div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
          <div>
            <h1 className="text-2xl font-semibold">余额监控</h1>
            <p className="text-sm text-muted-foreground">
              管理多个 API 余额配置，支持自定义提取器脚本（API Key 默认仅存内存）
            </p>
          </div>
          <div className="flex gap-2">
            <Button
              variant="outline"
              onClick={handleRefreshAll}
              disabled={!sortedConfigs.length || refreshingAll}
            >
              {refreshingAll ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  刷新中...
                </>
              ) : (
                <>
                  <RefreshCw className="mr-2 h-4 w-4" />
                  刷新全部
                </>
              )}
            </Button>
            <Button
              onClick={() => {
                setEditingConfig(null);
                setDialogOpen(true);
              }}
            >
              <Plus className="mr-2 h-4 w-4" />
              添加配置
            </Button>
          </div>
        </div>

        <Separator />

        {renderContent()}
      </div>

      <ConfigFormDialog
        open={dialogOpen}
        initial={editingConfig ?? undefined}
        onClose={() => {
          setDialogOpen(false);
          setEditingConfig(null);
        }}
        onSubmit={handleSubmit}
      />
    </PageContainer>
  );
}

export default BalancePage;
