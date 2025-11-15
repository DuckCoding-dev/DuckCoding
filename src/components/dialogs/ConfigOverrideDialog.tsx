import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { AlertCircle, Loader2, Save } from 'lucide-react';

interface ConfigOverrideDialogProps {
  open: boolean;
  targetProfile: string;
  configuring: boolean;
  onClose: () => void;
  onConfirmOverride: () => Promise<void>;
}

export function ConfigOverrideDialog({
  open,
  targetProfile,
  configuring,
  onClose,
  onConfirmOverride,
}: ConfigOverrideDialogProps) {
  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && onClose()}>
      <DialogContent className="sm:max-w-[500px]" onPointerDown={(e) => e.stopPropagation()}>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <AlertCircle className="h-5 w-5 text-amber-600" />
            配置已存在
          </DialogTitle>
          <DialogDescription>检测到配置 "{targetProfile}" 已存在，是否覆盖？</DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* 警告信息 */}
          <div className="bg-gradient-to-r from-amber-50 to-orange-50 dark:from-amber-950 dark:to-orange-950 rounded-lg border border-amber-200 dark:border-amber-800 p-4">
            <div className="flex items-start gap-2">
              <AlertCircle className="h-5 w-5 text-amber-600 dark:text-amber-400 flex-shrink-0 mt-0.5" />
              <div className="space-y-2">
                <h4 className="font-semibold text-amber-900 dark:text-amber-100">覆盖确认</h4>
                <p className="text-sm text-amber-800 dark:text-amber-200">
                  当前配置{' '}
                  <span className="font-mono bg-white/50 dark:bg-slate-900/50 px-2 py-0.5 rounded">
                    {targetProfile}
                  </span>{' '}
                  已存在。继续保存将覆盖原有内容。
                </p>
              </div>
            </div>
          </div>

          <p className="text-sm text-slate-700 dark:text-slate-300">
            如果需要保留旧配置，可在覆盖前手动复制信息或另存为其他名称。
          </p>
        </div>

        <DialogFooter className="gap-2">
          <Button type="button" variant="outline" onClick={onClose} disabled={configuring}>
            取消
          </Button>

          <Button
            type="button"
            variant="destructive"
            onClick={onConfirmOverride}
            disabled={configuring}
            className="shadow-sm hover:shadow-md transition-all"
          >
            {configuring ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                保存中...
              </>
            ) : (
              <>
                <Save className="mr-2 h-4 w-4" />
                确认覆盖
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
