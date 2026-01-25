// Import Token Dialog
//
// 导入令牌为 Profile 对话框

import { useState, useEffect } from 'react';
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
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Loader2, AlertCircle } from 'lucide-react';
import type { Provider } from '@/types/provider';
import type { RemoteToken, TokenImportStatus } from '@/types/remote-token';
import { importTokenAsProfile, checkTokenImportStatus } from '@/lib/tauri-commands/token';
import { pmListToolProfiles } from '@/lib/tauri-commands/profile';
import type { ToolId } from '@/lib/tauri-commands/types';
import { useToast } from '@/hooks/use-toast';

interface ImportTokenDialogProps {
  provider: Provider;
  token: RemoteToken;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSuccess: () => void;
}

const TOOL_OPTIONS = [
  { id: 'claude-code', name: 'Claude Code' },
  { id: 'codex', name: 'Codex' },
  { id: 'gemini-cli', name: 'Gemini CLI' },
];

/**
 * 导入令牌为 Profile 对话框
 */
export function ImportTokenDialog({
  provider,
  token,
  open,
  onOpenChange,
  onSuccess,
}: ImportTokenDialogProps) {
  const { toast } = useToast();
  const [importing, setImporting] = useState(false);
  const [toolId, setToolId] = useState('claude-code');
  const [profileName, setProfileName] = useState('');
  const [importStatus, setImportStatus] = useState<TokenImportStatus[]>([]);
  const [checkingStatus, setCheckingStatus] = useState(false);

  /**
   * Dialog 打开时检测导入状态并设置默认 Profile 名称
   */
  useEffect(() => {
    if (open) {
      // 设置默认 Profile 名称
      if (token.name && !profileName) {
        setProfileName(`${token.name}_profile`);
      }

      // 检测导入状态
      setCheckingStatus(true);
      checkTokenImportStatus(provider.id, token.id)
        .then((status) => {
          setImportStatus(status);

          // 如果当前选择的工具已导入，切换到第一个未导入的工具
          const currentToolStatus = status.find((s) => s.tool_id === toolId);
          if (currentToolStatus?.is_imported) {
            const firstAvailable = status.find((s) => !s.is_imported);
            if (firstAvailable) {
              setToolId(firstAvailable.tool_id);
            }
          }
        })
        .catch((err) => {
          console.error('检测导入状态失败:', err);
          toast({
            title: '检测失败',
            description: '无法检测令牌导入状态，请稍后重试',
            variant: 'destructive',
          });
        })
        .finally(() => {
          setCheckingStatus(false);
        });
    } else {
      // Dialog 关闭时重置状态
      setImportStatus([]);
      setProfileName('');
      setToolId('claude-code');
    }
  }, [open, token.id, token.name, provider.id, profileName, toolId, toast]);

  /**
   * 提交导入
   */
  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    // 验证表单
    if (!profileName.trim()) {
      toast({
        title: '验证失败',
        description: '请输入 Profile 名称',
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

    // 检查当前工具是否已导入
    const currentToolStatus = importStatus.find((s) => s.tool_id === toolId);
    if (currentToolStatus?.is_imported) {
      toast({
        title: '验证失败',
        description: `该令牌已导入到 ${currentToolStatus.imported_profile_name}，请选择其他工具`,
        variant: 'destructive',
      });
      return;
    }

    setImporting(true);
    try {
      // 检查是否已存在同名 Profile
      const existingProfiles = await pmListToolProfiles(toolId as ToolId);
      if (existingProfiles.includes(profileName)) {
        const toolName = TOOL_OPTIONS.find((t) => t.id === toolId)?.name || toolId;
        toast({
          title: '验证失败',
          description: `Profile「${profileName}」已存在于 ${toolName} 中，请使用其他名称`,
          variant: 'destructive',
        });
        setImporting(false);
        return;
      }

      await importTokenAsProfile(provider, token, toolId, profileName);
      toast({
        title: '导入成功',
        description: `令牌「${token.name}」已成功导入为 Profile「${profileName}」`,
      });
      onSuccess();
      // 重置表单
      setProfileName('');
      setToolId('claude-code');
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      toast({
        title: '导入失败',
        description: errorMsg,
        variant: 'destructive',
      });
    } finally {
      setImporting(false);
    }
  };

  // 检查是否所有工具都已导入
  const allToolsImported = importStatus.length === 3 && importStatus.every((s) => s.is_imported);

  // 获取工具的导入状态
  const getToolStatus = (toolIdToCheck: string) => {
    return importStatus.find((s) => s.tool_id === toolIdToCheck);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>导入令牌为 Profile</DialogTitle>
          <DialogDescription>将令牌「{token.name}」导入为本地 Profile 配置</DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4">
          {/* 全部导入提示 */}
          {allToolsImported && (
            <Alert>
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>
                该令牌已导入到所有工具，无法再次导入。您可以查看令牌详情或管理现有的 Profile 配置。
              </AlertDescription>
            </Alert>
          )}

          {/* 令牌信息 */}
          <div className="rounded-md border bg-muted/50 p-3 space-y-1">
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">令牌名称:</span>
              <span className="font-medium">{token.name}</span>
            </div>
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">分组:</span>
              <span>{token.group}</span>
            </div>
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">额度:</span>
              <span>
                {token.unlimited_quota
                  ? '无限'
                  : `$${(token.remain_quota / 500000).toFixed(2)} / $${((token.remain_quota + token.used_quota) / 500000).toFixed(2)}`}
              </span>
            </div>
          </div>

          {/* 选择工具 */}
          <div className="space-y-2">
            <Label>目标工具 *</Label>
            <TooltipProvider>
              <Tabs value={toolId} onValueChange={setToolId}>
                <TabsList className="grid w-full grid-cols-3">
                  {TOOL_OPTIONS.map((tool) => {
                    const status = getToolStatus(tool.id);
                    const isDisabled = status?.is_imported || checkingStatus;
                    const tooltipText = status?.is_imported
                      ? `该令牌已导入到 ${status.imported_profile_name}`
                      : checkingStatus
                        ? '正在检测导入状态...'
                        : '';

                    return (
                      <Tooltip key={tool.id}>
                        <TooltipTrigger asChild>
                          <div>
                            <TabsTrigger value={tool.id} disabled={isDisabled} className="w-full">
                              {tool.name}
                            </TabsTrigger>
                          </div>
                        </TooltipTrigger>
                        {tooltipText && (
                          <TooltipContent>
                            <p>{tooltipText}</p>
                          </TooltipContent>
                        )}
                      </Tooltip>
                    );
                  })}
                </TabsList>
              </Tabs>
            </TooltipProvider>
            <p className="text-xs text-muted-foreground">选择要导入到哪个工具的 Profile 配置</p>
          </div>

          {/* Profile 名称 */}
          <div className="space-y-2">
            <Label htmlFor="profile-name">Profile 名称 *</Label>
            <Input
              id="profile-name"
              value={profileName}
              onChange={(e) => setProfileName(e.target.value)}
              required
            />
            <p className="text-xs text-muted-foreground">为导入的 Profile 设置一个本地名称</p>
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={importing}
            >
              取消
            </Button>
            <Button
              type="submit"
              disabled={
                importing ||
                allToolsImported ||
                checkingStatus ||
                importStatus.find((s) => s.tool_id === toolId)?.is_imported
              }
            >
              {importing && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              导入
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
