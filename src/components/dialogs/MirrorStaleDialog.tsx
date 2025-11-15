import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { AlertCircle, Info } from 'lucide-react';

export interface NodeEnvironment {
  npm_available: boolean;
  node_version: string | null;
  npm_version: string | null;
}

interface MirrorStaleDialogProps {
  open: boolean;
  toolId: string;
  mirrorVersion: string;
  officialVersion: string;
  source: 'install' | 'update';
  nodeEnv: NodeEnvironment | null;
  onClose: () => void;
  onContinueMirror: (
    toolId: string,
    source: 'install' | 'update',
    mirrorVersion: string,
  ) => Promise<void>;
  onUseNpm: (toolId: string, officialVersion: string) => Promise<void>;
}

export function MirrorStaleDialog({
  open,
  toolId,
  mirrorVersion,
  officialVersion,
  source,
  nodeEnv,
  onClose,
  onContinueMirror,
  onUseNpm,
}: MirrorStaleDialogProps) {
  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && onClose()}>
      <DialogContent className="sm:max-w-[550px]" onPointerDown={(e) => e.stopPropagation()}>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <AlertCircle className="h-5 w-5 text-amber-600" />
            镜像版本滞后
          </DialogTitle>
          <DialogDescription>检测到镜像站的版本落后于官方最新版本</DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* 版本对比 */}
          <div className="bg-gradient-to-r from-amber-50 to-orange-50 dark:from-amber-950 dark:to-orange-950 rounded-lg border border-amber-200 dark:border-amber-800 p-4">
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-sm font-semibold text-amber-900 dark:text-amber-100">
                  镜像版本
                </span>
                <span className="font-mono text-sm font-semibold text-amber-700 dark:text-amber-300 bg-white/50 dark:bg-slate-900/50 px-3 py-1.5 rounded-lg">
                  {mirrorVersion}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm font-semibold text-amber-900 dark:text-amber-100">
                  官方最新
                </span>
                <span className="font-mono text-sm font-semibold text-green-700 dark:text-green-300 bg-white/50 dark:bg-slate-900/50 px-3 py-1.5 rounded-lg">
                  {officialVersion}
                </span>
              </div>
            </div>
          </div>

          {/* 说明文字 */}
          <div className="space-y-3 text-sm text-slate-700 dark:text-slate-300">
            <p>
              DuckCoding 镜像站的脚本版本（{mirrorVersion}）尚未同步到最新的官方版本（
              {officialVersion}）。
            </p>
            <p className="font-semibold">建议：</p>
            <ul className="list-disc list-inside space-y-1 ml-2">
              <li>
                改用 <strong>npm 安装</strong>可获取最新版本（{officialVersion}）
              </li>
              <li>或继续使用镜像安装较旧版本（{mirrorVersion}），功能基本可用</li>
            </ul>
          </div>

          {/* npm 安装提示 */}
          {nodeEnv?.npm_available && (
            <div className="bg-green-50 dark:bg-green-950/50 border border-green-200 dark:border-green-800 rounded-lg p-4">
              <div className="flex items-start gap-2">
                <Info className="h-4 w-4 text-green-600 dark:text-green-400 flex-shrink-0 mt-0.5" />
                <div className="space-y-2 flex-1">
                  <p className="text-xs font-semibold text-green-800 dark:text-green-200">
                    点击"改用 npm 安装"将自动切换为 npm 方式并重新安装
                  </p>
                  <p className="text-xs text-green-700 dark:text-green-300">
                    npm 安装会直接从 npm 仓库获取最新版本
                  </p>
                </div>
              </div>
            </div>
          )}

          {!nodeEnv?.npm_available && (
            <div className="bg-red-50 dark:bg-red-950/50 border border-red-200 dark:border-red-800 rounded-lg p-4">
              <div className="flex items-start gap-2">
                <AlertCircle className="h-4 w-4 text-red-600 dark:text-red-400 flex-shrink-0 mt-0.5" />
                <div className="text-xs text-red-700 dark:text-red-300">
                  <p className="font-semibold mb-1">npm 不可用</p>
                  <p>您的系统未安装 npm 或无法检测到，只能继续使用镜像安装</p>
                </div>
              </div>
            </div>
          )}
        </div>

        <DialogFooter className="gap-2">
          <Button type="button" variant="outline" onClick={onClose}>
            取消
          </Button>

          <Button
            type="button"
            variant="secondary"
            onClick={() => onContinueMirror(toolId, source, mirrorVersion)}
          >
            继续使用镜像 ({mirrorVersion})
          </Button>

          {nodeEnv?.npm_available && (
            <Button
              type="button"
              onClick={() => onUseNpm(toolId, officialVersion)}
              className="shadow-sm hover:shadow-md transition-all bg-gradient-to-r from-green-600 to-emerald-600 hover:from-green-700 hover:to-emerald-700"
            >
              改用 npm 安装 ({officialVersion})
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
