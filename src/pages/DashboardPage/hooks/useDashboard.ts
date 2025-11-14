import { useState, useRef } from 'react';
import {
  updateTool as updateToolCommand,
  checkAllUpdates,
  type ToolStatus,
} from '@/lib/tauri-commands';

export function useDashboard(initialTools: ToolStatus[]) {
  const [tools, setTools] = useState<ToolStatus[]>(initialTools);
  const [updating, setUpdating] = useState<string | null>(null);
  const [checkingUpdates, setCheckingUpdates] = useState(false);
  const [updateCheckMessage, setUpdateCheckMessage] = useState<{
    type: 'success' | 'error';
    text: string;
  } | null>(null);

  const updateMessageTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  // 检查更新
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
            latestVersion: updateInfo.latest_version || undefined,
            mirrorVersion: updateInfo.mirror_version || undefined,
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
  const updateTools = (newTools: ToolStatus[]) => {
    setTools(newTools);
  };

  return {
    tools,
    updating,
    checkingUpdates,
    updateCheckMessage,
    checkForUpdates,
    handleUpdate,
    updateTools,
  };
}
