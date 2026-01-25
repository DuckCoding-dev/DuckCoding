/**
 * Profile 卡片组件
 */

import { useState } from 'react';
import { MoreVertical, Pencil, Trash2, AlertCircle, Play, CheckCircle2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle, CardFooter } from '@/components/ui/card';
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
import { cn } from '@/lib/utils';

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

  /**
   * 获取来源显示文本和样式
   */
  const getSourceInfo = () => {
    if (profile.source.type === 'Custom') {
      return {
        text: '自定义',
        variant: 'secondary' as const,
        tooltip: '用户手动创建的 Profile',
      };
    } else {
      const importedAt = new Date(profile.source.imported_at * 1000);
      return {
        text: '从 ' + profile.source.provider_name + ' 导入',
        variant: 'outline' as const,
        tooltip:
          '从供应商「' +
          profile.source.provider_name +
          '」的令牌「' +
          profile.source.remote_token_name +
          '」导入\n分组: ' +
          profile.source.group +
          '\n导入时间: ' +
          importedAt.toLocaleString('zh-CN'),
      };
    }
  };

  const sourceInfo = getSourceInfo();
  const isActive = profile.is_active;

  return (
    <>
      <Card 
        className={cn(
          "transition-all duration-200",
          isActive 
            ? "border-primary/50 bg-primary/5 shadow-sm ring-1 ring-primary/20" 
            : "hover:border-primary/30 hover:shadow-sm"
        )}
      >
        <CardHeader className="flex flex-row items-start justify-between space-y-0 pb-2">
          <div className="space-y-1">
            <div className="flex items-center gap-2">
              <CardTitle className="text-base font-semibold flex items-center gap-2">
                {profile.name}
                {isActive && !proxyRunning && (
                  <CheckCircle2 className="h-4 w-4 text-primary" />
                )}
              </CardTitle>
            </div>
            <CardDescription className="text-xs flex items-center gap-2">
              <Badge
                variant={sourceInfo.variant}
                className="h-5 whitespace-nowrap px-1.5 font-normal"
                title={sourceInfo.tooltip}
              >
                {sourceInfo.text}
              </Badge>
            </CardDescription>
          </div>

          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon" className="h-8 w-8 -mr-2">
                <MoreVertical className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onClick={onEdit}>
                <Pencil className="mr-2 h-4 w-4" />
                编辑配置
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

        <CardContent className="space-y-3 text-sm pb-3">
          <div className="grid grid-cols-1 gap-2">
             <div className="flex flex-col space-y-1">
                <span className="text-xs text-muted-foreground">API Key</span>
                <code className="text-xs bg-muted px-1.5 py-0.5 rounded w-fit">{profile.api_key_preview}</code>
             </div>
             <div className="flex flex-col space-y-1">
                <span className="text-xs text-muted-foreground">Base URL</span>
                <span className="text-xs truncate text-foreground/80 font-medium" title={profile.base_url}>
                  {profile.base_url}
                </span>
             </div>
          </div>

          <div className="flex items-center justify-between text-xs text-muted-foreground border-t pt-2 mt-2">
            <span>{isActive ? '切换于' : '创建于'} {isActive && profile.switched_at ? formatTime(profile.switched_at) : formatTime(profile.created_at)}</span>
          </div>
        </CardContent>

        {/* Footer Actions */}
        {!isActive && (
           <CardFooter className="pt-0 pb-4">
              <Button 
                className="w-full" 
                variant={proxyRunning ? "outline" : "default"}
                size="sm"
                onClick={proxyRunning ? undefined : onActivate}
                disabled={proxyRunning}
              >
                {proxyRunning ? (
                   <>
                     <AlertCircle className="mr-2 h-3.5 w-3.5" />
                     代理运行中
                   </>
                ) : (
                   <>
                     <Play className="mr-2 h-3.5 w-3.5" />
                     激活配置
                   </>
                )}
              </Button>
           </CardFooter>
        )}
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