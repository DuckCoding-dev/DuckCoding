// Token 统计设置 Tab
// 配置自动清理策略和日志保留规则

import { useState, useEffect, useCallback } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { Switch } from '@/components/ui/switch';
import { Database, Save, Loader2, AlertCircle, Trash2 } from 'lucide-react';
import { useToast } from '@/hooks/use-toast';
import { Alert, AlertDescription } from '@/components/ui/alert';
import {
  getTokenStatsConfig,
  updateTokenStatsConfig,
  getTokenStatsSummary,
  cleanupTokenLogs,
} from '@/lib/tauri-commands';
import type { TokenStatsConfig, DatabaseSummary } from '@/types/token-stats';
import { DEFAULT_TOKEN_STATS_CONFIG } from '@/types/token-stats';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from '@/components/ui/alert-dialog';

export function TokenStatsTab() {
  const { toast } = useToast();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [isCleaningUp, setIsCleaningUp] = useState(false);
  const [config, setConfig] = useState<TokenStatsConfig>(DEFAULT_TOKEN_STATS_CONFIG);
  const [summary, setSummary] = useState<DatabaseSummary | null>(null);

  // 加载配置和数据库摘要
  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      const [currentConfig, currentSummary] = await Promise.all([
        getTokenStatsConfig(),
        getTokenStatsSummary(),
      ]);
      setConfig(currentConfig);
      setSummary(currentSummary);
    } catch (error) {
      console.error('Failed to load token stats config:', error);
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
    loadData();
  }, [loadData]);

  // 保存配置
  const handleSave = async () => {
    try {
      setSaving(true);
      await updateTokenStatsConfig(config);
      toast({
        title: '保存成功',
        description: '配置已更新',
      });
    } catch (error) {
      console.error('Failed to save token stats config:', error);
      toast({
        title: '保存失败',
        description: String(error),
        variant: 'destructive',
      });
    } finally {
      setSaving(false);
    }
  };

  // 手动清理日志
  const handleCleanup = async () => {
    setIsCleaningUp(true);
    try {
      const deletedCount = await cleanupTokenLogs(config.retention_days, config.max_log_count);
      toast({
        title: '清理成功',
        description: `已清理 ${deletedCount} 条旧日志`,
      });

      // 重新加载摘要
      const newSummary = await getTokenStatsSummary();
      setSummary(newSummary);
    } catch (error) {
      console.error('Failed to cleanup logs:', error);
      toast({
        title: '清理失败',
        description: String(error),
        variant: 'destructive',
      });
    } finally {
      setIsCleaningUp(false);
    }
  };

  // 格式化日期
  const formatDate = (timestamp?: number) => {
    if (!timestamp) return '无';
    return new Date(timestamp).toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-20">
        <Loader2 className="h-8 w-8 animate-spin text-primary" />
        <span className="ml-3 text-muted-foreground">加载配置中...</span>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* 页头 */}
      <div className="flex items-center gap-2">
        <Database className="h-5 w-5" />
        <div>
          <h3 className="text-lg font-medium">Token 统计配置</h3>
          <p className="text-sm text-muted-foreground">管理透明代理的 Token 日志保留策略</p>
        </div>
      </div>

      <Separator />

      {/* 数据库信息 */}
      {summary && (
        <div className="rounded-lg border border-border/50 bg-muted/30 p-4 space-y-2">
          <h4 className="font-medium text-sm">数据库状态</h4>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
            <div>
              <span className="text-muted-foreground">总记录数:</span>
              <span className="ml-2 font-medium">{summary.total_logs.toLocaleString('zh-CN')}</span>
            </div>
            <div>
              <span className="text-muted-foreground">最早记录:</span>
              <span className="ml-2 font-medium">{formatDate(summary.oldest_timestamp)}</span>
            </div>
            <div>
              <span className="text-muted-foreground">最新记录:</span>
              <span className="ml-2 font-medium">{formatDate(summary.newest_timestamp)}</span>
            </div>
          </div>
        </div>
      )}

      {/* 自动清理开关 */}
      <div className="flex items-center justify-between rounded-lg border border-border/50 p-4">
        <div className="space-y-0.5">
          <Label className="text-base">自动清理旧日志</Label>
          <p className="text-sm text-muted-foreground">
            启用后，系统将根据下方配置自动清理过期日志
          </p>
        </div>
        <Switch
          checked={config.auto_cleanup_enabled}
          onCheckedChange={(checked) => setConfig({ ...config, auto_cleanup_enabled: checked })}
        />
      </div>

      {/* 保留天数配置 */}
      <div className="space-y-2">
        <Label htmlFor="retention-days">
          保留天数
          <span className="ml-1 text-xs text-muted-foreground">（可选）</span>
        </Label>
        <Input
          id="retention-days"
          type="number"
          min="1"
          max="365"
          placeholder="例如: 30（留空表示不限制）"
          value={config.retention_days ?? ''}
          onChange={(e) => {
            const value = e.target.value ? parseInt(e.target.value) : undefined;
            setConfig({ ...config, retention_days: value });
          }}
          disabled={!config.auto_cleanup_enabled}
        />
        <p className="text-xs text-muted-foreground">系统将自动删除超过指定天数的日志记录</p>
      </div>

      {/* 最大日志条数配置 */}
      <div className="space-y-2">
        <Label htmlFor="max-log-count">
          最大日志条数
          <span className="ml-1 text-xs text-muted-foreground">（可选）</span>
        </Label>
        <Input
          id="max-log-count"
          type="number"
          min="100"
          max="1000000"
          placeholder="例如: 10000（留空表示不限制）"
          value={config.max_log_count ?? ''}
          onChange={(e) => {
            const value = e.target.value ? parseInt(e.target.value) : undefined;
            setConfig({ ...config, max_log_count: value });
          }}
          disabled={!config.auto_cleanup_enabled}
        />
        <p className="text-xs text-muted-foreground">当日志条数超过此限制时，自动删除最旧的记录</p>
      </div>

      {/* 警告提示 */}
      {config.auto_cleanup_enabled && !config.retention_days && !config.max_log_count && (
        <Alert variant="default">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>
            未设置保留天数和最大条数，自动清理将不会执行。请至少设置一项。
          </AlertDescription>
        </Alert>
      )}

      {/* 操作按钮 */}
      <div className="flex items-center gap-3 pt-4">
        <Button onClick={handleSave} disabled={saving}>
          {saving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
          {!saving && <Save className="mr-2 h-4 w-4" />}
          保存配置
        </Button>

        {/* 手动清理按钮 */}
        <AlertDialog>
          <AlertDialogTrigger asChild>
            <Button variant="outline" disabled={isCleaningUp}>
              {isCleaningUp && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              {!isCleaningUp && <Trash2 className="mr-2 h-4 w-4" />}
              立即清理
            </Button>
          </AlertDialogTrigger>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle className="flex items-center gap-2">
                <AlertCircle className="h-5 w-5 text-orange-500" />
                确认清理日志
              </AlertDialogTitle>
              <AlertDialogDescription className="space-y-2">
                <p>此操作将根据当前配置清理旧日志：</p>
                <ul className="list-disc list-inside space-y-1 text-sm">
                  {config.retention_days && <li>保留最近 {config.retention_days} 天的日志</li>}
                  {config.max_log_count && (
                    <li>最多保留 {config.max_log_count.toLocaleString('zh-CN')} 条记录</li>
                  )}
                </ul>
                <p className="text-destructive font-medium mt-2">此操作不可撤销！</p>
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>取消</AlertDialogCancel>
              <AlertDialogAction onClick={handleCleanup} disabled={isCleaningUp}>
                {isCleaningUp ? '清理中...' : '确认清理'}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </div>
    </div>
  );
}
