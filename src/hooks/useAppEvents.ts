import { useEffect } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { CloseAction } from '@/lib/tauri-commands';

const CLOSE_EVENT = 'duckcoding://request-close-action';
const CLOSE_PREFERENCE_KEY = 'duckcoding.closePreference';
const SINGLE_INSTANCE_EVENT = 'single-instance';

interface SingleInstancePayload {
  args: string[];
  cwd: string;
}

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

interface AppEventsOptions {
  onCloseRequest: () => void;
  onSingleInstance: (message: string) => void;
  onNavigateToInstall: () => void;
  onNavigateToList: () => void;
  onNavigateToConfig: (detail?: { toolId?: string }) => void;
  onNavigateToSettings: (detail?: { tab?: string }) => void;
  onNavigateToTransparentProxy: (detail?: { toolId?: string }) => void;
  onRefreshTools: () => void;
  executeCloseAction: (action: CloseAction, remember: boolean, autoTriggered: boolean) => void;
}

export function useAppEvents(options: AppEventsOptions) {
  const {
    onCloseRequest,
    onSingleInstance,
    onNavigateToInstall,
    onNavigateToList,
    onNavigateToConfig,
    onNavigateToSettings,
    onNavigateToTransparentProxy,
    onRefreshTools,
    executeCloseAction,
  } = options;

  // 监听窗口关闭事件
  useEffect(() => {
    if (!isTauriEnvironment()) {
      return;
    }

    let unlisten: UnlistenFn | null = null;
    let disposed = false;

    listen(CLOSE_EVENT, () => {
      if (typeof window !== 'undefined') {
        try {
          const savedPreference = window.localStorage.getItem(
            CLOSE_PREFERENCE_KEY,
          ) as CloseAction | null;

          if (savedPreference === 'minimize' || savedPreference === 'quit') {
            executeCloseAction(savedPreference, true, true);
            return;
          }
        } catch (storageError) {
          console.warn('读取关闭偏好失败:', storageError);
        }
      }

      onCloseRequest();
    })
      .then((fn) => {
        if (disposed) {
          fn();
        } else {
          unlisten = fn;
        }
      })
      .catch((error) => {
        console.error('注册关闭事件监听失败:', error);
      });

    return () => {
      disposed = true;
      if (unlisten) {
        unlisten();
      }
    };
  }, [onCloseRequest, executeCloseAction]);

  // 监听单例应用事件
  useEffect(() => {
    if (!isTauriEnvironment()) {
      return;
    }

    let unlisten: UnlistenFn | null = null;
    let disposed = false;

    listen<SingleInstancePayload>(SINGLE_INSTANCE_EVENT, (event) => {
      const args = event.payload?.args?.slice(1).join(' ') ?? '';
      const message = args
        ? `已切换到当前实例（参数：${args}）`
        : '检测到重复启动，已切换到当前实例。';
      onSingleInstance(message);
    })
      .then((fn) => {
        if (disposed) {
          fn();
        } else {
          unlisten = fn;
        }
      })
      .catch((error) => {
        console.error('注册 single-instance 事件监听失败:', error);
      });

    return () => {
      disposed = true;
      if (unlisten) {
        unlisten();
      }
    };
  }, [onSingleInstance]);

  // 监听页面导航事件
  useEffect(() => {
    const handleNavigateToConfig = (event: Event) => {
      const customEvent = event as CustomEvent<{ toolId?: string }>;
      onNavigateToConfig(customEvent.detail);
    };

    const handleNavigateToTransparentProxy = (event: Event) => {
      const customEvent = event as CustomEvent<{ toolId?: string }>;
      onNavigateToTransparentProxy(customEvent.detail);
    };

    const handleNavigateToSettings = (event: Event) => {
      const customEvent = event as CustomEvent<{ tab?: string }>;
      onNavigateToSettings(customEvent.detail);
    };

    window.addEventListener('navigate-to-install', onNavigateToInstall);
    window.addEventListener('navigate-to-config', handleNavigateToConfig);
    window.addEventListener('navigate-to-list', onNavigateToList);
    window.addEventListener('navigate-to-settings', handleNavigateToSettings);
    window.addEventListener('navigate-to-transparent-proxy', handleNavigateToTransparentProxy);
    window.addEventListener('refresh-tools', onRefreshTools);

    return () => {
      window.removeEventListener('navigate-to-install', onNavigateToInstall);
      window.removeEventListener('navigate-to-config', handleNavigateToConfig);
      window.removeEventListener('navigate-to-list', onNavigateToList);
      window.removeEventListener('navigate-to-settings', handleNavigateToSettings);
      window.removeEventListener('navigate-to-transparent-proxy', handleNavigateToTransparentProxy);
      window.removeEventListener('refresh-tools', onRefreshTools);
    };
  }, [
    onNavigateToInstall,
    onNavigateToConfig,
    onNavigateToSettings,
    onNavigateToTransparentProxy,
    onRefreshTools,
  ]);
}
