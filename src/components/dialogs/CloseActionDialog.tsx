import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Loader2, Minimize2, Power } from 'lucide-react';

export type CloseAction = 'minimize' | 'quit';

interface CloseActionDialogProps {
  open: boolean;
  closeActionLoading: CloseAction | null;
  rememberCloseChoice: boolean;
  onRememberChange: (checked: boolean) => void;
  onClose: () => void;
  onExecuteAction: (action: CloseAction, remember: boolean) => Promise<void>;
}

export function CloseActionDialog({
  open,
  closeActionLoading,
  rememberCloseChoice,
  onRememberChange,
  onClose,
  onExecuteAction,
}: CloseActionDialogProps) {
  return (
    <Dialog
      open={open}
      onOpenChange={(isOpen) => {
        if (!isOpen) {
          onClose();
        }
      }}
    >
      <DialogContent
        className="sm:max-w-[420px]"
        onPointerDown={(event) => event.stopPropagation()}
      >
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Power className="h-5 w-5 text-slate-600 dark:text-slate-100" />
            关闭 DuckCoding？
          </DialogTitle>
          <DialogDescription>选择关闭窗口或最小化到系统托盘。</DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <Button
            variant="secondary"
            className="w-full justify-between"
            disabled={Boolean(closeActionLoading)}
            onClick={() => onExecuteAction('minimize', rememberCloseChoice)}
          >
            <span className="flex items-center gap-2">
              {closeActionLoading === 'minimize' ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Minimize2 className="h-4 w-4" />
              )}
              最小化到系统托盘
            </span>
            <span className="text-xs text-slate-500">推荐</span>
          </Button>

          <Button
            variant="destructive"
            className="w-full justify-between"
            disabled={Boolean(closeActionLoading)}
            onClick={() => onExecuteAction('quit', rememberCloseChoice)}
          >
            <span className="flex items-center gap-2">
              {closeActionLoading === 'quit' ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Power className="h-4 w-4" />
              )}
              直接退出程序
            </span>
            <span className="text-xs opacity-80">退出</span>
          </Button>

          <label className="flex items-center gap-2 text-sm text-slate-600 dark:text-slate-300">
            <input
              type="checkbox"
              checked={rememberCloseChoice}
              onChange={(event) => onRememberChange(event.target.checked)}
              className="h-4 w-4 rounded border-slate-300 text-blue-600 focus:ring-blue-500"
            />
            记住我的选择
          </label>

          <p className="text-xs text-slate-500 dark:text-slate-400">
            若取消勾选「记住我的选择」，下次点击关闭时会再次询问。
          </p>
        </div>
      </DialogContent>
    </Dialog>
  );
}
