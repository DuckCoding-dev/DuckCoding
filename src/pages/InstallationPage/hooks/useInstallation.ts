import { useState, useEffect, useCallback } from 'react';
import {
  checkNodeEnvironment,
  installTool as installToolCommand,
  type ToolStatus,
} from '@/lib/tauri-commands';
import type { NodeEnvironment } from '@/components/dialogs/MirrorStaleDialog';

export function useInstallation(tools: ToolStatus[]) {
  const [installing, setInstalling] = useState<string | null>(null);
  const [nodeEnv, setNodeEnv] = useState<NodeEnvironment | null>(null);
  const [installMethods, setInstallMethods] = useState<Record<string, string>>({
    'claude-code': 'official',
    codex: navigator.userAgent.includes('Mac') ? 'brew' : 'npm',
    'gemini-cli': 'npm',
  });
  const [mirrorStaleDialog, setMirrorStaleDialog] = useState({
    open: false,
    toolId: '',
    mirrorVersion: '',
    officialVersion: '',
    source: 'install' as 'install' | 'update',
  });

  // 加载 Node 环境信息
  const loadNodeEnv = useCallback(async () => {
    try {
      const env = await checkNodeEnvironment();
      setNodeEnv(env);
    } catch (error) {
      console.error('Failed to load node environment:', error);
    }
  }, []);

  // 初始加载
  useEffect(() => {
    loadNodeEnv();
  }, [loadNodeEnv]);

  // 获取可用的安装方法
  const getAvailableInstallMethods = useCallback(
    (toolId: string): Array<{ value: string; label: string; disabled?: boolean }> => {
      const isMac = navigator.userAgent.includes('Mac');

      if (toolId === 'claude-code') {
        return [
          { value: 'official', label: '官方脚本 (推荐)' },
          { value: 'npm', label: 'npm 安装', disabled: !nodeEnv?.npm_available },
        ];
      } else if (toolId === 'codex') {
        const methods = [{ value: 'npm', label: 'npm 安装', disabled: !nodeEnv?.npm_available }];
        if (isMac) {
          methods.unshift({ value: 'brew', label: 'Homebrew (推荐)', disabled: false });
        }
        return methods;
      } else if (toolId === 'gemini-cli') {
        return [{ value: 'npm', label: 'npm 安装 (推荐)', disabled: !nodeEnv?.npm_available }];
      }
      return [];
    },
    [nodeEnv],
  );

  // 安装工具
  const handleInstall = useCallback(
    async (
      toolId: string,
    ): Promise<{ success: boolean; message: string; mirrorStale?: boolean }> => {
      try {
        setInstalling(toolId);
        const method = installMethods[toolId] || 'official';
        console.log(`Installing ${toolId} using method: ${method}`);
        await installToolCommand(toolId, method);

        return {
          success: true,
          message: `${toolId} 已成功安装`,
        };
      } catch (error) {
        console.error('Failed to install ' + toolId, error);
        const errorMsg = String(error);

        // 检查是否是镜像滞后错误
        if (errorMsg.includes('MIRROR_STALE')) {
          const parts = errorMsg.split('|');
          if (parts.length === 3) {
            const mirrorVer = parts[1];
            const officialVer = parts[2];

            // 显示镜像滞后对话框
            setMirrorStaleDialog({
              open: true,
              toolId: toolId,
              mirrorVersion: mirrorVer,
              officialVersion: officialVer,
              source: 'install',
            });

            return {
              success: false,
              message: '镜像版本滞后',
              mirrorStale: true,
            };
          }
        }

        return {
          success: false,
          message: String(error),
        };
      } finally {
        setInstalling(null);
      }
    },
    [installMethods],
  );

  // 处理镜像滞后对话框 - 继续使用镜像
  const handleContinueMirror = useCallback(
    async (
      toolId: string,
      source: 'install' | 'update',
      mirrorVersion: string,
    ): Promise<{ success: boolean; message: string }> => {
      setMirrorStaleDialog({
        open: false,
        toolId: '',
        mirrorVersion: '',
        officialVersion: '',
        source: 'install',
      });

      try {
        setInstalling(toolId);
        const method = installMethods[toolId] || 'official';
        await installToolCommand(toolId, method, true); // force=true

        return {
          success: true,
          message: `已安装镜像版本 ${mirrorVersion}`,
        };
      } catch (error) {
        console.error('Failed to force install', error);
        return {
          success: false,
          message: String(error),
        };
      } finally {
        setInstalling(null);
      }
    },
    [installMethods],
  );

  // 处理镜像滞后对话框 - 改用 npm
  const handleUseNpm = useCallback(
    async (toolId: string, officialVersion: string): Promise<{ success: boolean; message: string }> => {
      setMirrorStaleDialog({
        open: false,
        toolId: '',
        mirrorVersion: '',
        officialVersion: '',
        source: 'install',
      });

      // 改用 npm 安装
      setInstallMethods((prev) => ({ ...prev, [toolId]: 'npm' }));

      // 重新触发安装
      try {
        setInstalling(toolId);
        await installToolCommand(toolId, 'npm');

        return {
          success: true,
          message: `已获取最新版本 ${officialVersion}`,
        };
      } catch (error) {
        console.error('Failed to install with npm', error);
        return {
          success: false,
          message: String(error),
        };
      } finally {
        setInstalling(null);
      }
    },
    [],
  );

  // 关闭镜像滞后对话框
  const closeMirrorDialog = useCallback(() => {
    setMirrorStaleDialog({
      open: false,
      toolId: '',
      mirrorVersion: '',
      officialVersion: '',
      source: 'install',
    });
  }, []);

  return {
    installing,
    nodeEnv,
    installMethods,
    setInstallMethods,
    mirrorStaleDialog,
    getAvailableInstallMethods,
    handleInstall,
    handleContinueMirror,
    handleUseNpm,
    closeMirrorDialog,
  };
}
