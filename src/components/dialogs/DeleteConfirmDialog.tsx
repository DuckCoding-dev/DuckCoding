import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { AlertCircle, Trash2 } from 'lucide-react';
import { getToolDisplayName } from '@/utils/constants';

interface DeleteConfirmDialogProps {
  open: boolean;
  toolId: string;
  profile: string;
  onClose: () => void;
  onConfirm: () => void;
}

export function DeleteConfirmDialog({
  open,
  toolId,
  profile,
  onClose,
  onConfirm,
}: DeleteConfirmDialogProps) {
  return (
    <Dialog
      open={open}
      onOpenChange={(isOpen) => {
        if (!isOpen) {
          onClose();
        }
      }}
    >
      <DialogContent className="sm:max-w-[480px]" onPointerDown={(e) => e.stopPropagation()}>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <AlertCircle className="h-5 w-5 text-red-600" />
            确认删除配置
          </DialogTitle>
          <DialogDescription>
            此操作会永久删除 {getToolDisplayName(toolId)} 的配置 「{profile}」
            ，该操作不可恢复，请谨慎确认。
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-3 text-sm text-slate-700 dark:text-slate-300">
          <p>
            删除后，{getToolDisplayName(toolId)}{' '}
            将无法再使用该配置。如需要保留，请先备份或导出配置，再进行删除。
          </p>
          <div className="bg-red-50 dark:bg-red-950/40 border border-red-200 dark:border-red-800 rounded-lg p-4">
            <p className="text-xs text-red-700 dark:text-red-300">
              ⚠️ 注意：删除操作不可撤销，请确认已不再需要该配置。
            </p>
          </div>
        </div>

        <DialogFooter className="gap-2">
          <Button type="button" variant="outline" onClick={onClose}>
            取消
          </Button>

          <Button
            type="button"
            variant="destructive"
            onClick={onConfirm}
            className="shadow-sm hover:shadow-md transition-all"
          >
            <Trash2 className="mr-2 h-4 w-4" />
            确认删除
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
