/**
 * Profile 管理状态 Hook
 */

import { useState, useEffect, useCallback } from 'react';
import { useToast } from '@/hooks/use-toast';
import type { ProfileFormData, ProfileGroup, ToolId, ProfilePayload } from '@/types/profile';
import {
  pmListAllProfiles,
  pmSaveProfile,
  pmDeleteProfile,
  pmActivateProfile,
  pmCaptureFromNative,
  getAllProxyStatus,
  type AllProxyStatus,
} from '@/lib/tauri-commands';
import { TOOL_NAMES } from '@/types/profile';

interface UseProfileManagementReturn {
  // 状态
  profileGroups: ProfileGroup[];
  loading: boolean;
  error: string | null;
  allProxyStatus: AllProxyStatus;

  // 操作方法
  refresh: () => Promise<void>;
  loadAllProxyStatus: () => Promise<void>;
  createProfile: (toolId: ToolId, data: ProfileFormData) => Promise<void>;
  updateProfile: (toolId: ToolId, name: string, data: ProfileFormData) => Promise<void>;
  deleteProfile: (toolId: ToolId, name: string) => Promise<void>;
  activateProfile: (toolId: ToolId, name: string) => Promise<void>;
  captureFromNative: (toolId: ToolId, name: string) => Promise<void>;
}

export function useProfileManagement(): UseProfileManagementReturn {
  const { toast } = useToast();
  const [profileGroups, setProfileGroups] = useState<ProfileGroup[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [allProxyStatus, setAllProxyStatus] = useState<AllProxyStatus>({});

  // 加载所有 Profile
  const loadProfiles = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const allProfiles = await pmListAllProfiles();

      // 按工具分组
      const groups: ProfileGroup[] = (['claude-code', 'codex', 'gemini-cli'] as ToolId[]).map(
        (toolId) => {
          const toolProfiles = allProfiles.filter((p) => p.tool_id === toolId);
          const activeProfile = toolProfiles.find((p) => p.is_active);

          return {
            tool_id: toolId,
            tool_name: TOOL_NAMES[toolId],
            profiles: toolProfiles,
            active_profile: activeProfile,
          };
        },
      );

      setProfileGroups(groups);
    } catch (err) {
      const message = err instanceof Error ? err.message : '加载 Profile 失败';
      setError(message);
      toast({
        title: '加载失败',
        description: message,
        variant: 'destructive',
      });
    } finally {
      setLoading(false);
    }
  }, [toast]);

  // 刷新
  const refresh = useCallback(async () => {
    await loadProfiles();
  }, [loadProfiles]);

  // 加载所有透明代理状态
  const loadAllProxyStatus = useCallback(async () => {
    try {
      const status = await getAllProxyStatus();
      setAllProxyStatus(status);
    } catch (error) {
      console.error('Failed to load proxy status:', error);
    }
  }, []);

  // 创建 Profile
  const createProfile = useCallback(
    async (toolId: ToolId, data: ProfileFormData) => {
      try {
        const payload = buildProfilePayload(toolId, data);
        await pmSaveProfile(toolId, data.name, payload);
        toast({
          title: '创建成功',
          description: `Profile "${data.name}" 创建成功`,
        });
        // 不要立即刷新，等对话框关闭后由父组件刷新
      } catch (err) {
        const message = err instanceof Error ? err.message : '创建 Profile 失败';
        toast({
          title: '创建失败',
          description: message,
          variant: 'destructive',
        });
        throw err;
      }
    },
    [toast],
  );

  // 更新 Profile
  const updateProfile = useCallback(
    async (toolId: ToolId, name: string, data: ProfileFormData) => {
      try {
        const payload = buildProfilePayload(toolId, data);
        await pmSaveProfile(toolId, name, payload);
        toast({
          title: '更新成功',
          description: `Profile "${name}" 更新成功`,
        });
        // 不要立即刷新，等对话框关闭后由父组件刷新
      } catch (err) {
        const message = err instanceof Error ? err.message : '更新 Profile 失败';
        toast({
          title: '更新失败',
          description: message,
          variant: 'destructive',
        });
        throw err;
      }
    },
    [toast],
  );

  // 删除 Profile
  const deleteProfile = useCallback(
    async (toolId: ToolId, name: string) => {
      try {
        await pmDeleteProfile(toolId, name);
        toast({
          title: '删除成功',
          description: `Profile "${name}" 已删除`,
        });
        await refresh();
      } catch (err) {
        const message = err instanceof Error ? err.message : '删除 Profile 失败';
        toast({
          title: '删除失败',
          description: message,
          variant: 'destructive',
        });
        throw err;
      }
    },
    [refresh, toast],
  );

  // 激活 Profile
  const activateProfile = useCallback(
    async (toolId: ToolId, name: string) => {
      try {
        await pmActivateProfile(toolId, name);
        toast({
          title: '激活成功',
          description: `已切换到 Profile "${name}"`,
        });
        await refresh();
      } catch (err) {
        const message = err instanceof Error ? err.message : '激活 Profile 失败';
        toast({
          title: '激活失败',
          description: message,
          variant: 'destructive',
        });
        throw err;
      }
    },
    [refresh, toast],
  );

  // 从原生配置捕获
  const captureFromNative = useCallback(
    async (toolId: ToolId, name: string) => {
      try {
        await pmCaptureFromNative(toolId, name);
        toast({
          title: '捕获成功',
          description: `已从原生配置捕获到 Profile "${name}"`,
        });
        await refresh();
      } catch (err) {
        const message = err instanceof Error ? err.message : '捕获原生配置失败';
        toast({
          title: '捕获失败',
          description: message,
          variant: 'destructive',
        });
        throw err;
      }
    },
    [refresh, toast],
  );

  // 初始加载
  useEffect(() => {
    loadProfiles();
  }, [loadProfiles]);

  return {
    profileGroups,
    loading,
    error,
    allProxyStatus,
    refresh,
    loadAllProxyStatus,
    createProfile,
    updateProfile,
    deleteProfile,
    activateProfile,
    captureFromNative,
  };
}

// ==================== 辅助函数 ====================

/**
 * 构建 ProfilePayload（工具分组即类型，无需 type 字段）
 */
function buildProfilePayload(toolId: ToolId, data: ProfileFormData): ProfilePayload {
  switch (toolId) {
    case 'claude-code':
      return {
        api_key: data.api_key,
        base_url: data.base_url,
      };

    case 'codex':
      return {
        api_key: data.api_key,
        base_url: data.base_url,
        wire_api: data.wire_api || 'responses', // 确保有 wire_api
      };

    case 'gemini-cli':
      return {
        api_key: data.api_key,
        base_url: data.base_url,
        model: data.model && data.model !== '' ? data.model : undefined, // 空值不设置 model
      };

    default:
      throw new Error(`不支持的工具 ID: ${toolId}`);
  }
}
