import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import { Settings as SettingsIcon, Info } from 'lucide-react';
import { openExternalLink } from '@/utils/formatting';

interface BasicSettingsTabProps {
  userId: string;
  setUserId: (value: string) => void;
  systemToken: string;
  setSystemToken: (value: string) => void;
}

export function BasicSettingsTab({
  userId,
  setUserId,
  systemToken,
  setSystemToken,
}: BasicSettingsTabProps) {
  return (
    <div className="space-y-4 rounded-lg border p-6">
      <div className="flex items-center gap-2">
        <SettingsIcon className="h-5 w-5" />
        <h3 className="text-lg font-semibold">DuckCoding 账户</h3>
      </div>
      <Separator />

      <div className="space-y-4">
        <div className="space-y-2">
          <Label htmlFor="user-id">用户 ID *</Label>
          <Input
            id="user-id"
            placeholder="请输入您的用户 ID"
            value={userId}
            onChange={(e) => setUserId(e.target.value)}
            className="shadow-sm"
          />
          <p className="text-xs text-muted-foreground">用于识别您的账户和一键生成 API Key</p>
        </div>

        <div className="space-y-2">
          <Label htmlFor="system-token">系统访问令牌 *</Label>
          <Input
            id="system-token"
            type="password"
            placeholder="请输入系统访问令牌"
            value={systemToken}
            onChange={(e) => setSystemToken(e.target.value)}
            className="shadow-sm"
          />
          <p className="text-xs text-muted-foreground">用于验证您的身份和调用系统 API</p>
        </div>

        <div className="rounded-lg bg-blue-50 dark:bg-blue-950/30 border border-blue-200 dark:border-blue-800 p-4">
          <div className="flex items-start gap-2">
            <Info className="h-4 w-4 text-blue-600 dark:text-blue-400 flex-shrink-0 mt-0.5" />
            <div className="space-y-2 flex-1">
              <p className="text-sm font-semibold text-blue-800 dark:text-blue-200">
                如何获取用户 ID 和系统访问令牌？
              </p>
              <p className="text-xs text-blue-700 dark:text-blue-300">
                请访问 DuckCoding 控制台获取您的凭证信息
              </p>
              <button
                onClick={() => openExternalLink('https://duckcoding.com/console')}
                className="text-xs text-blue-600 dark:text-blue-400 hover:underline font-medium"
              >
                前往控制台 →
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
