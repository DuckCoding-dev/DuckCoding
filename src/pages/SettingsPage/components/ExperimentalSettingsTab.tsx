import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import { Sparkles, AlertCircle, Loader2, Power } from 'lucide-react';
import type { TransparentProxyStatus } from '@/lib/tauri-commands';

interface ExperimentalSettingsTabProps {
  transparentProxyEnabled: boolean;
  setTransparentProxyEnabled: (value: boolean) => void;
  transparentProxyPort: number;
  setTransparentProxyPort: (value: number) => void;
  transparentProxyApiKey: string;
  setTransparentProxyApiKey: (value: string) => void;
  transparentProxyAllowPublic: boolean;
  setTransparentProxyAllowPublic: (value: boolean) => void;
  transparentProxyStatus: TransparentProxyStatus | null;
  startingProxy: boolean;
  stoppingProxy: boolean;
  onGenerateProxyKey: () => void;
  onStartProxy: () => void;
  onStopProxy: () => void;
}

export function ExperimentalSettingsTab({
  transparentProxyEnabled,
  setTransparentProxyEnabled,
  transparentProxyPort,
  setTransparentProxyPort,
  transparentProxyApiKey,
  setTransparentProxyApiKey,
  transparentProxyAllowPublic,
  setTransparentProxyAllowPublic,
  transparentProxyStatus,
  startingProxy,
  stoppingProxy,
  onGenerateProxyKey,
  onStartProxy,
  onStopProxy,
}: ExperimentalSettingsTabProps) {
  return (
    <div className="space-y-4 rounded-lg border p-6">
      <div className="flex items-center gap-2">
        <Sparkles className="h-5 w-5" />
        <h3 className="text-lg font-semibold">ClaudeCode 透明代理</h3>
      </div>
      <Separator />

      {/* 实验性功能警告 */}
      <div className="rounded-lg bg-amber-50 dark:bg-amber-950/30 border border-amber-200 dark:border-amber-800 p-4">
        <div className="flex items-start gap-2">
          <AlertCircle className="h-4 w-4 text-amber-600 dark:text-amber-400 flex-shrink-0 mt-0.5" />
          <div className="space-y-1">
            <p className="text-sm font-semibold text-amber-800 dark:text-amber-200">实验性功能</p>
            <p className="text-xs text-amber-700 dark:text-amber-300">
              此功能处于实验阶段，可能存在不稳定性。
            </p>
          </div>
        </div>
      </div>

      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <Label>启用透明代理</Label>
            <p className="text-xs text-muted-foreground">
              允许 ClaudeCode 动态切换 API 配置，无需重启终端
            </p>
          </div>
          <input
            type="checkbox"
            checked={transparentProxyEnabled}
            onChange={(e) => setTransparentProxyEnabled(e.target.checked)}
            className="h-4 w-4 rounded border-slate-300"
          />
        </div>

        {transparentProxyEnabled && (
          <>
            <div className="space-y-2">
              <Label htmlFor="transparent-proxy-port">监听端口</Label>
              <Input
                id="transparent-proxy-port"
                type="number"
                value={transparentProxyPort}
                onChange={(e) => setTransparentProxyPort(parseInt(e.target.value) || 8787)}
              />
              <p className="text-xs text-muted-foreground">
                透明代理服务器监听的本地端口，默认 8787
              </p>
            </div>

            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label htmlFor="transparent-proxy-api-key">API Key *</Label>
                <Button type="button" variant="outline" size="sm" onClick={onGenerateProxyKey}>
                  <Sparkles className="h-3 w-3 mr-1" />
                  生成
                </Button>
              </div>
              <Input
                id="transparent-proxy-api-key"
                type="password"
                placeholder="点击「生成」按钮自动生成"
                value={transparentProxyApiKey}
                onChange={(e) => setTransparentProxyApiKey(e.target.value)}
              />
              <p className="text-xs text-muted-foreground">用于验证透明代理请求的密钥</p>
            </div>

            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <Label>允许公网访问</Label>
                <p className="text-xs text-muted-foreground">
                  允许从非本机地址访问透明代理（不推荐）
                </p>
              </div>
              <input
                type="checkbox"
                checked={transparentProxyAllowPublic}
                onChange={(e) => setTransparentProxyAllowPublic(e.target.checked)}
                className="h-4 w-4 rounded border-slate-300"
              />
            </div>

            {/* 透明代理状态 */}
            {transparentProxyStatus && (
              <div className="mt-4 p-4 rounded-lg border bg-slate-50 dark:bg-slate-900">
                <div className="flex items-center justify-between">
                  <div className="space-y-1">
                    <p className="text-sm font-semibold">代理状态</p>
                    <p className="text-xs text-muted-foreground">
                      {transparentProxyStatus.running
                        ? `运行中 (端口 ${transparentProxyStatus.port})`
                        : '未运行'}
                    </p>
                  </div>
                  <div className="flex gap-2">
                    {transparentProxyStatus.running ? (
                      <Button
                        variant="destructive"
                        size="sm"
                        onClick={onStopProxy}
                        disabled={stoppingProxy}
                      >
                        {stoppingProxy ? (
                          <>
                            <Loader2 className="h-3 w-3 mr-1 animate-spin" />
                            停止中...
                          </>
                        ) : (
                          <>
                            <Power className="h-3 w-3 mr-1" />
                            停止
                          </>
                        )}
                      </Button>
                    ) : (
                      <Button
                        variant="default"
                        size="sm"
                        onClick={onStartProxy}
                        disabled={startingProxy}
                      >
                        {startingProxy ? (
                          <>
                            <Loader2 className="h-3 w-3 mr-1 animate-spin" />
                            启动中...
                          </>
                        ) : (
                          <>
                            <Power className="h-3 w-3 mr-1" />
                            启动
                          </>
                        )}
                      </Button>
                    )}
                  </div>
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
