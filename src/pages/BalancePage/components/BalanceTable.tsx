import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { RefreshCw, Pencil, Trash2, CheckCircle, AlertCircle, Loader2 } from 'lucide-react';
import type { BalanceConfig, BalanceRuntimeState } from '../types';
import { formatDistanceToNow } from 'date-fns';
import { zhCN } from 'date-fns/locale';

interface BalanceTableProps {
  configs: BalanceConfig[];
  stateMap: Record<string, BalanceRuntimeState>;
  onRefresh: (id: string) => void;
  onEdit: (config: BalanceConfig) => void;
  onDelete: (id: string) => void;
}

export function BalanceTable({
  configs,
  stateMap,
  onRefresh,
  onEdit,
  onDelete,
}: BalanceTableProps) {
  const formatTime = (timestamp?: number) => {
    if (!timestamp) return '-';
    try {
      return formatDistanceToNow(timestamp, { addSuffix: true, locale: zhCN });
    } catch {
      return '-';
    }
  };

  const formatCurrency = (amount?: number) => {
    if (amount === undefined || amount === null) return '-';
    return `$${amount.toFixed(4)}`;
  };

  const getStatusIcon = (state?: BalanceRuntimeState) => {
    if (!state) return <span className="text-muted-foreground">-</span>;
    if (state.loading) return <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />;
    if (state.error) return (
      <span title={state.error}>
        <AlertCircle className="h-4 w-4 text-destructive" />
      </span>
    );
    if (state.lastResult) return <CheckCircle className="h-4 w-4 text-green-500" />;
    return <span className="text-muted-foreground">-</span>;
  };

  const getUsagePercentage = (data?: any) => {
    if (!data || !data.total) return 0;
    const used = data.used || 0;
    const total = data.total || 1; // avoid division by zero
    const percent = (used / total) * 100;
    return Math.min(Math.max(percent, 0), 100);
  };

  return (
    <div className="rounded-md border">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead className="w-[180px]">配置名称</TableHead>
            <TableHead className="w-[80px]">状态</TableHead>
            <TableHead className="w-[120px]">剩余余额</TableHead>
            <TableHead className="w-[120px]">已用额度</TableHead>
            <TableHead className="w-[120px]">总额度</TableHead>
            <TableHead className="w-[200px]">使用情况</TableHead>
            <TableHead className="w-[150px]">最后更新</TableHead>
            <TableHead className="text-right">操作</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {configs.map((config) => {
            const state = stateMap[config.id];
            const data = state?.lastResult;
            const percentage = getUsagePercentage(data);

            return (
              <TableRow key={config.id}>
                <TableCell className="font-medium">{config.name}</TableCell>
                <TableCell>{getStatusIcon(state)}</TableCell>
                <TableCell className="font-mono text-green-600 font-medium">
                  {formatCurrency(data?.remaining)}
                </TableCell>
                <TableCell className="font-mono text-muted-foreground">
                  {formatCurrency(data?.used)}
                </TableCell>
                <TableCell className="font-mono text-muted-foreground">
                  {formatCurrency(data?.total)}
                </TableCell>
                <TableCell>
                  {data?.total ? (
                    <div className="flex items-center gap-2">
                      <Progress value={percentage} className="h-2 w-24" />
                      <span className="text-xs text-muted-foreground">{percentage.toFixed(1)}%</span>
                    </div>
                  ) : (
                    <span className="text-xs text-muted-foreground">-</span>
                  )}
                </TableCell>
                <TableCell className="text-xs text-muted-foreground">
                  {formatTime(state?.lastFetchedAt)}
                </TableCell>
                <TableCell className="text-right">
                  <div className="flex justify-end gap-2">
                    <Button
                      size="icon"
                      variant="ghost"
                      className="h-8 w-8"
                      onClick={() => onRefresh(config.id)}
                      disabled={state?.loading}
                      title="刷新"
                    >
                      <RefreshCw className={`h-4 w-4 ${state?.loading ? 'animate-spin' : ''}`} />
                    </Button>
                    <Button
                      size="icon"
                      variant="ghost"
                      className="h-8 w-8"
                      onClick={() => onEdit(config)}
                      title="编辑"
                    >
                      <Pencil className="h-4 w-4" />
                    </Button>
                    <Button
                      size="icon"
                      variant="ghost"
                      className="h-8 w-8 text-destructive hover:text-destructive"
                      onClick={() => onDelete(config.id)}
                      title="删除"
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    </div>
  );
}