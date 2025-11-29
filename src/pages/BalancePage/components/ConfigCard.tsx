import { useMemo } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { cn } from '@/lib/utils';
import { AlertCircle, Clock3, Edit3, Loader2, RefreshCw, Trash2, Wallet } from 'lucide-react';
import { BalanceConfig, BalanceRuntimeState } from '../types';

interface ConfigCardProps {
  config: BalanceConfig;
  state: BalanceRuntimeState;
  onRefresh: (id: string) => void;
  onEdit: (config: BalanceConfig) => void;
  onDelete: (id: string) => void;
}

export function ConfigCard({ config, state, onRefresh, onEdit, onDelete }: ConfigCardProps) {
  const progress = useMemo(() => {
    const total = state.lastResult?.total ?? 0;
    const used = state.lastResult?.used ?? 0;
    if (!total || total <= 0) return 0;
    return Math.min(100, Math.max(0, (used / total) * 100));
  }, [state.lastResult]);

  const unit = state.lastResult?.unit ?? 'USD';
  const remaining = state.lastResult?.remaining;
  const total = state.lastResult?.total;
  const used = state.lastResult?.used;
  const planName = state.lastResult?.planName;

  return (
    <Card className="shadow-sm border">
      <CardHeader className="flex flex-row items-start justify-between space-y-0">
        <div className="space-y-1">
          <CardTitle className="text-base">{config.name}</CardTitle>
          {planName && (
            <Badge variant="secondary" className="text-xs">
              {planName}
            </Badge>
          )}
        </div>
        <div className="flex gap-2">
          <Button size="icon" variant="ghost" onClick={() => onEdit(config)} aria-label="编辑">
            <Edit3 className="h-4 w-4" />
          </Button>
          <Button size="icon" variant="ghost" onClick={() => onDelete(config.id)} aria-label="删除">
            <Trash2 className="h-4 w-4 text-destructive" />
          </Button>
        </div>
      </CardHeader>

      <CardContent className="space-y-4">
        <div className="grid grid-cols-3 gap-4 text-sm">
          <Metric label="总额度" value={formatNumber(total, unit)} />
          <Metric label="已用额度" value={formatNumber(used, unit)} />
          <Metric label="剩余额度" value={formatNumber(remaining, unit)} />
        </div>

        <div className="space-y-2">
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>使用比例</span>
            <span>{progress.toFixed(1)}%</span>
          </div>
          <Progress value={progress} />
          {state.lastResult?.expiresAt && (
            <div className="text-xs text-muted-foreground">
              到期时间：{state.lastResult.expiresAt}
            </div>
          )}
        </div>

        <div className="flex items-center justify-between text-xs text-muted-foreground">
          <div className="flex items-center gap-2">
            <Clock3 className="h-4 w-4" />
            {state.lastFetchedAt ? new Date(state.lastFetchedAt).toLocaleString() : '尚未查询'}
          </div>
          <div className="flex items-center gap-2">
            {state.loading ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Wallet className="h-4 w-4" />
            )}
            {config.intervalSec && config.intervalSec > 0
              ? `自动：每 ${config.intervalSec}s`
              : '手动刷新'}
          </div>
        </div>

        {state.error && (
          <div className="flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-xs text-destructive">
            <AlertCircle className="h-4 w-4 mt-0.5" />
            <span>{state.error}</span>
          </div>
        )}

        <div className="flex gap-2">
          <Button
            size="sm"
            variant="outline"
            className={cn('flex-1', state.loading && 'pointer-events-none')}
            onClick={() => onRefresh(config.id)}
          >
            {state.loading ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                查询中...
              </>
            ) : (
              <>
                <RefreshCw className="mr-2 h-4 w-4" />
                刷新
              </>
            )}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="space-y-1">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="text-sm font-semibold">{value}</div>
    </div>
  );
}

function formatNumber(value?: number | null, currency?: string) {
  if (value == null) return '--';
  const formatted = value.toLocaleString(undefined, { maximumFractionDigits: 2 });
  return currency ? `${formatted} ${currency}` : formatted;
}
