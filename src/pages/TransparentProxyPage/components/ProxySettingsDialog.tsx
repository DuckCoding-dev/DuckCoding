// 代理设置弹窗组件
// 用于配置透明代理的端口、密钥等参数

import { useEffect, useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Sparkles, Copy, Check, Info, Loader2, User } from 'lucide-react';
import { useToast } from '@/hooks/use-toast';
import type { ToolProxyConfig, AmpUserInfo } from '@/lib/tauri-commands';
import { validateAndSaveAmpToken } from '@/lib/tauri-commands';
import type { ToolId } from '../types/proxy-history';

// 工具默认端口映射
function getDefaultPort(toolId: ToolId): number {
  switch (toolId) {
    case 'claude-code':
      return 8787;
    case 'codex':
      return 8788;
    case 'gemini-cli':
      return 8789;
    case 'amp-code':
      return 8790;
    default:
      return 8787;
  }
}

interface ProxySettingsDialogProps {
  /** 弹窗开关状态 */
  open: boolean;
  /** 开关状态变更回调 */
  onOpenChange: (open: boolean) => void;
  /** 工具 ID */
  toolId: ToolId;
  /** 工具名称 */
  toolName: string;
  /** 当前配置 */
  config: ToolProxyConfig | null;
  /** 代理是否运行中 */
  isRunning: boolean;
  /** 保存配置回调 */
  onSave: (updates: Partial<ToolProxyConfig>) => Promise<void>;
}

/**
 * 代理设置弹窗组件
 *
 * 功能：
 * - 配置代理端口、保护密钥
 * - 启用/禁用代理
 * - 会话级端点配置开关（工具级）
 */
