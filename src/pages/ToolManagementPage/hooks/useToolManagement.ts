import { useState, useEffect, useCallback } from 'react';
import {
  getToolInstances,
  refreshToolInstances,
  refreshAllToolVersions,
  addWslToolInstance,
  addSshToolInstance,
  deleteToolInstance,
  checkUpdateForInstance,
  updateToolInstance,
} from '@/lib/tauri-commands';
import type { ToolInstance, SSHConfig } from '@/types/tool-management';
import { useToast } from '@/hooks/use-toast';
import { getErrorMessage } from '@/utils/error';

// 更新状态信息
interface UpdateInfo {
  hasUpdate: boolean;
  currentVersion: string | null;
  latestVersion: string | null;
}

export function useToolManagement() {
  const { toast } = useToast();
  const [groupedTools, setGroupedTools] = useState<Record<string, ToolInstance[]>>({});
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [initialized, setInitialized] = useState(false);

  // 更新状态：instanceId -> UpdateInfo
  const [updateInfoMap, setUpdateInfoMap] = useState<Record<string, UpdateInfo>>({});
  const [checkingUpdate, setCheckingUpdate] = useState<string | null>(null);
  const [updating, setUpdating] = useState<string | null>(null);

  // 加载工具实例（从数据库读取，毫秒级响应）
  const loadTools = useCallback(async () => {
    if (loading) return;

    console.log('[useToolManagement] 从数据库加载工具实例');
    try {
      setLoading(true);
      setError(null);
      const tools = await getToolInstances();
      console.log('[useToolManagement] 加载成功，工具数:', Object.keys(tools).length);
      setGroupedTools(tools);
      setInitialized(true);
    } catch (err) {
      const message = getErrorMessage(err) || '加载失败';
      console.error('[useToolManagement] 加载失败:', message);
      setError(message);
      toast({
        title: '加载工具失败',
        description: message,
        variant: 'destructive',
      });
    } finally {
      setLoading(false);
    }
  }, [loading, toast]);

  // 首次加载
  useEffect(() => {
    if (!initialized) {
      loadTools();
    }
  }, [initialized, loadTools]);

  // 刷新工具（刷新版本号并重新读取实例）
  const refreshTools = useCallback(async () => {
    try {
      setLoading(true);
      // 先刷新所有工具版本号
      await refreshAllToolVersions();
      // 再重新读取实例列表
      const tools = await refreshToolInstances();
      setGroupedTools(tools);
      toast({ title: '刷新成功' });
    } catch (err) {
      toast({
        title: '刷新失败',
        description: getErrorMessage(err),
        variant: 'destructive',
      });
    } finally {
      setLoading(false);
    }
  }, [toast]);

  // 添加实例
  const handleAddInstance = useCallback(
    async (
      baseId: string,
      type: 'local' | 'wsl' | 'ssh',
      sshConfig?: SSHConfig,
      distroName?: string,
    ) => {
      try {
        if (type === 'local') {
          // 本地实例已通过 detectSingleTool 或 addManualToolInstance 添加
          // 这里只需要重新读取数据库，不重新检测
          toast({ title: '添加成功', description: '本地工具实例已添加' });
          await loadTools(); // 只从数据库读取，不重新检测
        } else if (type === 'wsl') {
          if (!distroName) {
            throw new Error('WSL发行版名称不能为空');
          }
          await addWslToolInstance(baseId, distroName);
          toast({ title: '添加成功', description: 'WSL工具实例已添加' });
          await refreshTools();
        } else {
          if (!sshConfig) {
            throw new Error('SSH配置不能为空');
          }
          await addSshToolInstance(baseId, sshConfig);
          toast({ title: '添加成功', description: 'SSH工具实例已添加' });
          await refreshTools();
        }
      } catch (err) {
        toast({
          title: '添加失败',
          description: getErrorMessage(err),
          variant: 'destructive',
        });
      }
    },
    [refreshTools, loadTools, toast],
  );

  // 删除实例
  const handleDeleteInstance = useCallback(
    async (instanceId: string) => {
      try {
        await deleteToolInstance(instanceId);
        toast({ title: '删除成功' });
        await refreshTools();
      } catch (err) {
        toast({
          title: '删除失败',
          description: getErrorMessage(err),
          variant: 'destructive',
        });
      }
    },
    [refreshTools, toast],
  );

  // 检查更新（仅检测，不执行更新）
  const handleCheckUpdate = useCallback(
    async (instanceId: string) => {
      try {
        setCheckingUpdate(instanceId);

        // 使用基于实例的更新检测（会使用配置的路径并更新数据库）
        const result = await checkUpdateForInstance(instanceId);

        // 更新状态信息
        setUpdateInfoMap((prev) => ({
          ...prev,
          [instanceId]: {
            hasUpdate: result.has_update,
            currentVersion: result.current_version,
            latestVersion: result.latest_version,
          },
        }));

        // 从 instanceId 解析工具名称用于显示
        const parts = instanceId.split('-');
        const typeIndex = parts.findIndex((p) => ['local', 'wsl', 'ssh'].includes(p));
        const toolName = typeIndex > 0 ? parts.slice(0, typeIndex).join('-') : parts[0];

        if (result.has_update) {
          toast({
            title: '发现新版本',
            description: `${toolName}: ${result.current_version || '未知'} → ${result.latest_version || '未知'}`,
          });
        } else {
          toast({
            title: '已是最新版本',
            description: `${toolName} 当前版本: ${result.current_version || '未知'}`,
          });
        }

        // 同步更新 groupedTools 中的版本号（避免重新加载导致 Tab 跳转）
        setGroupedTools((prev) => {
          const updated = { ...prev };
          for (const [toolId, instances] of Object.entries(updated)) {
            updated[toolId] = instances.map(
              (inst): ToolInstance =>
                inst.instance_id === instanceId
                  ? { ...inst, version: result.current_version ?? inst.version }
                  : inst,
            );
          }
          return updated;
        });
      } catch (err) {
        toast({
          title: '检测失败',
          description: getErrorMessage(err),
          variant: 'destructive',
        });
      } finally {
        setCheckingUpdate(null);
      }
    },
    [toast],
  );

  // 执行更新
  const handleUpdate = useCallback(
    async (instanceId: string) => {
      // 从 instance 中解析 baseId（用于显示）
      const parts = instanceId.split('-');
      const typeIndex = parts.findIndex((p) => ['local', 'wsl', 'ssh'].includes(p));
      const baseId = typeIndex > 0 ? parts.slice(0, typeIndex).join('-') : parts[0];

      try {
        setUpdating(instanceId);

        toast({
          title: '正在更新',
          description: `正在更新 ${baseId}...`,
        });

        // 使用新的基于实例的更新命令
        const result = await updateToolInstance(instanceId);

        if (result.success) {
          toast({
            title: '更新成功',
            description: `${baseId} 已更新到 ${result.latest_version || '最新版本'}`,
          });

          // 清除更新状态
          setUpdateInfoMap((prev) => {
            const newMap = { ...prev };
            delete newMap[instanceId];
            return newMap;
          });

          // 刷新工具列表
          await refreshTools();
        } else {
          toast({
            title: '更新失败',
            description: result.message || '未知错误',
            variant: 'destructive',
          });
        }
      } catch (err) {
        toast({
          title: '更新失败',
          description: getErrorMessage(err),
          variant: 'destructive',
        });
      } finally {
        setUpdating(null);
      }
    },
    [toast, refreshTools],
  );

  return {
    groupedByTool: groupedTools,
    loading,
    error,
    refreshTools,
    handleAddInstance,
    handleDeleteInstance,
    handleCheckUpdate,
    handleUpdate,
    updateInfoMap,
    checkingUpdate,
    updating,
  };
}
