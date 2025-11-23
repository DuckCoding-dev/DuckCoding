import { useCallback, useEffect, useRef, useState } from 'react';
import {
  checkAllUpdates,
  checkUpdate,
  type ToolStatus,
  updateTool as updateToolCommand,
} from '@/lib/tauri-commands';

export function useDashboard(initialTools: ToolStatus[]) {
  const [tools, setTools] = useState<ToolStatus[]>(initialTools);
  const [updating, setUpdating] = useState<string | null>(null);
  const [checkingUpdates, setCheckingUpdates] = useState(false);
  const [checkingSingleTool, setCheckingSingleTool] = useState<string | null>(null);
  const [updateCheckMessage, setUpdateCheckMessage] = useState<{
    type: 'success' | 'error';
    text: string;
  } | null>(null);

  const updateMessageTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  // 检查所有工具更新（顶部按钮）
  const checkForUpdates = async () => {
    try {
      setCheckingUpdates(true);
      setUpdateCheckMessage(null);

      if (updateMessageTimeoutRef.current) {
        clearTimeout(updateMessageTimeoutRef.current);
        updateMessageTimeoutRef.current = null;
      }

      const results = await checkAllUpdates();

      const updatedTools = tools.map((tool) => {
        const updateInfo = results.find((r) => r.tool_id === tool.id);
        if (updateInfo && updateInfo.success && tool.installed) {
          return {
            ...tool,
            hasUpdate: updateInfo.has_update,
            latestVersion: updateInfo.latest_version || null,
            mirrorVersion: updateInfo.mirror_version || null,
            mirrorIsStale: updateInfo.mirror_is_stale || false,
          };
        }
        return tool;
      });
      setTools(updatedTools);

      const updatesAvailable = updatedTools.filter((t) => t.hasUpdate).length;
      if (updatesAvailable > 0) {
        setUpdateCheckMessage({
          type: 'success',
          text: `发现 ${updatesAvailable} 个工具有可用更新！`,
        });
      } else {
        setUpdateCheckMessage({
          type: 'success',
          text: '所有工具均已是最新版本',
        });
      }

      updateMessageTimeoutRef.current = setTimeout(() => {
        setUpdateCheckMessage(null);
        updateMessageTimeoutRef.current = null;
      }, 5000);
    } catch (error) {
      console.error('Failed to check for updates:', error);
      setUpdateCheckMessage({
        type: 'error',
        text: '检查更新失败，请重试',
      });
      updateMessageTimeoutRef.current = setTimeout(() => {
        setUpdateCheckMessage(null);
      }, 5000);
    } finally {
      setCheckingUpdates(false);
    }
  };

  // 检查单个工具更新（工具卡片按钮）
  const checkSingleToolUpdate = async (toolId: string) => {
    try {
      setCheckingSingleTool(toolId);

      const updateInfo = await checkUpdate(toolId);

      if (updateInfo.success) {
        setTools((prevTools) =>
          prevTools.map((tool) => {
            if (tool.id === toolId && tool.installed) {
              return {
                ...tool,
                hasUpdate: updateInfo.has_update,
                latestVersion: updateInfo.latest_version || null,
                mirrorVersion: updateInfo.mirror_version || null,
                mirrorIsStale: updateInfo.mirror_is_stale || false,
              };
            }
            return tool;
          }),
        );
      }
    } catch (error) {
      console.error('Failed to check update for ' + toolId, error);
    } finally {
      setCheckingSingleTool(null);
    }
  };

  // 更新工具
  const handleUpdate = async (
    toolId: string,
  ): Promise<{ success: boolean; message: string; isUpdating?: boolean }> => {
    if (updating) {
      return {
        success: false,
        message: '已有更新任务正在进行，请等待完成后再试',
        isUpdating: true,
      };
    }

    try {
      setUpdating(toolId);
      await updateToolCommand(toolId);

      return {
        success: true,
        message: '已更新到最新版本',
      };
    } catch (error) {
      console.error('Failed to update ' + toolId, error);
      return {
        success: false,
        message: String(error),
      };
    } finally {
      setUpdating(null);
    }
  };

  // 更新tools数据（用于外部同步）
  // 智能合并：保留现有的更新检测字段，避免被外部状态覆盖
  // 但如果版本号变化，说明已更新，使用新数据
  const updateTools = useCallback((newTools: ToolStatus[]) => {
    setTools((prevTools) => {
      return newTools.map((newTool) => {
        const existingTool = prevTools.find((t) => t.id === newTool.id);

        if (existingTool) {
          // 版本号相同：保留更新检测字段（避免外部状态覆盖检查更新的结果）
          if (existingTool.version === newTool.version) {
            return {
              ...newTool,
              // 保留检查更新后设置的字段，确保类型匹配
              hasUpdate: existingTool.hasUpdate,
              latestVersion: existingTool.latestVersion,
              mirrorVersion: existingTool.mirrorVersion,
              mirrorIsStale: existingTool.mirrorIsStale,
            };
          }

          // 版本号不同：工具已更新，明确清除更新状态
          return {
            ...newTool,
            hasUpdate: false,
            latestVersion: null,
            mirrorVersion: null,
            mirrorIsStale: false,
          };
        }

        // 新工具直接使用
        return newTool;
      });
    });
  }, []); // 空依赖数组，因为使用了函数式更新

  // 组件卸载时清理定时器，避免潜在的状态更新警告
  useEffect(() => {
    return () => {
      if (updateMessageTimeoutRef.current) {
        clearTimeout(updateMessageTimeoutRef.current);
      }
    };
  }, []);

  return {
    tools,
    updating,
    checkingUpdates,
    checkingSingleTool,
    updateCheckMessage,
    checkForUpdates,
    checkSingleToolUpdate,
    handleUpdate,
    updateTools,
  };
}
