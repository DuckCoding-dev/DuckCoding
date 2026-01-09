// Claude Code 代理内容组件
// 显示代理请求历史记录表格

import { useState, useEffect, useCallback } from 'react';
import { emit } from '@tauri-apps/api/event';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import {
  FileText,
  Trash2,
  Loader2,
  Settings,
  Pencil,
  HelpCircle,
  X,
  Info,
  ExternalLink,
  History,
} from 'lucide-react';
import { useSessionData } from '../../hooks/useSessionData';
import { SessionConfigDialog } from '../SessionConfigDialog';
import { SessionNoteDialog } from '../SessionNoteDialog';
import {
  getProxyConfig,
  getGlobalConfig,
  saveGlobalConfig,
  type SessionRecord,
} from '@/lib/tauri-commands';
import { isActiveSession } from '@/utils/sessionHelpers';
import { useToast } from '@/hooks/use-toast';

/**
 * 渲染配置显示内容
 * - global: 显示 "跟随主配置"
 * - custom: 显示自定义配置名称
 */
function ConfigBadge({ session }: { session: SessionRecord }) {
  if (session.config_name === 'global') {
    return (
      <span className="inline-flex items-center px-2 py-1 rounded-full bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-200 text-xs font-medium">
        跟随主配置
      </span>
    );
  }

  // custom 配置：显示配置名称
  const displayName = session.custom_profile_name || '自定义';
  return (
    <span className="inline-flex items-center px-2 py-1 rounded-full bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-200 text-xs font-medium">
      {displayName}
    </span>
  );
}

/**
 * 帮助弹窗组件
 */
function HelpDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>会话级端点配置说明</DialogTitle>
          <DialogDescription>了解如何为不同会话配置独立的 API 端点</DialogDescription>
        </DialogHeader>
        <div className="space-y-4 text-sm">
          <div className="space-y-2">
            <h4 className="font-medium">什么是会话级端点配置？</h4>
            <p className="text-muted-foreground">
              会话级端点配置允许您为每个 Claude Code 会话单独设置 API
              端点。这意味着不同的终端窗口可以使用不同的 API 配置，非常适合：
            </p>
            <ul className="list-disc list-inside text-muted-foreground ml-2 space-y-1">
              <li>多账户切换：不同项目使用不同账户</li>
              <li>测试与生产分离：开发环境和生产环境使用不同配置</li>
              <li>团队协作：团队成员使用各自的配置</li>
            </ul>
          </div>

          <div className="space-y-2">
            <h4 className="font-medium">如何使用？</h4>
            <ol className="list-decimal list-inside text-muted-foreground ml-2 space-y-1">
              <li>在设置页面开启「会话级端点配置」</li>
              <li>启动 Claude Code 透明代理</li>
              <li>在新的终端中启动 Claude Code</li>
              <li>回到此页面，找到对应的会话记录</li>
              <li>点击设置按钮（齿轮图标）切换配置</li>
              <li>选择要使用的配置文件并保存</li>
            </ol>
          </div>

          <div className="space-y-2">
            <h4 className="font-medium">配置说明</h4>
            <ul className="list-disc list-inside text-muted-foreground ml-2 space-y-1">
              <li>
                <strong>跟随主配置</strong>：使用透明代理的全局配置
              </li>
              <li>
                <strong>自定义配置</strong>：使用指定的配置文件，独立于主配置
              </li>
            </ul>
          </div>

          <div className="space-y-2">
            <h4 className="font-medium">使用说明</h4>
            <ul className="list-disc list-inside text-muted-foreground ml-2 space-y-1">
              <li>1.在正确配置代理后启动ClaudeCode ，会话记录会自动显示在此处。</li>
              <li>
                1-1.如果没有显示，请尝试在ClaudeCode命令会话框中输入任意文本并发送以触发请求。
              </li>
              <li>2.重启ClaudeCode后会话ID变化 需要重新选择并配置</li>
              <li>3.运行/clear命令后会话ID变化 需要重新选择并配置</li>
            </ul>
          </div>

          <div className="p-3 bg-muted rounded-lg">
            <p className="text-xs text-muted-foreground">
              <strong>提示：</strong>您可以为会话添加备注（点击铅笔图标），方便识别不同的会话用途。
            </p>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

