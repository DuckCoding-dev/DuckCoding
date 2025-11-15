import { useState, useCallback } from 'react';
import {
  listProfiles,
  getActiveConfig,
  type ToolStatus,
  type ActiveConfig,
} from '@/lib/tauri-commands';

/**
 * 共享配置加载 Hook
 *
 * 功能：
 * - 并行加载所有工具的配置文件列表和当前激活配置
 * - 性能优化：从串行改为并行，3个工具加载速度提升 3x
 * - 统一错误处理：加载失败时提供默认值
 *
 * @param tools 工具列表
 * @param profileTransform 可选的配置列表转换函数（如排序）
 */
export function useProfileLoader(
  tools: ToolStatus[],
  profileTransform?: (toolId: string, profiles: string[]) => string[],
) {
  const [profiles, setProfiles] = useState<Record<string, string[]>>({});
  const [activeConfigs, setActiveConfigs] = useState<Record<string, ActiveConfig>>({});

  /**
   * 并行加载所有已安装工具的配置
   */
  const loadAllProfiles = useCallback(async () => {
    const installedTools = tools.filter((t) => t.installed);
    const profileData: Record<string, string[]> = {};
    const configData: Record<string, ActiveConfig> = {};

    // 并行加载所有工具的配置，提升性能
    const results = await Promise.allSettled(
      installedTools.flatMap((tool) => [
        listProfiles(tool.id).then((profiles) => ({
          tool,
          type: 'profiles' as const,
          data: profileTransform ? profileTransform(tool.id, profiles) : profiles,
        })),
        getActiveConfig(tool.id).then((config) => ({
          tool,
          type: 'config' as const,
          data: config,
        })),
      ]),
    );

    // 处理结果
    results.forEach((result) => {
      if (result.status === 'fulfilled') {
        const { tool, type, data } = result.value;
        if (type === 'profiles') {
          profileData[tool.id] = data as string[];
        } else {
          configData[tool.id] = data as ActiveConfig;
        }
      } else {
        console.error('Failed to load config:', result.reason);
      }
    });

    // 确保所有工具都有数据（即使加载失败）
    installedTools.forEach((tool) => {
      if (!profileData[tool.id]) {
        profileData[tool.id] = [];
      }
      if (!configData[tool.id]) {
        configData[tool.id] = { api_key: '未配置', base_url: '未配置' };
      }
    });

    setProfiles(profileData);
    setActiveConfigs(configData);
  }, [tools, profileTransform]);

  return {
    profiles,
    setProfiles,
    activeConfigs,
    setActiveConfigs,
    loadAllProfiles,
  };
}
