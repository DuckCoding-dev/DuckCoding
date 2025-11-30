import { useState, useEffect, useCallback } from 'react';
import {
  getToolInstances,
  refreshToolInstances,
  addWslToolInstance,
  addSshToolInstance,
  deleteToolInstance,
  checkUpdate,
  updateTool,
} from '@/lib/tauri-commands';
import type { ToolInstance, SSHConfig } from '@/types/tool-management';
import { useToast } from '@/hooks/use-toast';

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
      const message = err instanceof Error ? err.message : '加载失败';
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

  // 刷新工具（重新检测并更新数据库）
  const refreshTools = useCallback(async () => {
    try {
      setLoading(true);
      const tools = await refreshToolInstances();
      setGroupedTools(tools);
      toast({ title: '刷新成功' });
    } catch (err) {
      toast({
        title: '刷新失败',
        description: String(err),
        variant: 'destructive',
      });
    } finally {
      setLoading(false);
    }
  }, [toast]);

  // 添加实例
  const handleAddInstance = useCallback(
    async (baseId: string, type: 'wsl' | 'ssh', sshConfig?: SSHConfig, distroName?: string) => {
      try {
        if (type === 'wsl') {
          if (!distroName) {
            throw new Error('WSL发行版名称不能为空');
          }
          await addWslToolInstance(baseId, distroName);
          toast({ title: '添加成功', description: 'WSL工具实例已添加' });
        } else {
          if (!sshConfig) {
            throw new Error('SSH配置不能为空');
          }
          await addSshToolInstance(baseId, sshConfig);
          toast({ title: '添加成功', description: 'SSH工具实例已添加' });
        }
        await refreshTools();
      } catch (err) {
        toast({
          title: '添加失败',
          description: String(err),
          variant: 'destructive',
        });
      }
    },
    [refreshTools, toast],
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
          description: String(err),
          variant: 'destructive',
        });
      }
    },
    [refreshTools, toast],
  );

  // 检查更新（仅检测，不执行更新）
  const handleCheckUpdate = useCallback(
    async (instanceId: string) => {
      // 从 instance 中解析 baseId
      // 格式: claude-code-local, codex-wsl-ubuntu, gemini-cli-ssh-dev
      const parts = instanceId.split('-');
      // 找到类型标识符的位置 (local, wsl, ssh)
      const typeIndex = parts.findIndex((p) => ['local', 'wsl', 'ssh'].includes(p));
      const baseId = typeIndex > 0 ? parts.slice(0, typeIndex).join('-') : parts[0];

      try {
        setCheckingUpdate(instanceId);

        const result = await checkUpdate(baseId);

        // 更新状态信息
        setUpdateInfoMap((prev) => ({
          ...prev,
          [instanceId]: {
            hasUpdate: result.has_update,
            currentVersion: result.current_version,
            latestVersion: result.latest_version,
          },
        }));

        if (result.has_update) {
          toast({
            title: '发现新版本',
            description: `${baseId}: ${result.current_version || '未知'} → ${result.latest_version || '未知'}`,
          });
        } else {
          toast({
            title: '已是最新版本',
            description: `${baseId} 当前版本: ${result.current_version || '未知'}`,
          });
        }
      } catch (err) {
        toast({
          title: '检测失败',
          description: String(err),
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
      // 从 instance 中解析 baseId
      const parts = instanceId.split('-');
      const typeIndex = parts.findIndex((p) => ['local', 'wsl', 'ssh'].includes(p));
      const baseId = typeIndex > 0 ? parts.slice(0, typeIndex).join('-') : parts[0];

      try {
        setUpdating(instanceId);

        toast({
          title: '正在更新',
          description: `正在更新 ${baseId}...`,
        });

        const result = await updateTool(baseId);

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
          description: String(err),
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
