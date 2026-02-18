import { useState, useEffect, useCallback } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { Switch } from '@/components/ui/switch';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  CalendarCheck,
  Loader2,
  CheckCircle2,
  XCircle,
  TrendingUp,
  Calendar,
  Coins,
  Clock,
} from 'lucide-react';
import type { Provider } from '@/lib/tauri-commands';
import { performCheckin, getCheckinStatus } from '@/services/checkin';
import { useToast } from '@/hooks/use-toast';

interface CheckinDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  provider: Provider;
  onUpdate: (provider: Provider) => void;
}

/** 格式化时间范围描述 */
function formatRangeDesc(start: number, end: number): string {
  if (start >= end) return '全天随机时间自动签到';
  return `每天 ${start.toString().padStart(2, '0')}:00 ~ ${end.toString().padStart(2, '0')}:00 随机时间自动签到`;
}

export function CheckinDialog({ open, onOpenChange, provider, onUpdate }: CheckinDialogProps) {
  const { toast } = useToast();
  const [loading, setLoading] = useState(false);
  const [statusLoading, setStatusLoading] = useState(false);
  const [checkinStatus, setCheckinStatus] = useState<any>(null);
  const [autoCheckinEnabled, setAutoCheckinEnabled] = useState(
    provider.checkin_config?.enabled || false,
  );
  const [checkinHourStart, setCheckinHourStart] = useState(
    provider.checkin_config?.checkin_hour_start ?? 0,
  );
  const [checkinHourEnd, setCheckinHourEnd] = useState(
    provider.checkin_config?.checkin_hour_end ?? 0,
  );

  // provider 变化时重置状态
  useEffect(() => {
    setCheckinStatus(null);
    setAutoCheckinEnabled(provider.checkin_config?.enabled || false);
    setCheckinHourStart(provider.checkin_config?.checkin_hour_start ?? 0);
    setCheckinHourEnd(provider.checkin_config?.checkin_hour_end ?? 0);
  }, [provider]);

  const loadCheckinStatus = useCallback(async () => {
    setStatusLoading(true);
    try {
      const result = await getCheckinStatus(provider);
      if (result.success && result.data) {
        setCheckinStatus(result.data);
      } else if (!result.success) {
        if (result.message?.includes('不支持签到') || result.message?.includes('404')) {
          setCheckinStatus({ unsupported: true, message: result.message });
        }
      }
    } catch (error) {
      console.error('Failed to load checkin status:', error);
    } finally {
      setStatusLoading(false);
    }
  }, [provider]);

  // 加载签到状态
  useEffect(() => {
    if (open) {
      loadCheckinStatus();
    }
  }, [open, loadCheckinStatus]);

  const handleCheckin = async () => {
    setLoading(true);
    try {
      const result = await performCheckin(provider);

      if (result.success) {
        toast({
          title: '签到成功',
          description: result.data?.quota_awarded
            ? `获得额度: $${(result.data.quota_awarded / 500000).toFixed(2)}`
            : result.message,
        });

        const updatedProvider = {
          ...provider,
          checkin_config: {
            ...provider.checkin_config,
            enabled: autoCheckinEnabled,
            endpoint: '/api/user/checkin',
            last_checkin_at: Math.floor(Date.now() / 1000),
            last_checkin_status: 'success' as const,
            last_checkin_message: result.message,
            total_checkins: (provider.checkin_config?.total_checkins || 0) + 1,
            total_quota:
              (provider.checkin_config?.total_quota || 0) + (result.data?.quota_awarded || 0),
          },
        };
        onUpdate(updatedProvider);

        await loadCheckinStatus();
      } else {
        toast({
          title: '签到失败',
          description: result.message || '未知错误',
          variant: 'destructive',
        });
      }
    } catch (error) {
      toast({
        title: '签到失败',
        description: error instanceof Error ? error.message : '未知错误',
        variant: 'destructive',
      });
    } finally {
      setLoading(false);
    }
  };

  const handleAutoCheckinToggle = (enabled: boolean) => {
    setAutoCheckinEnabled(enabled);
    const updatedProvider = {
      ...provider,
      checkin_config: {
        ...provider.checkin_config,
        enabled,
        endpoint: '/api/user/checkin',
        checkin_hour_start: checkinHourStart,
        checkin_hour_end: checkinHourEnd,
      },
    };
    onUpdate(updatedProvider);

    toast({
      title: enabled ? '已启用自动签到' : '已关闭自动签到',
      description: enabled ? formatRangeDesc(checkinHourStart, checkinHourEnd) : '不再自动签到',
    });
  };

  const handleCheckinRangeChange = (type: 'start' | 'end', hour: string) => {
    const newHour = parseInt(hour);
    const newStart = type === 'start' ? newHour : checkinHourStart;
    const newEnd = type === 'end' ? newHour : checkinHourEnd;

    if (type === 'start') setCheckinHourStart(newHour);
    else setCheckinHourEnd(newHour);

    if (autoCheckinEnabled) {
      const updatedProvider = {
        ...provider,
        checkin_config: {
          ...provider.checkin_config,
          enabled: autoCheckinEnabled,
          endpoint: '/api/user/checkin',
          checkin_hour_start: newStart,
          checkin_hour_end: newEnd,
          // 时间范围变了，清除已生成的计划让调度器重新生成
          next_checkin_at: undefined,
        },
      };
      onUpdate(updatedProvider);

      toast({
        title: '签到时间已更新',
        description: formatRangeDesc(newStart, newEnd),
      });
    }
  };

  const formatDate = (timestamp?: number) => {
    if (!timestamp) return '-';
    return new Date(timestamp * 1000).toLocaleString('zh-CN');
  };

  const isFullDay = checkinHourStart >= checkinHourEnd;
  const stats = checkinStatus?.stats;
  const checkedInToday = stats?.checked_in_today || false;
  const controlsDisabled = statusLoading || !stats;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <CalendarCheck className="h-5 w-5 text-primary" />
            签到管理 - {provider.name}
          </DialogTitle>
          <DialogDescription>管理供应商的签到设置和查看签到历史</DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* 自动签到开关 */}
          <div className="space-y-3 p-3 bg-muted/50 rounded-lg">
            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <Label htmlFor="auto-checkin" className="text-sm font-medium">
                  自动签到
                </Label>
                <p className="text-xs text-muted-foreground">在时间范围内随机时间自动签到</p>
              </div>
              <Switch
                id="auto-checkin"
                checked={autoCheckinEnabled}
                onCheckedChange={handleAutoCheckinToggle}
                disabled={controlsDisabled}
              />
            </div>

            {/* 签到时间范围选择 */}
            {autoCheckinEnabled && (
              <div className="space-y-2 pt-2 border-t">
                <div className="flex items-center gap-2">
                  <Clock className="h-4 w-4 text-muted-foreground" />
                  <Label className="text-xs text-muted-foreground">签到时间范围:</Label>
                  {isFullDay && (
                    <Badge variant="secondary" className="text-xs">
                      全天随机
                    </Badge>
                  )}
                </div>
                <div className="flex items-center gap-2">
                  <Select
                    value={checkinHourStart.toString()}
                    onValueChange={(v) => handleCheckinRangeChange('start', v)}
                    disabled={controlsDisabled}
                  >
                    <SelectTrigger className="h-8 w-24">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {Array.from({ length: 24 }, (_, i) => (
                        <SelectItem key={i} value={i.toString()}>
                          {i.toString().padStart(2, '0')}:00
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  <span className="text-xs text-muted-foreground">至</span>
                  <Select
                    value={checkinHourEnd.toString()}
                    onValueChange={(v) => handleCheckinRangeChange('end', v)}
                    disabled={controlsDisabled}
                  >
                    <SelectTrigger className="h-8 w-24">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {Array.from({ length: 24 }, (_, i) => (
                        <SelectItem key={i} value={i.toString()}>
                          {i.toString().padStart(2, '0')}:00
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
              </div>
            )}
          </div>

          <Separator />

          {/* 签到状态 */}
          {statusLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : stats ? (
            (stats as any).unsupported ? (
              <div className="text-center py-8 space-y-3">
                <XCircle className="h-12 w-12 mx-auto text-muted-foreground/50" />
                <div className="space-y-1">
                  <p className="text-sm font-medium text-muted-foreground">
                    该供应商不支持签到功能
                  </p>
                  <p className="text-xs text-muted-foreground">
                    {(stats as any).message || '请确认供应商是否已升级到支持签到的版本'}
                  </p>
                </div>
              </div>
            ) : (
              <div className="space-y-3">
                {/* 今日状态 */}
                <div className="flex items-center justify-between p-3 bg-muted/30 rounded-lg">
                  <div className="flex items-center gap-2">
                    {checkedInToday ? (
                      <CheckCircle2 className="h-4 w-4 text-green-500" />
                    ) : (
                      <XCircle className="h-4 w-4 text-muted-foreground" />
                    )}
                    <span className="text-sm font-medium">今日签到</span>
                  </div>
                  <Badge variant={checkedInToday ? 'default' : 'secondary'}>
                    {checkedInToday ? '已签到' : '未签到'}
                  </Badge>
                </div>

                {/* 统计信息 */}
                <div className="grid grid-cols-3 gap-3">
                  <div className="p-3 bg-muted/30 rounded-lg space-y-1">
                    <div className="flex items-center gap-1 text-xs text-muted-foreground">
                      <Calendar className="h-3 w-3" />
                      <span>本月</span>
                    </div>
                    <div className="text-lg font-semibold">{stats.checkin_count || 0}</div>
                  </div>

                  <div className="p-3 bg-muted/30 rounded-lg space-y-1">
                    <div className="flex items-center gap-1 text-xs text-muted-foreground">
                      <TrendingUp className="h-3 w-3" />
                      <span>累计</span>
                    </div>
                    <div className="text-lg font-semibold">{stats.total_checkins || 0}</div>
                  </div>

                  <div className="p-3 bg-muted/30 rounded-lg space-y-1">
                    <div className="flex items-center gap-1 text-xs text-muted-foreground">
                      <Coins className="h-3 w-3" />
                      <span>总额度</span>
                    </div>
                    <div className="text-lg font-semibold">
                      ${((stats.total_quota || 0) / 500000).toFixed(2)}
                    </div>
                  </div>
                </div>

                {/* 最近签到记录 */}
                {stats.records && stats.records.length > 0 && (
                  <div className="space-y-2">
                    <Label className="text-xs text-muted-foreground">最近签到记录</Label>
                    <div className="space-y-1 max-h-32 overflow-y-auto">
                      {stats.records.slice(0, 5).map((record: any, index: number) => (
                        <div
                          key={index}
                          className="flex items-center justify-between p-2 bg-muted/20 rounded text-xs"
                        >
                          <span className="text-muted-foreground">{record.checkin_date}</span>
                          <span className="font-medium">
                            +${(record.quota_awarded / 500000).toFixed(2)}
                          </span>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            )
          ) : (
            <div className="text-center py-8 text-sm text-muted-foreground">暂无签到数据</div>
          )}

          {/* 最后签到信息 */}
          {provider.checkin_config?.last_checkin_at && (
            <div className="text-xs text-muted-foreground space-y-1">
              <div>最后签到: {formatDate(provider.checkin_config.last_checkin_at)}</div>
              {provider.checkin_config.last_checkin_message && (
                <div>消息: {provider.checkin_config.last_checkin_message}</div>
              )}
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            关闭
          </Button>
          <Button
            onClick={handleCheckin}
            disabled={loading || statusLoading || !stats || checkedInToday}
          >
            {loading ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                签到中...
              </>
            ) : statusLoading ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                检查中...
              </>
            ) : checkedInToday ? (
              '今日已签到'
            ) : (
              <>
                <CalendarCheck className="mr-2 h-4 w-4" />
                立即签到
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
