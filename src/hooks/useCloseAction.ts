import { useState, useCallback } from 'react';
import { applyCloseAction, type CloseAction } from '@/lib/tauri-commands';

const CLOSE_PREFERENCE_KEY = 'duckcoding.closePreference';

const isTauriEnvironment = () => {
  if (typeof window === 'undefined') {
    return false;
  }

  const globalWindow = window as unknown as Record<string, unknown>;
  return Boolean(
    globalWindow.__TAURI_INTERNALS__ ??
      globalWindow.__TAURI_METADATA__ ??
      globalWindow.__TAURI_IPC__,
  );
};

export function useCloseAction(onError: (message: string) => void) {
  const [closeDialogOpen, setCloseDialogOpen] = useState(false);
  const [rememberCloseChoice, setRememberCloseChoice] = useState(false);
  const [closeActionLoading, setCloseActionLoading] = useState<CloseAction | null>(null);

  // 执行窗口关闭动作
  const executeCloseAction = useCallback(
    async (action: CloseAction, remember = false, autoTriggered = false) => {
      if (!isTauriEnvironment()) {
        setCloseDialogOpen(false);
        return;
      }

      setCloseActionLoading(action);
      try {
        await applyCloseAction(action);

        if (typeof window !== 'undefined') {
          try {
            if (remember) {
              window.localStorage.setItem(CLOSE_PREFERENCE_KEY, action);
            } else if (!autoTriggered) {
              window.localStorage.removeItem(CLOSE_PREFERENCE_KEY);
            }
          } catch (storageError) {
            console.warn('保存关闭偏好失败:', storageError);
          }
        }
      } catch (error) {
        console.error('执行窗口操作失败:', error);
        onError(error instanceof Error ? error.message : '请稍后重试，或从系统托盘退出/展开窗口');

        if (!autoTriggered) {
          setCloseDialogOpen(true);
        }
      } finally {
        setCloseActionLoading(null);
        if (!autoTriggered) {
          setCloseDialogOpen(false);
          setRememberCloseChoice(false);
        }
      }
    },
    [onError],
  );

  // 打开关闭对话框
  const openCloseDialog = useCallback(() => {
    setCloseDialogOpen(true);
  }, []);

  // 关闭对话框
  const closeDialog = useCallback(() => {
    setCloseDialogOpen(false);
    setRememberCloseChoice(false);
  }, []);

  return {
    closeDialogOpen,
    rememberCloseChoice,
    closeActionLoading,
    setRememberCloseChoice,
    executeCloseAction,
    openCloseDialog,
    closeDialog,
  };
}