/**
 * 未开启会话级端点配置时的提示组件
 */
function DisabledHint({
  onNavigateToSettings,
  onDismiss,
  onClose,
  dismissed,
  closed,
}: {
  onNavigateToSettings: () => void;
  onDismiss: () => void;
  onClose: () => void;
  dismissed: boolean;
  closed: boolean;
}) {
  const showHint = !dismissed && !closed;

  return (
    <div className="space-y-4">
      {showHint && (
        <Alert className="relative">
          <Info className="h-4 w-4" />
          <AlertTitle>会话级端点配置暂未开启</AlertTitle>
          <AlertDescription className="mt-2">
            <p className="mb-3">
              开启后可为每个会话单独配置 API 端点，支持多账户、多配置灵活切换。
            </p>
            <div className="flex items-center gap-2">
              <Button size="sm" variant="default" onClick={onNavigateToSettings}>
                <ExternalLink className="h-3 w-3 mr-1" />
                前往设置开启
              </Button>
              <Button size="sm" variant="ghost" onClick={onDismiss}>
                不再显示
              </Button>
            </div>
          </AlertDescription>
          <Button
            variant="ghost"
            size="sm"
            className="absolute top-2 right-2 h-6 w-6 p-0"
            onClick={onClose}
          >
            <X className="h-3 w-3" />
          </Button>
        </Alert>
      )}

      <div className="flex flex-col items-center justify-center py-16 text-center">
        <FileText className="h-12 w-12 text-muted-foreground mb-4" />
        <h3 className="text-lg font-semibold mb-2">会话级端点配置未开启</h3>
        <p className="text-sm text-muted-foreground max-w-md">
          开启会话级端点配置后，可在此查看和管理 Claude Code 代理会话。
        </p>
      </div>
    </div>
  );
}

/**
 * Claude Code 代理内容组件
 *
 * 功能：
 * - 展示代理请求历史记录表格
 * - 列：会话标识符 | 会话启动时间 | 请求次数 | 目前使用配置 | 操作
 * - 支持切换会话配置
 * - 支持编辑会话备注
 * - 支持删除单个会话
 * - 支持查看会话日志
 * - 自动定时轮询更新（5 秒间隔）
 */
