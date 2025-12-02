import { useCallback, useEffect, useMemo, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { Switch } from '@/components/ui/switch';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Loader2, RefreshCw, Wand2, Trash2, Radio } from 'lucide-react';
import { listen } from '@tauri-apps/api/event';
import {
  ackExternalChange,
  cleanLegacyBackups,
  getActiveConfig,
  getExternalChanges,
  getGlobalConfig,
  getMigrationReport,
  importNativeChange,
  getWatcherStatus,
  saveWatcherSettings,
  type ExternalConfigChange,
  type LegacyCleanupResult,
  type MigrationRecord,
} from '@/lib/tauri-commands';
import { useToast } from '@/hooks/use-toast';

export function ConfigManagementTab() {
  const { toast } = useToast();
  const [loading, setLoading] = useState(false);
  const [savingWatch, setSavingWatch] = useState(false);
  const [cleaning, setCleaning] = useState(false);
  const [externalChanges, setExternalChanges] = useState<ExternalConfigChange[]>([]);
  const [migrations, setMigrations] = useState<MigrationRecord[]>([]);
  const [cleanupResults, setCleanupResults] = useState<LegacyCleanupResult[]>([]);
  const [notifyEnabled, setNotifyEnabled] = useState(true);
  const [pollIntervalMs, setPollIntervalMs] = useState(500);
  const [nameDialog, setNameDialog] = useState<{
    open: boolean;
    toolId: string;
    defaultName: string;
  }>({ open: false, toolId: '', defaultName: '' });
  const [inputName, setInputName] = useState('');

  const loadAll = useCallback(async () => {
    setLoading(true);
    try {
      const [changes, mig, watcherOn, cfg] = await Promise.all([
        getExternalChanges().catch(() => []),
        getMigrationReport().catch(() => []),
        getWatcherStatus().catch(() => false),
        getGlobalConfig().catch(() => null),
      ]);
      setExternalChanges(changes);
      setMigrations(mig);
      setNotifyEnabled(watcherOn);
      if (cfg?.external_poll_interval_ms !== undefined) {
        setPollIntervalMs(cfg.external_poll_interval_ms);
      }
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadAll();
  }, [loadAll]);

  const saveWatchSettings = useCallback(async () => {
    setSavingWatch(true);
    try {
      await saveWatcherSettings(notifyEnabled, pollIntervalMs);
      toast({
        title: '监听设置已保存',
        description: notifyEnabled
          ? `已开启监听，间隔 ${pollIntervalMs}ms`
          : '监听已关闭，仅手动刷新生效',
      });
    } catch (error) {
      toast({
        title: '保存失败',
        description: String(error),
        variant: 'destructive',
      });
    } finally {
      setSavingWatch(false);
    }
  }, [notifyEnabled, pollIntervalMs, toast]);

  const handleAck = useCallback(
    async (toolId: string) => {
      try {
        await ackExternalChange(toolId);
        toast({ title: '已标记为已处理', description: toolId });
        void loadAll();
      } catch (error) {
        toast({ title: '操作失败', description: String(error), variant: 'destructive' });
      }
    },
    [loadAll, toast],
  );

  const handleImport = useCallback(
    async (toolId: string, asNew: boolean) => {
      if (asNew) {
        const defaultName = `imported-${toolId}`;
        setInputName(defaultName);
        setNameDialog({ open: true, toolId, defaultName });
        return;
      }

      // 覆盖当前：直接用当前激活 profile
      const profileName = (await getActiveConfig(toolId)).profile_name || 'default';
      try {
        await importNativeChange(toolId, profileName, false);
        toast({
          title: '导入完成',
          description: `已覆盖 ${profileName}`,
        });
        void loadAll();
      } catch (error) {
        console.error('[ConfigManagement] import failed', error);
        toast({ title: '导入失败', description: String(error), variant: 'destructive' });
      }
    },
    [loadAll, toast],
  );

  const handleConfirmImportNew = useCallback(async () => {
    const targetName = inputName.trim();
    if (!targetName) {
      toast({
        title: '导入失败',
        description: '请输入非空的 Profile 名称',
        variant: 'destructive',
      });
      return;
    }
    const toolId = nameDialog.toolId;
    try {
      await importNativeChange(toolId, targetName, true);
      toast({
        title: '导入完成',
        description: `已导入为 ${targetName}`,
      });
      setNameDialog({ open: false, toolId: '', defaultName: '' });
      void loadAll();
    } catch (error) {
      console.error('[ConfigManagement] import new failed', error);
      toast({ title: '导入失败', description: String(error), variant: 'destructive' });
    }
  }, [inputName, loadAll, nameDialog.toolId, toast]);

  const handleCleanup = useCallback(async () => {
    setCleaning(true);
    try {
      const results = await cleanLegacyBackups();
      setCleanupResults(results);
      toast({
        title: '清理完成',
        description: `成功 ${results.reduce((s, r) => s + r.removed.length, 0)} 项`,
      });
    } catch (error) {
      toast({ title: '清理失败', description: String(error), variant: 'destructive' });
    } finally {
      setCleaning(false);
    }
  }, [toast]);

  const latestMigrations = useMemo(() => migrations.slice(-10).reverse(), [migrations]);

  // 监听实时外部改动事件，前端直接追加
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    const setup = async () => {
      try {
        unlisten = await listen<ExternalConfigChange>('external-config-changed', (event) => {
          setExternalChanges((prev) => {
            const payload = event.payload;
            const filtered = prev.filter(
              (c) => !(c.tool_id === payload.tool_id && c.path === payload.path),
            );
            return [...filtered, payload];
          });
        });
      } catch (error) {
        toast({
          title: '监听事件失败',
          description: String(error),
          variant: 'destructive',
        });
      }
    };
    void setup();
    return () => {
      if (unlisten) unlisten();
    };
  }, [toast]);

  // 切换开关即刻应用
  const handleToggleWatch = useCallback(
    async (enabled: boolean) => {
      const previous = notifyEnabled;
      setNotifyEnabled(enabled);
      setSavingWatch(true);
      try {
        await saveWatcherSettings(enabled, pollIntervalMs);
        const latest = await getWatcherStatus().catch(() => enabled);
        setNotifyEnabled(latest);
        toast({
          title: '监听设置已更新',
          description: latest
            ? `已开启监听，间隔 ${pollIntervalMs}ms`
            : '监听已关闭，仅手动刷新生效',
        });
      } catch (error) {
        setNotifyEnabled(previous);
        toast({
          title: '保存失败',
          description: String(error),
          variant: 'destructive',
        });
      } finally {
        setSavingWatch(false);
      }
    },
    [notifyEnabled, pollIntervalMs, toast],
  );

  return (
    <div className="space-y-4 rounded-lg border p-6">
      <div className="flex items-center gap-2">
        <Radio className="h-5 w-5" />
        <div>
          <h3 className="text-lg font-semibold">配置文件监控</h3>
        </div>
      </div>
      <Separator />

      <Dialog
        open={nameDialog.open}
        onOpenChange={(open) =>
          setNameDialog((prev) => (open ? prev : { open: false, toolId: '', defaultName: '' }))
        }
      >
        <DialogContent className="sm:max-w-[420px]" onPointerDown={(e) => e.stopPropagation()}>
          <DialogHeader>
            <DialogTitle>导入为新配置</DialogTitle>
          </DialogHeader>
          <div className="space-y-2 pt-2">
            <div className="text-sm text-muted-foreground">
              请输入新配置名称（工具：{nameDialog.toolId || '-'}）
            </div>
            <Input
              value={inputName}
              onChange={(e) => setInputName(e.target.value)}
              placeholder={nameDialog.defaultName || 'new-profile'}
            />
          </div>
          <DialogFooter className="gap-2">
            <Button
              variant="outline"
              onClick={() => setNameDialog({ open: false, toolId: '', defaultName: '' })}
            >
              取消
            </Button>
            <Button onClick={handleConfirmImportNew}>确认导入</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <div className="space-y-4">
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div>
              <div className="text-sm font-medium">实时监听</div>
              <p className="text-xs text-muted-foreground">
                打开后自动监听文件改动；关闭时仅在手动刷新时检查。
              </p>
            </div>
            <Switch
              checked={notifyEnabled}
              onCheckedChange={handleToggleWatch}
              disabled={savingWatch}
            />
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <div className="flex items-center gap-2">
              <span className="text-xs text-muted-foreground">轮询间隔 (ms)</span>
              <Input
                className="h-8 w-24"
                type="number"
                min={0}
                value={pollIntervalMs}
                onChange={(e) => setPollIntervalMs(Number(e.target.value) || 0)}
              />
            </div>
            <Button size="sm" variant="outline" onClick={saveWatchSettings} disabled={savingWatch}>
              {savingWatch && <Loader2 className="h-4 w-4 mr-2 animate-spin" aria-hidden />}
              保存
            </Button>
            <Button size="sm" variant="outline" onClick={loadAll} disabled={loading}>
              {loading && <Loader2 className="h-4 w-4 mr-2 animate-spin" aria-hidden />}
              <RefreshCw className="h-4 w-4 mr-2" />
              刷新
            </Button>
          </div>
        </div>

        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div>
              <div className="text-sm font-medium">外部改动</div>
              <p className="text-xs text-muted-foreground">
                捕获配置文件的外部修改，可导入为新 Profile、覆盖当前或直接标记已处理。
              </p>
            </div>
            <Badge variant={externalChanges.length > 0 ? 'destructive' : 'outline'}>
              {externalChanges.length} 项
            </Badge>
          </div>
          <div className="space-y-2">
            {externalChanges.length === 0 ? (
              <div className="text-sm text-muted-foreground">暂无外部改动</div>
            ) : (
              externalChanges.map((change) => (
                <div
                  key={`${change.tool_id}-${change.path}`}
                  className="flex flex-wrap items-center justify-between gap-2 rounded border bg-background px-3 py-2"
                >
                  <div className="flex flex-col text-sm">
                    <span className="font-medium">{change.tool_id}</span>
                    <span className="text-xs text-muted-foreground break-all">{change.path}</span>
                    <span className="text-xs text-muted-foreground">
                      检测时间：{new Date(change.detected_at).toLocaleString()}
                    </span>
                  </div>
                  <div className="flex flex-wrap items-center gap-2">
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={() => handleImport(change.tool_id, true)}
                    >
                      <Wand2 className="h-4 w-4 mr-1" />
                      导入为新
                    </Button>
                    <Button size="sm" onClick={() => handleImport(change.tool_id, false)}>
                      覆盖当前
                    </Button>
                    <Button size="sm" variant="outline" onClick={() => handleAck(change.tool_id)}>
                      已处理
                    </Button>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div>
              <div className="text-sm font-medium">迁移记录</div>
              <p className="text-xs text-muted-foreground">回顾最近的配置迁移动作与结果。</p>
            </div>
            <Badge variant="outline">{latestMigrations.length}</Badge>
          </div>
          <div className="space-y-2">
            {latestMigrations.length === 0 ? (
              <div className="text-sm text-muted-foreground">暂无迁移记录</div>
            ) : (
              latestMigrations.map((record) => (
                <div
                  key={`${record.tool_id}-${record.profile_name}-${record.timestamp}`}
                  className="flex flex-wrap items-center justify-between rounded border bg-background px-3 py-2 text-sm"
                >
                  <div className="flex flex-col">
                    <span className="font-medium">
                      {record.tool_id} / {record.profile_name}
                    </span>
                    <span className="text-xs text-muted-foreground break-all">
                      {record.from_path}
                    </span>
                    <span className="text-xs text-muted-foreground">
                      {new Date(record.timestamp).toLocaleString()}
                    </span>
                  </div>
                  <Badge variant={record.succeeded ? 'outline' : 'destructive'}>
                    {record.succeeded ? '成功' : '失败'}
                  </Badge>
                </div>
              ))
            )}
          </div>
        </div>

        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div>
              <div className="text-sm font-medium">清理旧备份</div>
              <p className="text-xs text-muted-foreground">
                一键删除历史备份，释放空间；失败项会单独列出。
              </p>
            </div>
            <Button size="sm" variant="outline" onClick={handleCleanup} disabled={cleaning}>
              {cleaning && <Loader2 className="h-4 w-4 mr-2 animate-spin" aria-hidden />}
              <Trash2 className="h-4 w-4 mr-1" />
              开始清理
            </Button>
          </div>
          <div className="space-y-2">
            {cleanupResults.length === 0 ? (
              <div className="text-sm text-muted-foreground">尚未执行清理</div>
            ) : (
              cleanupResults.map((r) => (
                <div
                  key={r.tool_id}
                  className="flex flex-wrap items-center justify-between rounded border bg-background px-3 py-2 text-sm"
                >
                  <div className="flex flex-col">
                    <span className="font-medium">{r.tool_id}</span>
                    <span className="text-xs text-muted-foreground">
                      清理 {r.removed.length}，失败 {r.failed.length}
                    </span>
                  </div>
                  {r.failed.length > 0 && <Badge variant="destructive">部分失败</Badge>}
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
