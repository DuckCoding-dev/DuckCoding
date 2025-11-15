import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Loader2, Power, AlertCircle, Sparkles, Settings as SettingsIcon } from 'lucide-react';

interface ProxyStatusBannerProps {
  isEnabled: boolean;
  isRunning: boolean;
  startingProxy: boolean;
  stoppingProxy: boolean;
  onStartProxy: () => void;
  onStopProxy: () => void;
  onNavigateToSettings: () => void;
}

export function ProxyStatusBanner({
  isEnabled,
  isRunning,
  startingProxy,
  stoppingProxy,
  onStartProxy,
  onStopProxy,
  onNavigateToSettings,
}: ProxyStatusBannerProps) {
  // 已启用透明代理
  if (isEnabled) {
    // 正在运行 - 显示蓝色成功状态
    if (isRunning) {
      return (
        <div className="mb-6 p-4 bg-gradient-to-r from-blue-50 to-indigo-50 dark:from-blue-950 dark:to-indigo-950 rounded-lg border border-blue-200 dark:border-blue-800">
          <div className="flex items-center justify-between">
            <div className="flex items-start gap-3 flex-1">
              <Power className="h-5 w-5 text-blue-600 dark:text-blue-400 flex-shrink-0 mt-0.5" />
              <div className="space-y-1 flex-1">
                <h4 className="font-semibold text-blue-900 dark:text-blue-100 flex items-center gap-2">
                  ClaudeCode 透明代理
                  <Badge variant="default" className="text-xs">
                    运行中
                  </Badge>
                </h4>
                <p className="text-sm text-blue-800 dark:text-blue-200">
                  透明代理正在运行，切换配置无需重启终端，配置将实时生效。
                </p>
              </div>
            </div>
            <div className="flex gap-2">
              <Button
                type="button"
                variant="destructive"
                size="sm"
                onClick={onStopProxy}
                disabled={stoppingProxy}
                className="shadow-sm"
              >
                {stoppingProxy ? (
                  <>
                    <Loader2 className="h-4 w-4 mr-1 animate-spin" />
                    停止中...
                  </>
                ) : (
                  <>
                    <Power className="h-4 w-4 mr-1" />
                    停止代理
                  </>
                )}
              </Button>
            </div>
          </div>
        </div>
      );
    }

    // 已启用但未运行 - 显示红色警告
    return (
      <div className="mb-6 p-4 bg-gradient-to-r from-red-50 to-orange-50 dark:from-red-950 dark:to-orange-950 rounded-lg border-2 border-red-300 dark:border-red-700 animate-pulse">
        <div className="flex items-center justify-between">
          <div className="flex items-start gap-3 flex-1">
            <AlertCircle className="h-6 w-6 text-red-600 dark:text-red-400 flex-shrink-0 mt-0.5" />
            <div className="space-y-1 flex-1">
              <h4 className="font-semibold text-red-900 dark:text-red-100 flex items-center gap-2">
                ⚠️ 透明代理未启动
                <Badge variant="destructive" className="text-xs">
                  无法使用
                </Badge>
              </h4>
              <p className="text-sm text-red-800 dark:text-red-200 font-medium">
                您已启用透明代理但尚未启动服务！ClaudeCode 当前
                <strong className="underline">无法正常使用</strong>
                ，请立即启动代理或在设置中禁用透明代理功能。
              </p>
            </div>
          </div>
          <div className="flex gap-2">
            <Button
              type="button"
              variant="default"
              size="sm"
              onClick={onStartProxy}
              disabled={startingProxy}
              className="shadow-sm bg-red-600 hover:bg-red-700"
            >
              {startingProxy ? (
                <>
                  <Loader2 className="h-4 w-4 mr-1 animate-spin" />
                  启动中...
                </>
              ) : (
                <>
                  <Power className="h-4 w-4 mr-1" />
                  立即启动
                </>
              )}
            </Button>
          </div>
        </div>
      </div>
    );
  }

  // 未启用透明代理 - 显示绿色推荐Banner
  return (
    <div className="mb-6 p-4 bg-gradient-to-r from-green-50 to-emerald-50 dark:from-green-950 dark:to-emerald-950 rounded-lg border border-green-200 dark:border-green-800">
      <div className="flex items-start gap-3">
        <Sparkles className="h-5 w-5 text-green-600 dark:text-green-400 flex-shrink-0 mt-0.5" />
        <div className="space-y-2 flex-1">
          <h4 className="font-semibold text-green-900 dark:text-green-100 flex items-center gap-2">
            💡 推荐体验：ClaudeCode 透明代理
            <Badge
              variant="outline"
              className="text-xs border-green-600 text-green-700 dark:text-green-300"
            >
              实验性
            </Badge>
          </h4>
          <p className="text-sm text-green-800 dark:text-green-200">
            启用透明代理后，切换 ClaudeCode 配置<strong>无需重启终端</strong>
            ，配置实时生效！大幅提升工作效率。
          </p>
          <div className="flex gap-2 mt-3">
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={onNavigateToSettings}
              className="shadow-sm border-green-600 text-green-700 hover:bg-green-100 dark:text-green-300 dark:hover:bg-green-950"
            >
              <SettingsIcon className="h-3 w-3 mr-1" />
              前往设置启用
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
