/**
 * Profile 编辑器对话框
 */

import { useState, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Info, Sparkles, Loader2, ExternalLink } from 'lucide-react';
import { generateApiKeyForTool, getGlobalConfig } from '@/lib/tauri-commands';
import { useToast } from '@/hooks/use-toast';
import { openExternalLink } from '@/utils/formatting';
import { groupNameMap } from '@/utils/constants';
import type { ProfileFormData, ToolId } from '@/types/profile';
import { TOOL_NAMES } from '@/types/profile';

interface ProfileEditorProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  toolId: ToolId;
  mode: 'create' | 'edit';
  initialData?: ProfileFormData;
  onSave: (data: ProfileFormData) => Promise<void>;
}

export function ProfileEditor({
  open,
  onOpenChange,
  toolId,
  mode,
  initialData,
  onSave,
}: ProfileEditorProps) {
  const { toast } = useToast();
  const [formData, setFormData] = useState<ProfileFormData>({
    name: '',
    api_key: '',
    base_url: getDefaultBaseUrl(toolId),
    wire_api: toolId === 'codex' ? 'responses' : undefined,
    model: undefined, // Gemini 默认不设置 model
  });
  const [loading, setLoading] = useState(false);
  const [generatingKey, setGeneratingKey] = useState(false);
  const [apiProvider, setApiProvider] = useState<'duckcoding' | 'custom'>('duckcoding');

  useEffect(() => {
    if (initialData) {
      setFormData(initialData);
      // 根据 base_url 判断是否为 DuckCoding 提供商
      const isDuckCoding = initialData.base_url?.includes('duckcoding.com');
      setApiProvider(isDuckCoding ? 'duckcoding' : 'custom');
    } else {
      // Codex 默认 wire_api 为 "responses"
      const defaultWireApi = toolId === 'codex' ? 'responses' : undefined;
      setFormData({
        name: '',
        api_key: '',
        base_url: getDefaultBaseUrl(toolId, apiProvider),
        wire_api: defaultWireApi,
        model: undefined, // Gemini 默认不设置 model
      });
    }
  }, [initialData, toolId, open, apiProvider]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);

    try {
      await onSave(formData);
      onOpenChange(false);
    } catch (error) {
      // 错误已在 Hook 中处理
      console.error('保存失败:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleChange = (field: keyof ProfileFormData, value: string) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
  };

  // 处理 API 提供商变化
  const handleProviderChange = (provider: 'duckcoding' | 'custom') => {
    setApiProvider(provider);
    // 根据提供商更新 Base URL
    const newBaseUrl = getDefaultBaseUrl(toolId, provider);
    const updatedData: Partial<ProfileFormData> = {
      base_url: newBaseUrl,
    };

    // Codex provider 在切换 API 提供商时不改变，保持用户选择
    setFormData((prev) => ({ ...prev, ...updatedData }));
  };

  // 一键生成 API Key
  const handleGenerateApiKey = async () => {
    try {
      setGeneratingKey(true);

      // 检查全局配置
      const config = await getGlobalConfig();
      if (!config?.user_id || !config?.system_token) {
        toast({
          title: '缺少配置',
          description: '请先在设置中配置用户 ID 和系统访问令牌',
          variant: 'destructive',
        });
        window.dispatchEvent(new CustomEvent('navigate-to-settings'));
        return;
      }

      // 生成 API Key
      const result = await generateApiKeyForTool(toolId);

      if (result.success && result.api_key) {
        handleChange('api_key', result.api_key);
        toast({
          title: '生成成功',
          description: 'API Key 已自动填入配置框',
        });
      } else {
        toast({
          title: '生成失败',
          description: result.message || '未知错误',
          variant: 'destructive',
        });
      }
    } catch (error) {
      toast({
        title: '生成失败',
        description: String(error),
        variant: 'destructive',
      });
    } finally {
      setGeneratingKey(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px]">
        <form onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>
              {mode === 'create' ? '创建' : '编辑'} Profile - {TOOL_NAMES[toolId]}
            </DialogTitle>
            <DialogDescription>
              {mode === 'create' ? '填写以下信息以创建新的 Profile' : '修改 Profile 配置信息'}
            </DialogDescription>
          </DialogHeader>

          <div className="grid gap-4 py-4">
            {/* Profile 名称 */}
            <div className="grid gap-2">
              <Label htmlFor="name">Profile 名称</Label>
              <Input
                id="name"
                value={formData.name}
                onChange={(e) => handleChange('name', e.target.value)}
                placeholder="例如: default, work, personal"
                required
                disabled={mode === 'edit'}
              />
            </div>

            {/* API 提供商（仅创建时显示） */}
            {mode === 'create' && (
              <div className="grid gap-2">
                <Label htmlFor="api_provider">API 提供商</Label>
                <Select value={apiProvider} onValueChange={handleProviderChange}>
                  <SelectTrigger id="api_provider">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="duckcoding">DuckCoding (推荐)</SelectItem>
                    <SelectItem value="custom">自定义端点</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            )}

            {/* DuckCoding 提示信息（仅创建时显示） */}
            {mode === 'create' && apiProvider === 'duckcoding' && (
              <Alert>
                <Info className="h-4 w-4" />
                <AlertTitle>DuckCoding API Key 分组说明</AlertTitle>
                <AlertDescription className="text-sm space-y-2">
                  <div>
                    <p className="font-semibold mb-1">当前工具需要使用：</p>
                    <p className="font-mono bg-muted px-2 py-1 rounded inline-block">
                      {groupNameMap[toolId]} 分组
                    </p>
                  </div>
                  <ul className="list-disc list-inside space-y-1">
                    <li>每个工具必须使用其专用分组的 API Key</li>
                    <li>API Key 不能混用</li>
                  </ul>
                  <div>
                    <p className="font-semibold mb-1">获取 API Key：</p>
                    <button
                      type="button"
                      onClick={() =>
                        openExternalLink('https://duckcoding.com/console/api-providers')
                      }
                      className="inline-flex items-center gap-1 text-primary hover:underline font-medium cursor-pointer bg-transparent border-0 p-0"
                    >
                      访问 DuckCoding 控制台 <ExternalLink className="h-3 w-3" />
                    </button>
                  </div>
                </AlertDescription>
              </Alert>
            )}

            {/* API Key */}
            <div className="grid gap-2">
              <Label htmlFor="api_key">API Key {mode === 'edit' && '(留空表示不修改)'}</Label>
              <div className="flex gap-2">
                <Input
                  id="api_key"
                  type="password"
                  value={formData.api_key}
                  onChange={(e) => handleChange('api_key', e.target.value)}
                  placeholder={mode === 'edit' ? '留空不修改' : '输入 API Key'}
                  required={mode === 'create'}
                  className="flex-1"
                />
                {mode === 'create' && apiProvider === 'duckcoding' && (
                  <Button
                    type="button"
                    onClick={handleGenerateApiKey}
                    disabled={generatingKey}
                    variant="outline"
                    className="shrink-0"
                    title="一键生成 DuckCoding API Key"
                  >
                    {generatingKey ? (
                      <>
                        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                        生成中...
                      </>
                    ) : (
                      <>
                        <Sparkles className="mr-2 h-4 w-4" />
                        一键生成
                      </>
                    )}
                  </Button>
                )}
              </div>
              {mode === 'create' && apiProvider === 'duckcoding' && (
                <p className="text-xs text-muted-foreground">
                  点击"一键生成"可自动创建 DuckCoding API Key（需先配置全局设置）
                </p>
              )}
            </div>

            {/* Base URL */}
            <div className="grid gap-2">
              <Label htmlFor="base_url">Base URL</Label>
              <Input
                id="base_url"
                value={formData.base_url}
                onChange={(e) => handleChange('base_url', e.target.value)}
                placeholder="API 端点地址"
                required
                disabled={mode === 'create' && apiProvider === 'duckcoding'}
              />
              {mode === 'create' && apiProvider === 'duckcoding' && (
                <p className="text-xs text-muted-foreground">DuckCoding 提供商使用默认端点地址</p>
              )}
            </div>

            {/* Codex 特定：Wire API */}
            {toolId === 'codex' && (
              <div className="grid gap-2">
                <Label htmlFor="wire_api">Wire API</Label>
                <Select
                  value={formData.wire_api}
                  onValueChange={(value) => handleChange('wire_api', value)}
                >
                  <SelectTrigger id="wire_api">
                    <SelectValue placeholder="选择 Wire API" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="responses">responses</SelectItem>
                    <SelectItem value="chat">chat</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            )}

            {/* Gemini 特定：Model */}
            {toolId === 'gemini-cli' && (
              <div className="grid gap-2">
                <Label htmlFor="model">模型 (可选)</Label>
                <Select
                  value={formData.model || 'none'}
                  onValueChange={(value) => handleChange('model', value === 'none' ? '' : value)}
                >
                  <SelectTrigger id="model">
                    <SelectValue placeholder="不设置 (保留原有配置)" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="none">不设置 (保留原有配置)</SelectItem>
                    <SelectItem value="gemini-2.0-flash-exp">Gemini 2.0 Flash (Exp)</SelectItem>
                    <SelectItem value="gemini-1.5-pro">Gemini 1.5 Pro</SelectItem>
                    <SelectItem value="gemini-1.5-flash">Gemini 1.5 Flash</SelectItem>
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  不设置时，将保留工具原生配置文件中的模型设置
                </p>
              </div>
            )}
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={loading}
            >
              取消
            </Button>
            <Button type="submit" disabled={loading || (mode === 'create' && !formData.api_key)}>
              {loading ? '保存中...' : '保存'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

// ==================== 辅助函数 ====================

function getDefaultBaseUrl(
  toolId: ToolId,
  provider: 'duckcoding' | 'custom' = 'duckcoding',
): string {
  if (provider === 'custom') {
    // 自定义端点返回空字符串，让用户填写
    switch (toolId) {
      case 'claude-code':
        return 'https://api.anthropic.com';
      case 'codex':
        return 'https://api.openai.com/v1';
      case 'gemini-cli':
        return 'https://generativelanguage.googleapis.com';
      default:
        return '';
    }
  }

  // DuckCoding 提供商返回 DuckCoding 端点
  switch (toolId) {
    case 'claude-code':
      return 'https://jp.duckcoding.com';
    case 'codex':
      return 'https://jp.duckcoding.com/v1';
    case 'gemini-cli':
      return 'https://jp.duckcoding.com';
    default:
      return '';
  }
}
