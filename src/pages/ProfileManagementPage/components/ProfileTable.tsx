import { useState } from 'react';
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
import { MoreVertical, Pencil, Trash2, Power, AlertCircle, Check } from 'lucide-react';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import type { ProfileDescriptor } from '@/types/profile';
import { formatDistanceToNow } from 'date-fns';
import { zhCN } from 'date-fns/locale';

interface ProfileTableProps {
  profiles: ProfileDescriptor[];
  onActivate: (profileName: string) => void;
  onEdit: (profile: ProfileDescriptor) => void;
  onDelete: (profileName: string) => void;
  proxyRunning: boolean;
}

export function ProfileTable({
  profiles,
  onActivate,
  onEdit,
  onDelete,
  proxyRunning,
}: ProfileTableProps) {
  const [profileToDelete, setProfileToDelete] = useState<string | null>(null);

  const formatTime = (isoString: string) => {
    try {
      return formatDistanceToNow(new Date(isoString), {
        addSuffix: true,
        locale: zhCN,
      });
    } catch {
      return '未知';
    }
  };

  const getSourceBadge = (profile: ProfileDescriptor) => {
    if (profile.source.type === 'Custom') {
      return <Badge variant="secondary" className="text-xs">自定义</Badge>;
    }
    return <Badge variant="outline" className="text-xs">导入: {profile.source.provider_name}</Badge>;
  };

  return (
    <TooltipProvider>
      <div className="rounded-md border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-[200px]">名称</TableHead>
              <TableHead className="w-[100px]">状态</TableHead>
              <TableHead className="w-[150px]">来源</TableHead>
              <TableHead className="w-[200px]">Base URL</TableHead>
              <TableHead className="w-[150px]">创建时间</TableHead>
              <TableHead className="w-[150px]">最后切换</TableHead>
              <TableHead className="text-right">操作</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {profiles.map((profile) => (
              <TableRow key={profile.name}>
                <TableCell className="font-medium">{profile.name}</TableCell>
                <TableCell>
                  {profile.is_active ? (
                    <Badge variant="default" className="text-xs">
                      <Check className="mr-1 h-3 w-3" />
                      激活中
                    </Badge>
                  ) : (
                    <Badge variant="outline" className="text-xs text-muted-foreground">
                      未激活
                    </Badge>
                  )}
                </TableCell>
                <TableCell>{getSourceBadge(profile)}</TableCell>
                <TableCell className="truncate max-w-[200px]" title={profile.base_url}>
                  {profile.base_url}
                </TableCell>
                <TableCell className="text-xs text-muted-foreground">
                  {formatTime(profile.created_at)}
                </TableCell>
                <TableCell className="text-xs text-muted-foreground">
                  {profile.switched_at ? formatTime(profile.switched_at) : '-'}
                </TableCell>
                <TableCell className="text-right">
                  <div className="flex justify-end gap-2">
                    {!profile.is_active && (
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <span tabIndex={0} className="inline-flex">
                            <Button
                              size="sm"
                              variant={proxyRunning ? "ghost" : "default"}
                              disabled={proxyRunning}
                              onClick={() => onActivate(profile.name)}
                              className={`h-8 ${proxyRunning ? 'text-muted-foreground opacity-50' : ''}`}
                            >
                              {proxyRunning ? <AlertCircle className="h-4 w-4" /> : <Power className="h-3 w-3" />}
                              <span className="sr-only">激活</span>
                            </Button>
                          </span>
                        </TooltipTrigger>
                        {proxyRunning && (
                          <TooltipContent side="top">
                            <p>透明代理模式运行中<br/>无法切换本地配置</p>
                          </TooltipContent>
                        )}
                      </Tooltip>
                    )}
                    
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button variant="ghost" size="icon" className="h-8 w-8">
                          <MoreVertical className="h-4 w-4" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem onClick={() => onEdit(profile)}>
                          <Pencil className="mr-2 h-4 w-4" />
                          编辑
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem
                          onClick={() => setProfileToDelete(profile.name)}
                          className="text-destructive focus:text-destructive"
                        >
                          <Trash2 className="mr-2 h-4 w-4" />
                          删除
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </div>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      <AlertDialog open={!!profileToDelete} onOpenChange={(open) => !open && setProfileToDelete(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>确认删除</AlertDialogTitle>
            <AlertDialogDescription>
              确定要删除 Profile "{profileToDelete}" 吗？此操作无法撤销。
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>取消</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                if (profileToDelete) {
                  onDelete(profileToDelete);
                  setProfileToDelete(null);
                }
              }}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              删除
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </TooltipProvider>
  );
}