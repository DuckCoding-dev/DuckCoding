import { Button } from '@/components/ui/button';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { ExternalLink, ArrowUpRight } from 'lucide-react';
import type { AppUpdateInfo } from '@/hooks/useAppUpdate';

interface AppUpdateBannerProps {
  info: AppUpdateInfo;
  onDismiss?: () => void;
}

export function AppUpdateBanner({ info, onDismiss }: AppUpdateBannerProps) {
  return (
    <Alert className="mb-4 border-blue-200 bg-blue-50 dark:border-blue-800 dark:bg-blue-950/30">
      <div className="flex items-start gap-3">
        <div className="flex-1 space-y-1">
          <AlertTitle className="flex items-center gap-3 text-blue-900 dark:text-blue-100">
            发现新版本 v{info.version}
            {info.required && (
              <Badge variant="destructive" className="text-xs">
                强制更新
              </Badge>
            )}
          </AlertTitle>
          <AlertDescription className="text-sm text-blue-800 dark:text-blue-200">
            当前版本 v{__APP_VERSION__}，
            {info.releaseNotes || '建议立即下载最新版本以获得最佳体验。'}
          </AlertDescription>
        </div>
        {onDismiss && (
          <Button variant="ghost" size="sm" onClick={onDismiss}>
            关闭
          </Button>
        )}
      </div>

      <div className="mt-4 flex flex-wrap gap-3">
        <Button asChild size="sm" className="gap-1">
          <a href={info.downloadUrl} target="_blank" rel="noreferrer">
            <ArrowUpRight className="h-4 w-4" />
            立即下载
          </a>
        </Button>
        <Button variant="outline" size="sm" asChild className="gap-1">
          <a href={info.releasePage} target="_blank" rel="noreferrer">
            <ExternalLink className="h-4 w-4" />
            查看更新说明
          </a>
        </Button>
      </div>
    </Alert>
  );
}