export function ProxySettingsDialog({
  open,
  onOpenChange,
  toolId,
  toolName,
  config,
  isRunning,
  onSave,
}: ProxySettingsDialogProps) {
  const { toast } = useToast();
  const [saving, setSaving] = useState(false);
  const [copied, setCopied] = useState(false);

  // 表单状态
  const [enabled, setEnabled] = useState(config?.enabled ?? false);
  const [port, setPort] = useState(config?.port ?? getDefaultPort(toolId));
  const [localApiKey, setLocalApiKey] = useState(config?.local_api_key ?? '');
  const [allowPublic, setAllowPublic] = useState(config?.allow_public ?? false);
  const [sessionEndpointEnabled, setSessionEndpointEnabled] = useState(
    config?.session_endpoint_config_enabled ?? false,
  );
  const [autoStart, setAutoStart] = useState(config?.auto_start ?? false);

  // AMP Access Token 状态（仅 amp-code）
  const [ampAccessToken, setAmpAccessToken] = useState(config?.real_api_key ?? '');
  const [ampUserInfo, setAmpUserInfo] = useState<AmpUserInfo | null>(null);
  const [validatingToken, setValidatingToken] = useState(false);
  // Tavily API Key 状态（仅 amp-code，用于本地搜索）
  const [tavilyApiKey, setTavilyApiKey] = useState(config?.tavily_api_key ?? '');

  // 打开弹窗时重置表单状态
  useEffect(() => {
    if (open && config) {
      setEnabled(config.enabled);
      setPort(config.port);
      setLocalApiKey(config.local_api_key ?? '');
      setAllowPublic(config.allow_public);
      setSessionEndpointEnabled(config.session_endpoint_config_enabled ?? false);
      setAutoStart(config.auto_start ?? false);
      // AMP Access Token
      setAmpAccessToken(config.real_api_key ?? '');
      setAmpUserInfo(null);
      // Tavily API Key
      setTavilyApiKey(config.tavily_api_key ?? '');

      // 如果有保存的 token，自动获取用户信息
      if (toolId === 'amp-code' && config.real_api_key) {
        import('@/lib/tauri-commands').then(({ getSavedAmpUserInfo }) => {
          getSavedAmpUserInfo()
            .then((info) => {
              if (info) setAmpUserInfo(info);
            })
            .catch(console.error);
        });
      }
    }
  }, [open, config, toolId]);

  // 生成随机 API Key
  const handleGenerateApiKey = () => {
    const charset = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    let result = `dc-${toolId.replace('-', '')}-`;
    for (let i = 0; i < 24; i++) {
      result += charset.charAt(Math.floor(Math.random() * charset.length));
    }
    setLocalApiKey(result);
  };

  // 复制密钥
  const handleCopyApiKey = async () => {
    if (!localApiKey) return;
    try {
      await navigator.clipboard.writeText(localApiKey);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      console.error('Failed to copy:', error);
    }
  };

  // 验证 AMP Access Token
  const handleValidateAmpToken = async () => {
    if (!ampAccessToken.trim()) {
      toast({
        title: '请输入 Access Token',
        variant: 'destructive',
      });
      return;
    }

    setValidatingToken(true);
    try {
      const userInfo = await validateAndSaveAmpToken(ampAccessToken.trim());
      setAmpUserInfo(userInfo);
      toast({
        title: '验证成功',
        description: `已登录为 ${userInfo.username || userInfo.email || userInfo.id}`,
      });
    } catch (error) {
      toast({
        title: '验证失败',
        description: String(error),
        variant: 'destructive',
      });
      setAmpUserInfo(null);
    } finally {
      setValidatingToken(false);
    }
  };

  // 保存配置
  const handleSave = async () => {
    // 验证
    if (enabled && !localApiKey) {
      toast({
        title: '配置不完整',
        description: '请先生成或填写保护密钥',
        variant: 'destructive',
      });
      return;
    }

    if (port < 1024 || port > 65535) {
      toast({
        title: '端口无效',
        description: '端口号需要在 1024-65535 之间',
        variant: 'destructive',
      });
      return;
    }

    setSaving(true);
    try {
      const updates: Partial<ToolProxyConfig> = {
        enabled,
        port,
        local_api_key: localApiKey || null,
        allow_public: allowPublic,
        session_endpoint_config_enabled: sessionEndpointEnabled,
        auto_start: autoStart,
      };
      // AMP Access Token 需要一起保存（空值也需要保存以清除配置）
      if (toolId === 'amp-code') {
        updates.real_api_key = ampAccessToken || null;
        updates.real_base_url = ampAccessToken ? 'https://ampcode.com' : null;
        updates.tavily_api_key = tavilyApiKey || null;
      }
      await onSave(updates);
      // 触发配置更新事件
      window.dispatchEvent(new Event('proxy-config-updated'));
      toast({
        title: '配置已保存',
        description: '代理设置已更新',
      });
      onOpenChange(false);
    } catch (error) {
      toast({
        title: '保存失败',
        description: String(error),
        variant: 'destructive',
      });
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[480px]">
        <DialogHeader>
          <DialogTitle>{toolName} 代理设置</DialogTitle>
          <DialogDescription>配置透明代理的端口、密钥等参数</DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* 运行时禁用提示 */}
          {isRunning && (
            <Alert>
              <Info className="h-4 w-4" />
              <AlertDescription>代理运行时无法修改配置，请先停止代理后再进行修改</AlertDescription>
            </Alert>
          )}

          {/* 启用代理 */}
          <div className="flex items-center justify-between">
            <div className="space-y-0.5">
              <Label>启用代理</Label>
              <p className="text-xs text-muted-foreground">开启后可使用透明代理功能</p>
            </div>
            <Switch checked={enabled} onCheckedChange={setEnabled} disabled={isRunning} />
          </div>

          {enabled && (
            <>
              {/* 监听端口 */}
              <div className="space-y-2">
                <Label htmlFor="port">监听端口</Label>
                <Input
                  id="port"
                  type="number"
                  min={1024}
                  max={65535}
                  value={port}
                  onChange={(e) => setPort(parseInt(e.target.value) || getDefaultPort(toolId))}
                  disabled={isRunning}
                  className="w-full"
                />
                <p className="text-xs text-muted-foreground">代理服务监听的本地端口号</p>
              </div>

              {/* 保护密钥 */}
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <Label htmlFor="api-key">保护密钥</Label>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={handleGenerateApiKey}
                    disabled={isRunning}
                    className="h-6 text-xs px-2"
                  >
                    <Sparkles className="h-3 w-3 mr-1" />
                    生成
                  </Button>
                </div>
                <div className="flex gap-2">
                  <Input
                    id="api-key"
                    type="password"
                    placeholder="点击「生成」按钮自动生成"
                    value={localApiKey}
                    onChange={(e) => setLocalApiKey(e.target.value)}
                    disabled={isRunning}
                    className="flex-1 font-mono"
                  />
                  <Button
                    type="button"
                    variant="outline"
                    size="icon"
                    onClick={handleCopyApiKey}
                    disabled={!localApiKey}
                    title="复制密钥"
                  >
                    {copied ? (
                      <Check className="h-4 w-4 text-green-500" />
                    ) : (
                      <Copy className="h-4 w-4" />
                    )}
                  </Button>
                </div>
                <p className="text-xs text-muted-foreground">用于验证请求的本地 API 密钥</p>
              </div>

              {/* AMP Access Token（仅 amp-code） */}
              {toolId === 'amp-code' && (
                <div className="space-y-2 pt-2 border-t">
                  <Label htmlFor="amp-token">AMP Access Token</Label>
                  <div className="flex gap-2">
                    <Input
                      id="amp-token"
                      type="password"
                      placeholder="输入你的 AMP Access Token"
                      value={ampAccessToken}
                      onChange={(e) => setAmpAccessToken(e.target.value)}
                      disabled={isRunning || validatingToken}
                      className="flex-1 font-mono"
                    />
                    <Button
                      type="button"
                      variant="outline"
                      onClick={handleValidateAmpToken}
                      disabled={isRunning || validatingToken || !ampAccessToken.trim()}
                    >
                      {validatingToken ? <Loader2 className="h-4 w-4 animate-spin" /> : '验证'}
                    </Button>
                  </div>
                  <p className="text-xs text-muted-foreground">
                    用于登录 AMP 并获取用户信息，可在{' '}
                    <a
                      href="https://ampcode.com/settings"
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-primary underline"
                    >
                      ampcode.com/settings
                    </a>{' '}
                    获取
                  </p>
                  {ampUserInfo && (
                    <div className="flex items-center gap-2 p-2 bg-muted rounded-md">
                      <User className="h-4 w-4 text-green-500" />
                      <span className="text-sm">
                        已登录: {ampUserInfo.username || ampUserInfo.email || ampUserInfo.id}
                      </span>
                    </div>
                  )}

                  {/* Tavily API Key（用于本地搜索） */}
                  <div className="space-y-2 pt-2 border-t">
                    <Label htmlFor="tavily-key">Tavily API Key（可选）</Label>
                    <Input
                      id="tavily-key"
                      type="password"
                      placeholder="输入 Tavily API Key（用于本地搜索）"
                      value={tavilyApiKey}
                      onChange={(e) => setTavilyApiKey(e.target.value)}
                      disabled={isRunning}
                      className="font-mono"
                    />
                    <p className="text-xs text-muted-foreground">
                      用于本地处理 webSearch2 请求，不配置则使用 DuckDuckGo 搜索。可在{' '}
                      <a
                        href="https://tavily.com"
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-primary underline"
                      >
                        tavily.com
                      </a>{' '}
                      免费获取（每月 1000 次）
                    </p>
                  </div>
                </div>
              )}

              {/* 允许公网访问 */}
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label>允许公网访问</Label>
                  <p className="text-xs text-muted-foreground">不推荐，可能存在安全风险</p>
                </div>
                <Switch
                  checked={allowPublic}
                  onCheckedChange={setAllowPublic}
                  disabled={isRunning}
                />
              </div>

              {/* 会话级端点配置（仅非 AMP） */}
              {toolId !== 'amp-code' && (
                <div className="flex items-center justify-between pt-2 border-t">
                  <div className="space-y-0.5">
                    <Label>会话级端点配置</Label>
                    <p className="text-xs text-muted-foreground">
                      允许为每个代理会话单独配置 API 端点
                    </p>
                  </div>
                  <Switch
                    checked={sessionEndpointEnabled}
                    onCheckedChange={setSessionEndpointEnabled}
                    disabled={isRunning}
                  />
                </div>
              )}

              {/* 应用启动时自动运行 */}
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label>应用启动时自动运行</Label>
                  <p className="text-xs text-muted-foreground">启动 DuckCoding 时自动启动此代理</p>
                </div>
                <Switch checked={autoStart} onCheckedChange={setAutoStart} disabled={isRunning} />
              </div>
            </>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            取消
          </Button>
          <Button onClick={handleSave} disabled={saving || isRunning}>
            {saving ? '保存中...' : '保存配置'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
