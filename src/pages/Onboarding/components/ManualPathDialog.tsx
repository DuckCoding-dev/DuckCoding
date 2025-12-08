import { useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import {
  Dialog,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '@/components/ui/dialog';
import * as DialogPrimitive from '@radix-ui/react-dialog';
import { X, InfoIcon, Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { validateToolPath } from '@/lib/tauri-commands';
import { useToast } from '@/hooks/use-toast';
import { cn } from '@/lib/utils';

interface ManualPathDialogProps {
  toolId: string;
  toolName: string;
  onConfirm: (toolId: string, path: string) => Promise<void>;
  onCancel: () => void;
}

export function ManualPathDialog({ toolId, toolName, onConfirm, onCancel }: ManualPathDialogProps) {
  const [path, setPath] = useState('');
  const [validating, setValidating] = useState(false);
  const [validationError, setValidationError] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const { toast } = useToast();

  // 获取常见安装路径示例
  const getCommonPaths = () => {
    const isWindows = navigator.platform.toLowerCase().includes('win');
    if (isWindows) {
      return [
        `C:\\Users\\用户名\\AppData\\Roaming\\npm\\${toolId}.cmd`,
        `C:\\Users\\用户名\\.npm-global\\${toolId}.cmd`,
        `C:\\Program Files\\${toolName}\\${toolId}.exe`,
      ];
    } else {
      return [
        `~/.npm-global/bin/${toolId}`,
        `/usr/local/bin/${toolId}`,
        `/opt/homebrew/bin/${toolId}`,
        `~/.local/bin/${toolId}`,
      ];
    }
  };

  // 打开文件选择器
  const handleBrowse = async () => {
    try {
      const isWindows = navigator.platform.toLowerCase().includes('win');
      const selected = await open({
        directory: false,
        multiple: false,
        title: `选择 ${toolName} 可执行文件`,
        filters: [
          {
            name: '可执行文件',
            extensions: isWindows ? ['exe', 'cmd', 'bat'] : [],
          },
        ],
      });

      if (selected && typeof selected === 'string') {
        setPath(selected);
        handleValidate(selected);
      }
    } catch (error) {
      toast({
        variant: 'destructive',
        title: '打开文件选择器失败',
        description: String(error),
      });
    }
  };

  // 验证路径
  const handleValidate = async (pathToValidate: string) => {
    if (!pathToValidate.trim()) {
      setValidationError('请输入路径');
      return;
    }

    setValidating(true);
    setValidationError(null);

    try {
      await validateToolPath(toolId, pathToValidate);
      // 验证成功，清除错误
    } catch (error) {
      setValidationError(String(error));
    } finally {
      setValidating(false);
    }
  };

  // 确认保存
  const handleConfirm = async () => {
    if (!path || validating || validationError) return;

    setConfirming(true);
    try {
      await onConfirm(toolId, path);
    } catch (error) {
      toast({
        variant: 'destructive',
        title: '保存路径失败',
        description: String(error),
      });
    } finally {
      setConfirming(false);
    }
  };

  return (
    <Dialog open={true} onOpenChange={(open) => !open && onCancel()}>
      <DialogPrimitive.Portal>
        {/* 自定义 Overlay，z-index 高于 Onboarding (10000) */}
        <DialogPrimitive.Overlay
          className={cn(
            'fixed inset-0 bg-black/80 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0',
          )}
          style={{ zIndex: 10001 }}
        />
        {/* 自定义 Content，z-index 高于 Overlay */}
        <DialogPrimitive.Content
          className={cn(
            'fixed left-[50%] top-[50%] grid w-full max-w-lg translate-x-[-50%] translate-y-[-50%] gap-4 border bg-background p-6 shadow-lg duration-200 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[state=closed]:slide-out-to-left-1/2 data-[state=closed]:slide-out-to-top-[48%] data-[state=open]:slide-in-from-left-1/2 data-[state=open]:slide-in-from-top-[48%] sm:rounded-lg',
          )}
          style={{ zIndex: 10002 }}
        >
          {/* 关闭按钮 */}
          <DialogPrimitive.Close className="absolute right-4 top-4 rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none data-[state=open]:bg-accent data-[state=open]:text-muted-foreground">
            <X className="h-4 w-4" />
            <span className="sr-only">Close</span>
          </DialogPrimitive.Close>

          <DialogHeader>
            <DialogTitle>手动指定 {toolName} 路径</DialogTitle>
            <DialogDescription>自动扫描未找到该工具，请选择可执行文件路径</DialogDescription>
          </DialogHeader>

          <div className="space-y-4">
            {/* 常见路径提示 */}
            <Alert>
              <InfoIcon className="h-4 w-4" />
              <AlertTitle>常见安装路径</AlertTitle>
              <AlertDescription>
                <ul className="list-disc list-inside text-sm space-y-1">
                  {getCommonPaths().map((examplePath, index) => (
                    <li key={index} className="font-mono">
                      {examplePath}
                    </li>
                  ))}
                </ul>
              </AlertDescription>
            </Alert>

            {/* 路径输入 */}
            <div className="flex gap-2">
              <Input
                value={path}
                onChange={(e) => {
                  setPath(e.target.value);
                  setValidationError(null);
                }}
                onBlur={() => {
                  if (path) handleValidate(path);
                }}
                placeholder="输入或浏览选择可执行文件"
                disabled={validating || confirming}
              />
              <Button onClick={handleBrowse} variant="outline" disabled={validating || confirming}>
                浏览...
              </Button>
            </div>

            {/* 验证状态 */}
            {validating && (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Loader2 className="h-4 w-4 animate-spin" />
                验证中...
              </div>
            )}

            {validationError && (
              <Alert variant="destructive">
                <AlertDescription>{validationError}</AlertDescription>
              </Alert>
            )}
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={onCancel} disabled={confirming}>
              取消
            </Button>
            <Button
              onClick={handleConfirm}
              disabled={!path || validating || !!validationError || confirming}
            >
              {confirming ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  保存中...
                </>
              ) : (
                '确认'
              )}
            </Button>
          </DialogFooter>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </Dialog>
  );
}