export function ClaudeContent() {
  const { toast } = useToast();
  const [configDialogOpen, setConfigDialogOpen] = useState(false);
  const [noteDialogOpen, setNoteDialogOpen] = useState(false);
  const [helpDialogOpen, setHelpDialogOpen] = useState(false);
  const [selectedSession, setSelectedSession] = useState<SessionRecord | null>(null);

  // 会话级端点配置状态
  const [sessionEndpointEnabled, setSessionEndpointEnabled] = useState<boolean | null>(null);
  const [hintDismissed, setHintDismissed] = useState(false);
  const [hintClosed, setHintClosed] = useState(false);

  // 导航到 Token 统计页面并筛选该会话的日志
  const handleViewLogs = useCallback(
    async (sessionId: string) => {
      try {
        await emit('app-navigate', {
          tab: 'token-statistics',
          params: { sessionId, toolType: 'claude-code' },
        });
      } catch (error) {
        console.error('导航失败:', error);
        toast({
          title: '导航失败',
          description: '无法打开统计页面',
          variant: 'destructive',
        });
      }
    },
    [toast],
  );

  // 打开代理设置弹窗
  const openProxySettings = useCallback(() => {
    window.dispatchEvent(new CustomEvent('open-proxy-settings', { detail: 'claude-code' }));
  }, []);

  // 加载配置状态
  const loadConfig = useCallback(() => {
    Promise.all([getProxyConfig('claude-code'), getGlobalConfig()])
      .then(([proxyConfig, globalConfig]) => {
        // 从 ProxyConfigManager 读取会话端点配置开关
        setSessionEndpointEnabled(proxyConfig?.session_endpoint_config_enabled ?? false);
        // 读取是否已隐藏提示
        setHintDismissed(globalConfig?.hide_session_config_hint ?? false);
      })
      .catch(() => {
        setSessionEndpointEnabled(false);
        setHintDismissed(false);
      });
  }, []);

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  // 监听配置更新事件
  useEffect(() => {
    const handleConfigUpdate = () => {
      loadConfig();
    };
    window.addEventListener('proxy-config-updated', handleConfigUpdate);
    return () => {
      window.removeEventListener('proxy-config-updated', handleConfigUpdate);
    };
  }, [loadConfig]);

  // 处理不再显示
  const handleDismiss = useCallback(async () => {
    try {
      const config = await getGlobalConfig();
      if (!config) return;
      await saveGlobalConfig({
        ...config,
        hide_session_config_hint: true,
      });
      setHintDismissed(true);
    } catch (error) {
      console.error('保存提示设置失败:', error);
    }
  }, []);

  // 处理关闭提示
  const handleClose = useCallback(() => {
    setHintClosed(true);
  }, []);

  // 仅在开启功能时加载会话数据
  const { sessions, loading, deleteSession, refresh } = useSessionData(
    sessionEndpointEnabled ? 'claude-code' : null,
  );

  // 配置加载中
  if (sessionEndpointEnabled === null) {
    return (
      <div className="flex flex-col items-center justify-center py-16 text-center">
        <Loader2 className="h-12 w-12 text-primary animate-spin mb-4" />
        <p className="text-sm text-muted-foreground">加载配置中...</p>
      </div>
    );
  }

  // 功能未开启：显示提示
  if (!sessionEndpointEnabled) {
    return (
      <DisabledHint
        onNavigateToSettings={openProxySettings}
        onDismiss={handleDismiss}
        onClose={handleClose}
        dismissed={hintDismissed}
        closed={hintClosed}
      />
    );
  }

  // 功能已开启：加载会话数据
  if (loading && sessions.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16 text-center">
        <Loader2 className="h-12 w-12 text-primary animate-spin mb-4" />
        <p className="text-sm text-muted-foreground">加载会话记录中...</p>
      </div>
    );
  }

  // 空状态展示
  if (sessions.length === 0) {
    return (
      <div className="space-y-4">
        <div className="flex justify-end">
          <Button variant="outline" size="sm" onClick={() => setHelpDialogOpen(true)}>
            <HelpCircle className="h-3 w-3 mr-1" />
            帮助
          </Button>
        </div>

        <div className="flex flex-col items-center justify-center py-16 text-center">
          <FileText className="h-12 w-12 text-muted-foreground mb-4" />
          <h3 className="text-lg font-semibold mb-2">暂无代理会话记录</h3>
          <p className="text-sm text-muted-foreground max-w-md">
            启动代理后，Claude Code 的请求会话记录将显示在此处。
          </p>
        </div>

        <HelpDialog open={helpDialogOpen} onOpenChange={setHelpDialogOpen} />
      </div>
    );
  }

  // 表格展示
  return (
    <div className="space-y-4">
      <div className="flex justify-end">
        <Button variant="outline" size="sm" onClick={() => setHelpDialogOpen(true)}>
          <HelpCircle className="h-3 w-3 mr-1" />
          帮助
        </Button>
      </div>

      <div className="rounded-lg border overflow-hidden">
        <table className="w-full">
          <thead className="bg-muted/50">
            <tr>
              <th className="px-4 py-3 text-left text-sm font-medium text-muted-foreground w-[220px]">
                会话标识符
              </th>
              <th className="px-4 py-3 text-left text-sm font-medium text-muted-foreground w-[100px]">
                状态
              </th>
              <th className="px-4 py-3 text-left text-sm font-medium text-muted-foreground w-[180px]">
                会话启动时间
              </th>
              <th className="px-4 py-3 text-left text-sm font-medium text-muted-foreground w-[120px]">
                请求次数
              </th>
              <th className="px-4 py-3 text-left text-sm font-medium text-muted-foreground">
                目前使用配置
              </th>
              <th className="px-4 py-3 text-right text-sm font-medium text-muted-foreground w-[120px]">
                操作
              </th>
            </tr>
          </thead>
          <tbody>
            {sessions.map((session) => (
              <tr key={session.session_id} className="border-t hover:bg-muted/30 transition-colors">
                <td className="px-4 py-3 text-sm">
                  <div className="flex items-center gap-2">
                    <span className="font-semibold">{session.note || '未命名'}</span>
                    <Badge variant="outline" className="font-mono text-xs">
                      {session.display_id.slice(0, 8)}
                    </Badge>
                  </div>
                </td>
                <td className="px-4 py-3 text-sm">
                  {isActiveSession(session.last_seen_at) ? (
                    <Badge variant="default" className="bg-green-500 hover:bg-green-600">
                      活跃
                    </Badge>
                  ) : (
                    <Badge variant="secondary" className="text-muted-foreground">
                      空闲
                    </Badge>
                  )}
                </td>
                <td className="px-4 py-3 text-sm text-muted-foreground">
                  {new Date(session.first_seen_at * 1000).toLocaleString('zh-CN')}
                </td>
                <td className="px-4 py-3 text-sm">
                  <span className="inline-flex items-center px-2 py-1 rounded-full bg-blue-100 dark:bg-blue-900 text-blue-800 dark:text-blue-200 text-xs font-medium">
                    {session.request_count} 次
                  </span>
                </td>
                <td className="px-4 py-3 text-sm">
                  <ConfigBadge session={session} />
                </td>
                <td className="px-4 py-3 text-right">
                  <div className="flex items-center justify-end gap-1">
                    {/* 查看日志按钮 */}
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-8"
                      onClick={() => handleViewLogs(session.display_id)}
                      title="查看日志"
                    >
                      <History className="h-3 w-3" />
                    </Button>
                    {/* 备注按钮 */}
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-8"
                      onClick={() => {
                        setSelectedSession(session);
                        setNoteDialogOpen(true);
                      }}
                      title="编辑备注"
                    >
                      <Pencil className="h-3 w-3" />
                    </Button>
                    {/* 配置按钮 */}
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-8"
                      onClick={() => {
                        setSelectedSession(session);
                        setConfigDialogOpen(true);
                      }}
                      title="切换配置"
                    >
                      <Settings className="h-3 w-3" />
                    </Button>
                    {/* 删除按钮 */}
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-8"
                      onClick={() => deleteSession(session.session_id)}
                      title="删除会话"
                    >
                      <Trash2 className="h-3 w-3" />
                    </Button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>

        {/* 配置切换弹窗 */}
        {selectedSession && (
          <SessionConfigDialog
            open={configDialogOpen}
            onOpenChange={setConfigDialogOpen}
            sessionId={selectedSession.session_id}
            currentConfig={selectedSession.config_name}
            currentCustomProfileName={selectedSession.custom_profile_name}
            onConfigUpdated={refresh}
          />
        )}

        {/* 备注编辑弹窗 */}
        {selectedSession && (
          <SessionNoteDialog
            open={noteDialogOpen}
            onOpenChange={setNoteDialogOpen}
            sessionId={selectedSession.session_id}
            currentNote={selectedSession.note}
            onNoteUpdated={refresh}
          />
        )}
      </div>

      {/* 帮助弹窗 */}
      <HelpDialog open={helpDialogOpen} onOpenChange={setHelpDialogOpen} />
    </div>
  );
}
