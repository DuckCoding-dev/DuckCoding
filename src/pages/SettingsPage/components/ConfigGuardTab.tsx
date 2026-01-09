/**
 * 配置守护设置标签页
 */
import { useState, useEffect, useCallback } from 'react';
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Switch } from '@/components/ui/switch';
import { Loader2, Save, AlertCircle, Info, Settings, History } from 'lucide-react';
import { useToast } from '@/hooks/use-toast';
import { getWatchConfig, updateWatchConfig } from '@/lib/tauri-commands';
import type { ConfigWatchConfig, WatchMode } from '@/types/config-watch';
import { WATCH_MODE_LABELS, WATCH_MODE_DESCRIPTIONS } from '@/types/config-watch';
import { FieldManagementDialog } from '@/components/dialogs/FieldManagementDialog';
import { ChangeLogDialog } from '@/components/dialogs/ChangeLogDialog';

export function ConfigGuardTab() {
  const { toast } = useToast();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [config, setConfig] = useState<ConfigWatchConfig | null>(null);
  const [enabled, setEnabled] = useState(false);
  const [mode, setMode] = useState<WatchMode>('default');
  const [scanInterval, setScanInterval] = useState(5);
  const [fieldsDialogOpen, setFieldsDialogOpen] = useState(false);
  const [historyDialogOpen, setHistoryDialogOpen] = useState(false);

  // 加载配置
  const loadConfig = useCallback(async () => {
    try {
      setLoading(true);
      const watchConfig = await getWatchConfig();
      setConfig(watchConfig);
      setEnabled(watchConfig.enabled);
      setMode(watchConfig.mode);
      setScanInterval(watchConfig.scan_interval);
    } catch (error) {
      toast({
        title: '加载失败',
        description: String(error),
        variant: 'destructive',
      });
    } finally {
      setLoading(false);
    }
  }, [toast]);

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  const handleSave = async () => {
    if (!config) return;

    try {
      setSaving(true);
      const updatedConfig: ConfigWatchConfig = {
        ...config,
        enabled,
        mode,
        scan_interval: scanInterval,
      };
      await updateWatchConfig(updatedConfig);
      setConfig(updatedConfig);
      toast({
        title: '保存成功',
        description: '配置守护设置已更新',
      });
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

  const hasChanges = () => {
    if (!config) return false;
    return (
      enabled !== config.enabled || mode !== config.mode || scanInterval !== config.scan_interval
    );
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (!config) {
    return (
      <div className="flex flex-col items-center justify-center space-y-4 py-12">
        <AlertCircle className="h-12 w-12 text-muted-foreground" />
        <p className="text-sm text-muted-foreground">无法加载配置守护设置</p>
        <Button onClick={loadConfig} variant="outline">
          重试
        </Button>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>配置守护设置</CardTitle>
          <CardDescription>自动监控工具的原生配置文件，防止外部修改导致配置不一致</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* 守护开关 */}
          <div className="flex items-center justify-between space-x-2 rounded-lg border p-4">
            <div className="flex-1 space-y-0.5">
              <Label htmlFor="guard-enabled" className="text-base font-medium">
                启用配置守护
              </Label>
              <p className="text-sm text-muted-foreground">自动检测并通知配置文件的外部变更</p>
            </div>
            <Switch
              id="guard-enabled"
              checked={enabled}
              onCheckedChange={setEnabled}
              aria-label="启用配置守护"
            />
          </div>

          {/* 守护模式 */}
          <div className="space-y-3">
            <Label className="text-base font-medium">守护模式</Label>
            <RadioGroup
              value={mode}
              onValueChange={(v) => setMode(v as WatchMode)}
              disabled={!enabled}
              className="space-y-3"
            >
              <div
                className={`flex items-start space-x-3 rounded-lg border p-4 transition-colors ${
                  enabled ? 'hover:bg-accent/50' : 'opacity-50'
                }`}
              >
                <RadioGroupItem value="default" id="mode-default" className="mt-1" />
                <div className="flex-1 space-y-1">
                  <Label htmlFor="mode-default" className="text-base font-medium cursor-pointer">
                    {WATCH_MODE_LABELS.default}
                    <Badge variant="secondary" className="ml-2">
                      推荐
                    </Badge>
                  </Label>
                  <p className="text-sm text-muted-foreground">{WATCH_MODE_DESCRIPTIONS.default}</p>
                </div>
              </div>

              <div
                className={`flex items-start space-x-3 rounded-lg border p-4 transition-colors ${
                  enabled ? 'hover:bg-accent/50' : 'opacity-50'
                }`}
              >
                <RadioGroupItem value="full" id="mode-full" className="mt-1" />
                <div className="flex-1 space-y-1">
                  <Label htmlFor="mode-full" className="text-base font-medium cursor-pointer">
                    {WATCH_MODE_LABELS.full}
                  </Label>
                  <p className="text-sm text-muted-foreground">{WATCH_MODE_DESCRIPTIONS.full}</p>
                </div>
              </div>
            </RadioGroup>
          </div>

          {/* 扫描间隔 */}
          <div className="space-y-2">
            <Label htmlFor="scan-interval" className="text-base font-medium">
              扫描间隔
            </Label>
            <div className="flex items-center gap-2">
              <Input
                id="scan-interval"
                type="number"
                min={1}
                max={60}
                value={scanInterval}
                onChange={(e) => setScanInterval(Number(e.target.value))}
                disabled={!enabled}
                className="w-24"
              />
              <span className="text-sm text-muted-foreground">秒</span>
            </div>
            <p className="text-xs text-muted-foreground">建议设置为 3-10 秒之间，过短会影响性能</p>
          </div>

          {/* 提示信息 */}
          <div className="flex items-start gap-2 rounded-lg bg-blue-500/10 p-3 text-sm">
            <Info className="mt-0.5 h-4 w-4 flex-shrink-0 text-blue-500" />
            <p className="text-muted-foreground">
              当检测到配置文件被外部修改时，系统会弹出对话框让您选择：
              <span className="font-medium">阻止变更</span>（恢复到上次保存的状态）或
              <span className="font-medium">允许变更</span>（更新配置快照）。
            </p>
          </div>
        </CardContent>

        <CardFooter className="flex gap-2">
          <Button onClick={handleSave} disabled={saving || !hasChanges()} className="gap-2">
            {saving ? (
              <>
                <Loader2 className="h-4 w-4 animate-spin" />
                保存中...
              </>
            ) : (
              <>
                <Save className="h-4 w-4" />
                保存设置
              </>
            )}
          </Button>
          <Button variant="outline" onClick={() => setFieldsDialogOpen(true)} className="gap-2">
            <Settings className="h-4 w-4" />
            管理字段
          </Button>
          <Button variant="outline" onClick={() => setHistoryDialogOpen(true)} className="gap-2">
            <History className="h-4 w-4" />
            扫描历史
          </Button>
        </CardFooter>
      </Card>

      {/* 字段管理对话框 */}
      <FieldManagementDialog
        open={fieldsDialogOpen}
        onOpenChange={setFieldsDialogOpen}
        config={config}
        onConfigUpdate={(updatedConfig) => {
          setConfig(updatedConfig);
          loadConfig(); // 重新加载以确保同步
        }}
      />

      {/* 变更历史对话框 */}
      <ChangeLogDialog open={historyDialogOpen} onOpenChange={setHistoryDialogOpen} />
    </div>
  );
}
