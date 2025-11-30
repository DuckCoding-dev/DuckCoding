import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { InfoIcon } from 'lucide-react';

interface VersionManagementDialogProps {
  open: boolean;
  onClose: () => void;
  instanceId: string;
  toolName: string;
}

export function VersionManagementDialog({
  open,
  onClose,
  instanceId,
  toolName,
}: VersionManagementDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{toolName} - 版本管理</DialogTitle>
        </DialogHeader>

        <Alert>
          <InfoIcon className="h-4 w-4" />
          <AlertDescription>
            版本管理功能正在开发中，敬请期待。
            <br />
            该功能将支持：
            <ul className="list-disc list-inside mt-2 text-sm space-y-1">
              <li>查看历史版本列表</li>
              <li>回退到指定版本</li>
              <li>版本锁定功能</li>
              <li>版本更新记录</li>
            </ul>
          </AlertDescription>
        </Alert>

        {/* 预留UI结构 */}
        <div className="rounded border p-8 bg-muted/30 text-center">
          <p className="text-sm text-muted-foreground mb-2">版本列表将在此处显示</p>
          <p className="text-xs text-muted-foreground">Instance ID: {instanceId}</p>
        </div>
      </DialogContent>
    </Dialog>
  );
}
