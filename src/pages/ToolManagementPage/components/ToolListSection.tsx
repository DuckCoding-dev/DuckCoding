import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { RefreshCw, Trash2, History, Download } from 'lucide-react';
import type { ToolInstance } from '@/types/tool-management';
import { ToolType, ToolSource } from '@/types/tool-management';

// 更新状态信息
interface UpdateInfo {
  hasUpdate: boolean;
  currentVersion: string | null;
  latestVersion: string | null;
}

interface ToolListSectionProps {
  toolId: string;
  toolName: string;
  icon: string;
  instances: ToolInstance[];
  onCheckUpdate: (instanceId: string) => void;
  onUpdate: (instanceId: string) => void;
  onDelete: (instanceId: string) => void;
  onVersionManage?: (instanceId: string) => void;
  updateInfoMap: Record<string, UpdateInfo>;
  checkingUpdate: string | null;
  updating: string | null;
}

export function ToolListSection({
  instances,
  onCheckUpdate,
  onUpdate,
  onDelete,
  onVersionManage,
  updateInfoMap,
  checkingUpdate,
  updating,
}: ToolListSectionProps) {
  const getTypeLabel = (type: ToolType) => {
    switch (type) {
      case ToolType.Local:
        return '本地';
      case ToolType.WSL:
        return 'WSL';
      case ToolType.SSH:
        return 'SSH';
      default:
        return type;
    }
  };

  const getTypeBadge = (type: ToolType) => {
    let variant: 'default' | 'secondary' | 'outline' = 'outline';
    if (type === ToolType.Local) variant = 'default';
    if (type === ToolType.WSL) variant = 'secondary';

    return (
      <Badge variant={variant} className="text-xs">
        {getTypeLabel(type)}
      </Badge>
    );
  };

  const getSourceBadge = (source: ToolSource) => {
    if (source === ToolSource.DuckCodingManaged) {
      return (
        <Badge variant="default" className="text-xs">
          DuckCoding 安装
        </Badge>
      );
    }
    return (
      <Badge variant="secondary" className="text-xs">
        外部安装
      </Badge>
    );
  };

  return (
    <div className="space-y-4">
      {instances.length === 0 ? (
        <div className="text-center py-12 border rounded-lg bg-muted/30">
          <p className="text-muted-foreground text-sm mb-2">暂无实例</p>
          <p className="text-xs text-muted-foreground">点击右上角"添加实例"来添加</p>
        </div>
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-[120px]">环境类型</TableHead>
              <TableHead className="w-[150px]">安装来源</TableHead>
              <TableHead className="w-[100px]">状态</TableHead>
              <TableHead className="w-[120px]">版本</TableHead>
              <TableHead>安装路径</TableHead>
              <TableHead className="w-[280px] text-right">操作</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {instances.map((instance) => {
              const isDuckCoding = instance.tool_source === ToolSource.DuckCodingManaged;
              const isSSH = instance.tool_type === ToolType.SSH;
              const canDelete = isSSH && !instance.is_builtin;
              const updateInfo = updateInfoMap[instance.instance_id];
              const hasUpdate = updateInfo?.hasUpdate ?? false;
              const isChecking = checkingUpdate === instance.instance_id;
              const isUpdating = updating === instance.instance_id;

              return (
                <TableRow key={instance.instance_id}>
                  <TableCell className="font-medium">{getTypeBadge(instance.tool_type)}</TableCell>
                  <TableCell>{getSourceBadge(instance.tool_source)}</TableCell>
                  <TableCell>
                    {instance.installed ? (
                      <Badge
                        variant="outline"
                        className="bg-green-50 text-green-700 border-green-200"
                      >
                        已安装
                      </Badge>
                    ) : (
                      <Badge variant="outline" className="bg-gray-50 text-gray-600 border-gray-200">
                        未安装
                      </Badge>
                    )}
                  </TableCell>
                  <TableCell className="text-xs text-muted-foreground">
                    <div className="flex flex-col">
                      <span>{instance.version || '-'}</span>
                      {hasUpdate && updateInfo?.latestVersion && (
                        <span className="text-orange-600">→ {updateInfo.latestVersion}</span>
                      )}
                    </div>
                  </TableCell>
                  <TableCell className="text-xs text-muted-foreground truncate max-w-[200px]">
                    {instance.install_path || '-'}
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex justify-end gap-2">
                      {hasUpdate ? (
                        <Button
                          size="sm"
                          variant="default"
                          disabled={isUpdating}
                          onClick={() => onUpdate(instance.instance_id)}
                        >
                          <Download className="h-3 w-3 mr-1" />
                          {isUpdating ? '更新中...' : '更新'}
                        </Button>
                      ) : (
                        <Button
                          size="sm"
                          variant="outline"
                          disabled={!instance.installed || isChecking}
                          onClick={() => onCheckUpdate(instance.instance_id)}
                          title="检测是否有新版本"
                        >
                          <RefreshCw
                            className={`h-3 w-3 mr-1 ${isChecking ? 'animate-spin' : ''}`}
                          />
                          {isChecking ? '检测中...' : '检测更新'}
                        </Button>
                      )}
                      {isDuckCoding && (
                        <Button
                          size="sm"
                          variant="outline"
                          onClick={() => onVersionManage?.(instance.instance_id)}
                        >
                          <History className="h-3 w-3 mr-1" />
                          版本管理
                        </Button>
                      )}
                      {canDelete && (
                        <Button
                          size="sm"
                          variant="destructive"
                          onClick={() => onDelete(instance.instance_id)}
                        >
                          <Trash2 className="h-3 w-3 mr-1" />
                          删除
                        </Button>
                      )}
                    </div>
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
      )}
    </div>
  );
}
