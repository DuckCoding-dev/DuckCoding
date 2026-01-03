import { useState, useEffect } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Loader2, CheckCircle2, XCircle, User, Info } from 'lucide-react';
import type { Provider } from '@/lib/tauri-commands';
import { validateProviderConfig } from '@/lib/tauri-commands';
import { openExternalLink } from '@/utils/formatting.ts';

interface ProviderFormDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  provider: Provider | null;
  onSubmit: (provider: Provider) => Promise<void>;
  isEditing: boolean;
}

export function ProviderFormDialog({
  open,
  onOpenChange,
  provider,
  onSubmit,
  isEditing,
}: ProviderFormDialogProps) {
  const [formData, setFormData] = useState({
    id: '',
    name: '',
    website_url: '',
    user_id: '',
    access_token: '',
    is_default: false,
  });
  const [saving, setSaving] = useState(false);
  const [validating, setValidating] = useState(false);
  const [validationResult, setValidationResult] = useState<{
    success: boolean;
    username?: string;
    error?: string;
  } | null>(null);

  useEffect(() => {
    if (provider) {
      setFormData({
        id: provider.id,
        name: provider.name,
        website_url: provider.website_url,
        user_id: provider.user_id,
        access_token: provider.access_token,
        is_default: provider.is_default,
      });
    } else {
      setFormData({
        id: '',
        name: '',
        website_url: 'https://duckcoding.com',
        user_id: '',
        access_token: '',
        is_default: false,
      });
    }
    setValidationResult(null);
  }, [provider, open]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSaving(true);
    try {
      const now = Math.floor(Date.now() / 1000);
      // 创建新供应商时自动生成 ID（基于名称的小写字母 + 时间戳）
      const providerId = isEditing
        ? formData.id
        : `${formData.name.toLowerCase().replace(/\s+/g, '-')}-${Date.now()}`;

      const providerData: Provider = {
        ...formData,
        id: providerId,
        username: provider?.username || validationResult?.username,
        created_at: provider?.created_at || now,
        updated_at: now,
      };
      await onSubmit(providerData);
      onOpenChange(false);
    } catch (error) {
      console.error('保存供应商失败:', error);
    } finally {
      setSaving(false);
    }
  };

  const handleValidate = async () => {
    setValidating(true);
    setValidationResult(null);
    try {
      const now = Math.floor(Date.now() / 1000);
      const testProvider: Provider = {
        ...formData,
        created_at: now,
        updated_at: now,
      };

      const result = await validateProviderConfig(testProvider);
      setValidationResult(result);
    } catch (error) {
      setValidationResult({
        success: false,
        error: String(error),
      });
    } finally {
      setValidating(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>{isEditing ? '编辑供应商' : '添加供应商'}</DialogTitle>
          <DialogDescription>
            {isEditing ? '修改供应商配置信息' : '配置新的 AI 服务供应商'}
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit}>
          <div className="space-y-4 py-4">
            {/* 供应商名称 */}
            <div className="space-y-2">
              <Label htmlFor="name">供应商名称</Label>
              <Input
                id="name"
                value={formData.name}
                onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                placeholder="例如: DuckCoding"
                required
              />
            </div>

            {/* 官网地址 */}
            <div className="space-y-2">
              <Label htmlFor="website_url">官网地址</Label>
              <Input
                id="website_url"
                type="url"
                value={formData.website_url}
                onChange={(e) => setFormData({ ...formData, website_url: e.target.value })}
                placeholder="https://duckcoding.com"
                required
              />
            </div>

            {/* 用户 ID */}
            <div className="space-y-2">
              <Label htmlFor="user_id">用户 ID</Label>
              <Input
                id="user_id"
                value={formData.user_id}
                onChange={(e) => setFormData({ ...formData, user_id: e.target.value })}
                placeholder="您的用户 ID"
                required
              />
            </div>

            {/* 访问令牌 */}
            <div className="space-y-2">
              <Label htmlFor="access_token">访问令牌</Label>
              <Input
                id="access_token"
                type="password"
                value={formData.access_token}
                onChange={(e) => setFormData({ ...formData, access_token: e.target.value })}
                placeholder="您的访问令牌"
                required
              />
            </div>

            <div className="rounded-lg bg-blue-50 dark:bg-blue-950/30 border border-blue-200 dark:border-blue-800 p-4">
              <div className="flex items-start gap-2">
                <Info className="h-4 w-4 text-blue-600 dark:text-blue-400 flex-shrink-0 mt-0.5" />
                <div className="space-y-2 flex-1">
                  <p className="text-sm font-semibold text-blue-800 dark:text-blue-200">
                    如何获取用户 ID 和系统访问令牌？
                  </p>
                  <p className="text-xs text-blue-700 dark:text-blue-300">
                    请访问 对应平台的[控制台-{'>'}个人设置-{'>'}安全设置-{'>'}系统访问令牌]
                    获取您的凭证信息
                  </p>
                  <p className="text-sm font-semibold text-blue-800 dark:text-blue-200">
                    DuckCoding用户？
                  </p>
                  <button
                    onClick={() => openExternalLink('https://duckcoding.com/console/personal')}
                    className="text-xs text-blue-600 dark:text-blue-400 hover:underline font-medium"
                  >
                    前往控制台 →
                  </button>
                </div>
              </div>
            </div>

            {/* 验证结果 */}
            {validationResult && (
              <div
                className={`flex items-start gap-2 p-3 rounded-lg text-sm ${
                  validationResult.success
                    ? 'bg-green-50 text-green-700 dark:bg-green-950 dark:text-green-300'
                    : 'bg-red-50 text-red-700 dark:bg-red-950 dark:text-red-300'
                }`}
              >
                {validationResult.success ? (
                  <CheckCircle2 className="h-4 w-4 mt-0.5 flex-shrink-0" />
                ) : (
                  <XCircle className="h-4 w-4 mt-0.5 flex-shrink-0" />
                )}
                <div className="flex-1">
                  {validationResult.success ? (
                    <>
                      <p className="font-medium">配置验证通过</p>
                      {validationResult.username && (
                        <div className="mt-2 flex items-center gap-2 bg-white/50 dark:bg-black/20 rounded px-2 py-1.5">
                          <User className="h-3.5 w-3.5" />
                          <span className="text-xs">
                            用户名: <strong>{validationResult.username}</strong>
                          </span>
                        </div>
                      )}
                    </>
                  ) : (
                    <>
                      <p className="font-medium">验证失败</p>
                      <p className="mt-1 text-xs opacity-90">{validationResult.error}</p>
                    </>
                  )}
                </div>
              </div>
            )}
          </div>

          <DialogFooter className="gap-2">
            <Button type="button" variant="outline" onClick={handleValidate} disabled={validating}>
              {validating ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  验证中...
                </>
              ) : (
                '验证配置'
              )}
            </Button>
            <Button type="submit" disabled={saving}>
              {saving ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  保存中...
                </>
              ) : isEditing ? (
                '保存修改'
              ) : (
                '创建供应商'
              )}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
