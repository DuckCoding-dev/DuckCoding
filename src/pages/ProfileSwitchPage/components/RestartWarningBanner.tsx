import { AlertCircle } from 'lucide-react';

interface RestartWarningBannerProps {
  show: boolean;
}

export function RestartWarningBanner({ show }: RestartWarningBannerProps) {
  if (!show) return null;

  return (
    <div className="mb-6 p-4 bg-gradient-to-r from-amber-50 to-orange-50 dark:from-amber-950 dark:to-orange-950 rounded-lg border border-amber-200 dark:border-amber-800">
      <div className="flex items-start gap-3">
        <AlertCircle className="h-5 w-5 text-amber-600 dark:text-amber-400 flex-shrink-0 mt-0.5" />
        <div className="space-y-1">
          <h4 className="font-semibold text-amber-900 dark:text-amber-100">重要提示</h4>
          <p className="text-sm text-amber-800 dark:text-amber-200">
            切换配置后，如果工具正在运行，<strong>需要重启对应的工具</strong>
            才能使新配置生效。
          </p>
        </div>
      </div>
    </div>
  );
}
