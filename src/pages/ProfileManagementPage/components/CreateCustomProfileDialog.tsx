/**
 * 自定义 Profile 创建对话框
 *
 * 手动输入 API Key 和 Base URL 创建 Profile
 */

import { useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Loader2, Sparkles, X } from 'lucide-react';
import { useToast } from '@/hooks/use-toast';
import type { ToolId } from '@/types/profile';
import { ProfileNameInput } from './ProfileNameInput';
import { pmListToolProfiles } from '@/lib/tauri-commands/profile';
import { createCustomProfile } from '@/lib/tauri-commands/token';

interface CreateCustomProfileDialogProps {
  /** 对话框打开状态 */
  open: boolean;
  /** 对话框状态变更回调 */
  onOpenChange: (open: boolean) => void;
  /** 当前工具 ID */
  toolId: ToolId;
  /** 创建成功回调 */
  onSuccess: () => void;
  /** 一键配置回调（打开从供应商导入对话框） */
  onQuickSetup: () => void;
}

/**
 * 自定义 Profile 创建对话框
 */
export function CreateCustomProfileDialog({
  open,
  onOpenChange,
  toolId,
  onSuccess,
  onQuickSetup,
}: CreateCustomProfileDialogProps) {
  const { toast } = useToast();

  // 表单数据
  const [profileName, setProfileName] = useState('');
  const [baseUrl, setBaseUrl] = useState('https://jp.duckcoding.com/');
  const [apiKey, setApiKey] = useState('');
  const [wireApi, setWireApi] = useState('responses'); // Codex 特定
  const [model, setModel] = useState(''); // Gemini 特定

  // UI 状态
  const [creating, setCreating] = useState(false);
  const [bannerVisible, setBannerVisible] = useState(true);

  /**
   * Dialog 打开时重置状态
   */
  const handleOpenChange = (isOpen: boolean) => {
    if (isOpen) {
      // 重置表单
      setProfileName('');
      setBaseUrl('https://jp.duckcoding.com/');
      setApiKey('');
      setWireApi('responses');
      setModel('');
      // 重置横幅状态（每次打开时都显示）
      setBannerVisible(true);
    }
    onOpenChange(isOpen);
  };

  /**
   * 一键配置按钮点击
   */
  const handleQuickSetup = () => {
    onOpenChange(false); // 关闭当前对话框
    onQuickSetup(); // 触发父组件打开供应商导入对话框
  };

  /**
   * 表单提交
   */
  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    // 验证必填项
    if (!profileName.trim()) {
      toast({
        title: '请输入 Profile 名称',
        variant: 'destructive',
      });
      return;
    }

    if (!baseUrl.trim()) {
      toast({
        title: '请输入 Base URL',
        variant: 'destructive',
      });
      return;
    }

    if (!apiKey.trim()) {
      toast({
        title: '请输入 API Key',
        variant: 'destructive',
      });
      return;
    }

    // 检查保留前缀
    if (profileName.startsWith('dc_proxy_')) {
      toast({
        title: '验证失败',
        description: 'Profile 名称不能以 dc_proxy_ 开头（系统保留）',
        variant: 'destructive',
      });
      return;
    }

    setCreating(true);
    try {
      // 检查是否已存在同名 Profile
      const existingProfiles = await pmListToolProfiles(toolId);
      if (existingProfiles.includes(profileName)) {
        toast({
          title: '验证失败',
          description: '该 Profile 名称已存在，请使用其他名称',
          variant: 'destructive',
        });
        setCreating(false);
        return;
      }

      // 构建额外配置
      const extraConfig: { wire_api?: string; model?: string } = {};
      if (toolId === 'codex') {
        extraConfig.wire_api = wireApi;
      } else if (toolId === 'gemini-cli' && model.trim()) {
        extraConfig.model = model;
      }

      // 调用创建 API
      await createCustomProfile(toolId, profileName, apiKey, baseUrl, extraConfig);

      toast({
        title: '创建成功',
        description: `Profile「${profileName}」已成功创建`,
      });

      onSuccess();
      onOpenChange(false);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      toast({
        title: '创建失败',
        description: errorMsg,
        variant: 'destructive',
      });
    } finally {
      setCreating(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>创建自定义 Profile</DialogTitle>
          <DialogDescription>手动输入 API Key 和 Base URL 创建本地 Profile 配置</DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4">
          {/* 推荐横幅 */}
          {bannerVisible && (
            <Alert>
              <Sparkles className="h-4 w-4" />
              <AlertDescription className="flex items-start justify-between gap-2">
                <div className="flex-1">
                  <p className="font-semibold mb-1">推荐使用 DuckCoding 一键配置</p>
                  <p className="text-sm mb-2">
                    无需手动填写，自动获取最新 API Key 并导入为 Profile
                  </p>
                  <Button
                    type="button"
                    onClick={handleQuickSetup}
                    variant="outline"
                    size="sm"
                    className="w-full"
                  >
                    <Sparkles className="mr-2 h-4 w-4" />
                    一键配置
                  </Button>
                </div>
                <Button
                  type="button"
                  onClick={() => setBannerVisible(false)}
                  variant="ghost"
                  size="sm"
                  className="shrink-0 h-6 w-6 p-0"
                >
                  <X className="h-4 w-4" />
                  <span className="sr-only">关闭</span>
                </Button>
              </AlertDescription>
            </Alert>
          )}

          {/* Profile 名称 */}
          <ProfileNameInput
            value={profileName}
            onChange={setProfileName}
            placeholder="例如: my_custom_profile"
          />

          {/* Base URL */}
          <div className="space-y-2">
            <Label htmlFor="base-url">Base URL *</Label>
            <Input
              id="base-url"
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
              placeholder="例如: https://api.example.com"
              required
            />
            <p className="text-xs text-muted-foreground">API 端点的基础 URL</p>
          </div>

          {/* API Key */}
          <div className="space-y-2">
            <Label htmlFor="api-key">API Key *</Label>
            <Input
              id="api-key"
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="输入您的 API Key"
              required
            />
            <p className="text-xs text-muted-foreground">您的 API 访问密钥</p>
          </div>

          {/* Codex 特定配置 */}
          {toolId === 'codex' && (
            <div className="space-y-2">
              <Label htmlFor="wire-api">Wire API *</Label>
              <select
                id="wire-api"
                value={wireApi}
                onChange={(e) => setWireApi(e.target.value)}
                className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
              >
                <option value="responses">responses</option>
                <option value="chat">chat</option>
              </select>
              <p className="text-xs text-muted-foreground">选择 Codex API 类型</p>
            </div>
          )}

          {/* Gemini 特定配置 */}
          {toolId === 'gemini-cli' && (
            <div className="space-y-2">
              <Label htmlFor="model">Model（可选）</Label>
              <Input
                id="model"
                value={model}
                onChange={(e) => setModel(e.target.value)}
                placeholder="例如: gemini-2.0-flash-exp"
              />
              <p className="text-xs text-muted-foreground">指定模型名称（留空则使用原有配置）</p>
            </div>
          )}

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={creating}
            >
              取消
            </Button>
            <Button type="submit" disabled={creating}>
              {creating && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              创建
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
