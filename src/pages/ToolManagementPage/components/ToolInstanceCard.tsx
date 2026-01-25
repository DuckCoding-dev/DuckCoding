import {
  Card,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { RefreshCw, Trash2, History, Download, Monitor, Terminal, Server } from 'lucide-react';
import type { ToolInstance } from '@/types/tool-management';
import { ToolType, ToolSource } from '@/types/tool-management';

interface UpdateInfo {
  hasUpdate: boolean;
  currentVersion: string | null;
  latestVersion: string | null;
}

interface ToolInstanceCardProps {
  instance: ToolInstance;
  onCheckUpdate: (instanceId: string) => void;
  onUpdate: (instanceId: string) => void;
  onDelete: (instanceId: string) => void;
  onVersionManage?: (instanceId: string) => void;
  updateInfo?: UpdateInfo;
  checkingUpdate: boolean;
  updating: boolean;
}

export function ToolInstanceCard({
  instance,
  onCheckUpdate,
  onUpdate,
  onDelete,
  onVersionManage,
  updateInfo,
  checkingUpdate,
  updating,
}: ToolInstanceCardProps) {
  const isDuckCoding = instance.tool_source === ToolSource.DuckCodingManaged;
  const isSSH = instance.tool_type === ToolType.SSH;
  const canDelete = isSSH && !instance.is_builtin;
  const hasUpdate = updateInfo?.hasUpdate ?? false;

  const getTypeIcon = (type: ToolType) => {
    switch (type) {
      case ToolType.Local:
        return <Monitor className="h-4 w-4" />;
      case ToolType.WSL:
        return <Terminal className="h-4 w-4" />;
      case ToolType.SSH:
        return <Server className="h-4 w-4" />;
      default:
        return <Monitor className="h-4 w-4" />;
    }
  };

  const getSourceBadge = (source: ToolSource) => {
    if (source === ToolSource.DuckCodingManaged) {
      return <Badge variant="secondary" className="text-[10px]">DuckCoding</Badge>;
    }
    return <Badge variant="outline" className="text-[10px]">External</Badge>;
  };

  return (
    <Card className="flex flex-col">
      <CardHeader className="pb-3">
        <div className="flex justify-between items-start">
          <div className="flex items-center gap-2">
            <div className="p-2 bg-muted rounded-md">
              {getTypeIcon(instance.tool_type)}
            </div>
            <div>
              <CardTitle className="text-base font-medium">
                {instance.tool_type}
              </CardTitle>
              <div className="flex gap-2 mt-1">
                {getSourceBadge(instance.tool_source)}
                {instance.installed ? (
                  <Badge variant="outline" className="text-[10px] text-green-600 bg-green-50 border-green-200">
                    已安装
                  </Badge>
                ) : (
                  <Badge variant="outline" className="text-[10px] text-gray-500">
                    未安装
                  </Badge>
                )}
              </div>
            </div>
          </div>
        </div>
      </CardHeader>
      
      <CardContent className="flex-1 pb-3 text-sm space-y-3">
        <div className="space-y-1">
          <span className="text-xs text-muted-foreground">版本</span>
          <div className="flex items-center gap-2">
            <span className="font-medium">{instance.version || '-'}</span>
            {hasUpdate && updateInfo?.latestVersion && (
              <Badge variant="destructive" className="text-[10px] px-1 h-5">
                New: {updateInfo.latestVersion}
              </Badge>
            )}
          </div>
        </div>
        
        <div className="space-y-1">
          <span className="text-xs text-muted-foreground">路径</span>
          <div className="bg-muted/50 p-2 rounded text-xs font-mono break-all line-clamp-2" title={instance.install_path || ''}>
            {instance.install_path || '-'}
          </div>
        </div>
      </CardContent>

      <CardFooter className="pt-0 gap-2 justify-end">
        {hasUpdate ? (
          <Button
            size="sm"
            variant="default"
            className="h-8 text-xs w-full"
            disabled={updating || checkingUpdate}
            onClick={() => onUpdate(instance.instance_id)}
          >
            <Download className="h-3 w-3 mr-1.5" />
            {updating ? '更新中' : '立即更新'}
          </Button>
        ) : (
          <Button
            size="sm"
            variant="outline"
            className="h-8 text-xs flex-1"
            disabled={!instance.installed || checkingUpdate || updating}
            onClick={() => onCheckUpdate(instance.instance_id)}
          >
            <RefreshCw className={`h-3 w-3 mr-1.5 ${checkingUpdate ? 'animate-spin' : ''}`} />
            {checkingUpdate ? '检测中' : '检查更新'}
          </Button>
        )}

        {isDuckCoding && (
          <Button
            size="icon"
            variant="outline"
            className="h-8 w-8"
            title="版本管理"
            onClick={() => onVersionManage?.(instance.instance_id)}
          >
            <History className="h-3.5 w-3.5" />
          </Button>
        )}

        {canDelete && (
          <Button
            size="icon"
            variant="ghost"
            className="h-8 w-8 text-destructive hover:text-destructive"
            title="删除实例"
            onClick={() => onDelete(instance.instance_id)}
          >
            <Trash2 className="h-3.5 w-3.5" />
          </Button>
        )}
      </CardFooter>
    </Card>
  );
}
