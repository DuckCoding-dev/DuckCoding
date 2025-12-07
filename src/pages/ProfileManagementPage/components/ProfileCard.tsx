/**
 * Profile 卡片组件
 */

import { useState } from 'react';
import { Check, MoreVertical, Pencil, Power, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
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
import { Badge } from '@/components/ui/badge';
import type { ProfileDescriptor } from '@/types/profile';
import { formatDistanceToNow } from 'date-fns';
import { zhCN } from 'date-fns/locale';

interface ProfileCardProps {
  profile: ProfileDescriptor;
  onActivate: () => void;
  onEdit: () => void;
  onDelete: () => void;
  proxyRunning: boolean;
}

export function ProfileCard({
  profile,
  onActivate,
  onEdit,
  onDelete,
  proxyRunning,
}: ProfileCardProps) {
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);

  const handleDelete = () => {
    onDelete();
    setShowDeleteDialog(false);
  };

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

  return (
    <>
      <Card className={profile.is_active && !proxyRunning ? 'border-primary' : ''}>
        <CardHeader className="flex flex-row items-start justify-between space-y-0 pb-2">
          <div className="space-y-1">
            <div className="flex items-center gap-2">
              <CardTitle className="text-base font-semibold">{profile.name}</CardTitle>
              {profile.is_active && !proxyRunning && (
                <Badge variant="default" className="h-5">
                  <Check className="mr-1 h-3 w-3" />
                  激活中
                </Badge>
              )}
            </div>
            <CardDescription className="text-xs">
              API Key: {profile.api_key_preview}
            </CardDescription>
          </div>

          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon" className="h-8 w-8">
                <MoreVertical className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              {(!profile.is_active || proxyRunning) && (
                <>
                  <DropdownMenuItem
                    onClick={onActivate}
                    disabled={proxyRunning}
                    title={proxyRunning ? '透明代理运行中，无法激活配置' : ''}
                  >
                    <Power className="mr-2 h-4 w-4" />
                    激活
                  </DropdownMenuItem>
                  <DropdownMenuSeparator />
                </>
              )}
              <DropdownMenuItem onClick={onEdit}>
                <Pencil className="mr-2 h-4 w-4" />
                编辑
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                onClick={() => setShowDeleteDialog(true)}
                className="text-destructive focus:text-destructive"
              >
                <Trash2 className="mr-2 h-4 w-4" />
                删除
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </CardHeader>

        <CardContent className="space-y-2 text-sm">
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground">Base URL:</span>
            <span className="truncate max-w-[200px]" title={profile.base_url}>
              {profile.base_url}
            </span>
          </div>

          <div className="flex items-center justify-between text-xs text-muted-foreground">
            <span>创建于 {formatTime(profile.created_at)}</span>
          </div>

          {profile.is_active && profile.switched_at && (
            <div className="flex items-center justify-between text-xs text-muted-foreground">
              <span>切换于 {formatTime(profile.switched_at)}</span>
            </div>
          )}
        </CardContent>
      </Card>

      {/* 删除确认对话框 */}
      <AlertDialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>确认删除</AlertDialogTitle>
            <AlertDialogDescription>
              确定要删除 Profile "{profile.name}" 吗？此操作无法撤销。
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>取消</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDelete}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              删除
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
