/**
 * 编辑远程令牌对话框
 * 支持修改令牌的所有可配置字段，包括 CIDR 表达式验证的 IP 白名单
 */

import { useState, useEffect } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Checkbox } from '@/components/ui/checkbox';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Loader2, AlertTriangle } from 'lucide-react';
import { useToast } from '@/hooks/use-toast';
import type { Provider } from '@/types/provider';
import type { RemoteToken, RemoteTokenGroup, UpdateRemoteTokenRequest } from '@/types/remote-token';
import { validateIpWhitelist, formatIpWhitelist } from '../utils/ipValidation';

interface EditTokenDialogProps {
  /** 对话框打开状态 */
  open: boolean;
  /** 对话框状态变更回调 */
  onOpenChange: (open: boolean) => void;
  /** 当前供应商 */
  provider: Provider;
  /** 要编辑的令牌 */
  token: RemoteToken | null;
  /** 令牌分组列表 */
  tokenGroups: RemoteTokenGroup[];
  /** 更新成功回调 */
  onSuccess: () => void;
  /** 更新函数 */
  onUpdate: (
    provider: Provider,
    tokenId: number,
    request: UpdateRemoteTokenRequest,
  ) => Promise<void>;
}

/**
 * 编辑令牌对话框
 */
export function EditTokenDialog({
  open,
  onOpenChange,
  provider,
  token,
  tokenGroups,
  onSuccess,
  onUpdate,
}: EditTokenDialogProps) {
  const { toast } = useToast();

  // ==================== 表单状态 ====================
  const [name, setName] = useState('');
  const [groupId, setGroupId] = useState('');
  const [quota, setQuota] = useState(0);
  const [unlimitedQuota, setUnlimitedQuota] = useState(false);
  const [expiredTime, setExpiredTime] = useState(-1);
  const [unlimitedExpire, setUnlimitedExpire] = useState(true);
  const [modelLimitsEnabled, setModelLimitsEnabled] = useState(false);
  const [modelLimits, setModelLimits] = useState('');
  const [allowIps, setAllowIps] = useState('');

  // ==================== 验证状态 ====================
  const [ipValidationResult, setIpValidationResult] = useState<ReturnType<
    typeof validateIpWhitelist
  > | null>(null);

  // ==================== 加载状态 ====================
  const [updating, setUpdating] = useState(false);

  /**
   * Dialog 打开时初始化表单
   */
  useEffect(() => {
    if (open && token) {
      setName(token.name);
      setGroupId(token.group);
      setUnlimitedQuota(token.unlimited_quota);
      setQuota(token.remain_quota / 500000); // token -> USD
      setUnlimitedExpire(token.expired_time === -1);
      setExpiredTime(token.expired_time);
      setModelLimitsEnabled(token.model_limits_enabled);
      setModelLimits(token.model_limits);
      setAllowIps(token.allow_ips);
      setIpValidationResult(null);
    }
  }, [open, token]);

  /**
   * IP 白名单实时验证
   */
  useEffect(() => {
    if (allowIps) {
      const result = validateIpWhitelist(allowIps);
      setIpValidationResult(result);
    } else {
      setIpValidationResult(null);
    }
  }, [allowIps]);

  /**
   * 计算过期时间显示值（天数）
   */
  const getExpireDays = (): number => {
    if (unlimitedExpire || expiredTime === -1) return 0;
    const now = Math.floor(Date.now() / 1000);
    const remainingSeconds = expiredTime - now;
    return Math.max(0, Math.ceil(remainingSeconds / (24 * 60 * 60)));
  };

  /**
   * 设置过期天数
   */
  const setExpireDays = (days: number) => {
    const now = Math.floor(Date.now() / 1000);
    setExpiredTime(now + days * 24 * 60 * 60);
  };

  /**
   * 提交更新
   */
  const handleSubmit = async () => {
    if (!token) return;

    // 验证必填项
    if (!name.trim()) {
      toast({
        title: '请输入令牌名称',
        variant: 'destructive',
      });
      return;
    }

    if (!groupId) {
      toast({
        title: '请选择分组',
        variant: 'destructive',
      });
      return;
    }

    // 验证 IP 白名单
    if (allowIps.trim()) {
      const result = validateIpWhitelist(allowIps);
      if (!result.valid) {
        toast({
          title: 'IP 白名单验证失败',
          description: result.errors[0] || '请检查格式',
          variant: 'destructive',
        });
        return;
      }

      // 警告但允许继续
      if (result.warnings.length > 0) {
        // 用户需要确认警告
        const confirmed = window.confirm(
          `检测到以下安全警告:\n\n${result.warnings.join('\n')}\n\n是否继续保存？`,
        );
        if (!confirmed) return;
      }
    }

    setUpdating(true);
    try {
      // 构建更新请求
      const request: UpdateRemoteTokenRequest = {
        name: name.trim(),
        group: groupId,
        remain_quota: unlimitedQuota ? 500000 : quota * 500000, // USD -> token
        unlimited_quota: unlimitedQuota,
        expired_time: unlimitedExpire ? -1 : expiredTime,
        model_limits_enabled: modelLimitsEnabled,
        model_limits: modelLimitsEnabled ? modelLimits : '',
        allow_ips: formatIpWhitelist(allowIps), // 格式化后保存
      };

      await onUpdate(provider, token.id, request);

      toast({
        title: '更新成功',
        description: `令牌「${name}」已成功更新`,
      });

      onSuccess();
      onOpenChange(false);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      toast({
        title: '更新失败',
        description: errorMsg,
        variant: 'destructive',
      });
    } finally {
      setUpdating(false);
    }
  };

  if (!token) return null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[600px] max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>编辑令牌</DialogTitle>
          <DialogDescription>修改令牌的配置信息，支持 CIDR 表达式的 IP 白名单</DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* 令牌名称 */}
          <div className="space-y-2">
            <Label htmlFor="token-name">令牌名称 *</Label>
            <Input
              id="token-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="例如: my_api_key"
            />
            <p className="text-xs text-muted-foreground">为令牌设置一个便于识别的名称</p>
          </div>

          {/* 分组选择器 */}
          <div className="space-y-2">
            <Label htmlFor="group-select">分组 *</Label>
            <Select value={groupId} onValueChange={setGroupId}>
              <SelectTrigger id="group-select">
                <SelectValue placeholder="请选择分组" />
              </SelectTrigger>
              <SelectContent>
                {tokenGroups.length === 0 ? (
                  <div className="p-2 text-sm text-muted-foreground text-center">暂无分组</div>
                ) : (
                  tokenGroups.map((group) => (
                    <SelectItem key={group.id} value={group.id}>
                      <div className="flex items-center justify-between gap-4 w-full">
                        <span>{group.id}</span>
                        <span className="text-xs text-muted-foreground">倍率: {group.ratio}</span>
                      </div>
                    </SelectItem>
                  ))
                )}
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">选择令牌所属分组</p>
          </div>

          {/* 额度设置 */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label htmlFor="quota">限额 (美元)</Label>
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="unlimited-quota"
                  checked={unlimitedQuota}
                  onCheckedChange={(checked) => setUnlimitedQuota(checked === true)}
                />
                <label
                  htmlFor="unlimited-quota"
                  className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                >
                  无限额度
                </label>
              </div>
            </div>
            <Input
              id="quota"
              type="number"
              value={quota}
              onChange={(e) => setQuota(Number(e.target.value))}
              placeholder="例如: 100"
              disabled={unlimitedQuota}
            />
            <p className="text-xs text-muted-foreground">设置令牌的剩余使用限额</p>
          </div>

          {/* 有效期设置 */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label htmlFor="expire-days">有效期 (天)</Label>
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="unlimited-expire"
                  checked={unlimitedExpire}
                  onCheckedChange={(checked) => setUnlimitedExpire(checked === true)}
                />
                <label
                  htmlFor="unlimited-expire"
                  className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                >
                  永不过期
                </label>
              </div>
            </div>
            <Input
              id="expire-days"
              type="number"
              value={getExpireDays()}
              onChange={(e) => setExpireDays(Number(e.target.value))}
              placeholder="例如: 365"
              disabled={unlimitedExpire}
            />
            <p className="text-xs text-muted-foreground">设置令牌的剩余有效期</p>
          </div>

          {/* 模型限制 */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label htmlFor="model-limits">模型限制</Label>
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="model-limits-enabled"
                  checked={modelLimitsEnabled}
                  onCheckedChange={(checked) => setModelLimitsEnabled(checked === true)}
                />
                <label
                  htmlFor="model-limits-enabled"
                  className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                >
                  启用限制
                </label>
              </div>
            </div>
            <Input
              id="model-limits"
              value={modelLimits}
              onChange={(e) => setModelLimits(e.target.value)}
              placeholder="例如: gpt-4,gpt-3.5-turbo"
              disabled={!modelLimitsEnabled}
            />
            <p className="text-xs text-muted-foreground">使用逗号分隔多个模型名称</p>
          </div>

          {/* IP 白名单 */}
          <div className="space-y-2">
            <Label htmlFor="allow-ips">IP 白名单（支持 CIDR 表达式）</Label>
            <Textarea
              id="allow-ips"
              value={allowIps}
              onChange={(e) => setAllowIps(e.target.value)}
              placeholder="例如:&#10;192.168.1.1&#10;192.168.1.0/24&#10;2001:db8::/32"
              rows={4}
              className="font-mono text-sm"
            />
            <p className="text-xs text-muted-foreground">
              每行一个 IP 地址或 CIDR 表达式（如 192.168.1.0/24），留空表示不限制
            </p>

            {/* IP 验证结果 */}
            {ipValidationResult && (
              <>
                {/* 错误提示 */}
                {ipValidationResult.errors.length > 0 && (
                  <Alert variant="destructive">
                    <AlertTriangle className="h-4 w-4" />
                    <AlertDescription>
                      <div className="space-y-1">
                        {ipValidationResult.errors.map((error, index) => (
                          <div key={index} className="text-xs">
                            {error}
                          </div>
                        ))}
                      </div>
                    </AlertDescription>
                  </Alert>
                )}

                {/* 警告提示 */}
                {ipValidationResult.warnings.length > 0 && (
                  <Alert>
                    <AlertTriangle className="h-4 w-4" />
                    <AlertDescription>
                      <div className="space-y-1">
                        {ipValidationResult.warnings.map((warning, index) => (
                          <div key={index} className="text-xs">
                            {warning}
                          </div>
                        ))}
                      </div>
                    </AlertDescription>
                  </Alert>
                )}
              </>
            )}
          </div>
        </div>

        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={updating}
          >
            取消
          </Button>
          <Button onClick={handleSubmit} disabled={updating || ipValidationResult?.valid === false}>
            {updating && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            保存
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
