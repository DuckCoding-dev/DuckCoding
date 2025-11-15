import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { CheckCircle2, RefreshCw, Loader2, Key } from 'lucide-react';
import { logoMap, descriptionMap } from '@/utils/constants';
import { formatVersionLabel } from '@/utils/formatting';
import type { ToolStatus } from '@/lib/tauri-commands';

interface DashboardToolCardProps {
  tool: ToolStatus;
  updating: boolean;
  checkingUpdates: boolean;
  onUpdate: () => void;
  onCheckUpdates: () => void;
  onConfigure: () => void;
}

export function DashboardToolCard({
  tool,
  updating,
  checkingUpdates,
  onUpdate,
  onCheckUpdates,
  onConfigure,
}: DashboardToolCardProps) {
  return (
    <Card className="shadow-sm border">
      <CardContent className="p-5">
        <div className="flex items-start justify-between gap-6">
          <div className="flex items-center gap-4 flex-1">
            <div className="bg-secondary p-3 rounded-lg flex-shrink-0">
              <img src={logoMap[tool.id]} alt={tool.name} className="w-12 h-12" />
            </div>
            <div className="flex-1 space-y-1.5">
              <div className="flex items-center gap-3">
                <h4 className="font-semibold text-lg">{tool.name}</h4>
                <Badge variant="default" className="gap-1">
                  <CheckCircle2 className="h-3 w-3" />
                  已安装
                </Badge>
                {tool.hasUpdate && (
                  <Badge
                    variant="secondary"
                    className="gap-1 bg-amber-100 text-amber-800 dark:bg-amber-900 dark:text-amber-200"
                  >
                    <RefreshCw className="h-3 w-3" />
                    有更新
                  </Badge>
                )}
              </div>
              <p className="text-sm text-muted-foreground leading-relaxed">
                {descriptionMap[tool.id]}
              </p>
              <div className="flex items-center gap-3 mt-3">
                <div className="flex items-center gap-2">
                  <span className="text-xs font-semibold text-slate-600 dark:text-slate-400">
                    当前版本:
                  </span>
                  <span className="font-mono text-xs font-semibold text-blue-600 dark:text-blue-400 bg-blue-50 dark:bg-blue-950 px-2.5 py-1 rounded-lg shadow-sm">
                    {formatVersionLabel(tool.version)}
                  </span>
                </div>
                {tool.hasUpdate && tool.latestVersion && (
                  <div className="flex items-center gap-2">
                    <span className="text-xs font-semibold text-slate-600 dark:text-slate-400">
                      最新版本:
                    </span>
                    <span className="font-mono text-xs font-semibold text-green-600 dark:text-green-400 bg-green-50 dark:bg-green-950 px-2.5 py-1 rounded-lg shadow-sm">
                      {formatVersionLabel(tool.latestVersion)}
                    </span>
                  </div>
                )}
              </div>
            </div>
          </div>

          <div className="flex flex-col gap-2">
            <Button variant="outline" size="sm" onClick={onConfigure} className="w-32">
              <Key className="mr-2 h-4 w-4" />
              配置
            </Button>

            {tool.hasUpdate ? (
              <Button
                size="sm"
                onClick={onUpdate}
                disabled={updating}
                className="w-32 bg-gradient-to-r from-amber-500 to-orange-500 hover:from-amber-600 hover:to-orange-600"
              >
                {updating ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    更新中...
                  </>
                ) : (
                  <>
                    <RefreshCw className="mr-2 h-4 w-4" />
                    更新
                  </>
                )}
              </Button>
            ) : (
              <Button
                variant="outline"
                size="sm"
                onClick={onCheckUpdates}
                disabled={checkingUpdates}
                className="w-32"
              >
                <RefreshCw className="mr-2 h-4 w-4" />
                检查更新
              </Button>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
