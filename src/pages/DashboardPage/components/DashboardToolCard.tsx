import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { RefreshCw, Loader2, Key, Monitor, CheckCircle2, Package } from 'lucide-react';
import { logoMap } from '@/utils/constants';
import { formatVersionLabel } from '@/utils/formatting';
import type { ToolStatus } from '@/lib/tauri-commands';
import type { ToolInstance } from '@/types/tool-management';

interface DashboardToolCardProps {
  tool: ToolStatus;
  updating: boolean;
  checking: boolean; // 当前工具是否正在检测更新
  checkingAll: boolean; // 全局检测更新状态
  instanceSelection?: string; // 实例ID字符串
  instanceOptions: Array<{ value: string; label: string }>; // 实例选项列表
  toolInstances: ToolInstance[]; // 工具实例数据
  onUpdate: () => void;
  onCheckUpdates: () => void;
  onConfigure: () => void;
  onInstanceChange: (instanceType: string) => void;
  onInstall: () => void; // 新增:前往安装页面
  onAdd: () => void; // 新增：前往添加页面
}

export function DashboardToolCard({
  tool,
  updating,
  checking,
  checkingAll,
  instanceSelection,
  instanceOptions,
  toolInstances,
  onUpdate,
  onCheckUpdates,
  onConfigure,
  onInstanceChange,
  onInstall,
  onAdd,
}: DashboardToolCardProps) {
  // 是否正在检测更新（全局或单工具）
  const isChecking = checking || checkingAll;
  // 已检测完成且是最新版（确保只在检测更新后才显示）
  const isLatest = tool.hasUpdate === false && Boolean(tool.latestVersion);

  // 直接使用保存的实例ID，如果不存在则使用第一个选项
  const currentInstanceId = instanceSelection || instanceOptions[0]?.value || '';

  // 从 toolInstances 中找到当前选中的实例
  const currentInstance = toolInstances.find((inst) => inst.instance_id === currentInstanceId);

  // 显示版本：优先使用当前实例的版本，其次使用 tool.version（兼容旧逻辑）
  const displayVersion = currentInstance?.version || tool.version;

  return (
    <Card className="shadow-sm border">
      <CardContent className="p-5">
        <div className="space-y-3 mb-4">
          {/* 第一行：图标 + 标题 + Badges */}
          <div className="flex items-center gap-2">
            <div className="bg-secondary p-2.5 rounded-lg flex-shrink-0">
              <img src={logoMap[tool.id]} alt={tool.name} className="w-8 h-8" />
            </div>
            <h4 className="font-semibold text-lg whitespace-nowrap">{tool.name}</h4>
            {tool.hasUpdate && (
              <Badge
                variant="secondary"
                className="gap-1 bg-amber-100 text-amber-800 dark:bg-amber-900 dark:text-amber-200"
              >
                <RefreshCw className="h-3 w-3" />
                有更新
              </Badge>
            )}
            {isLatest && (
              <Badge
                variant="secondary"
                className="gap-1 bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200"
              >
                <CheckCircle2 className="h-3 w-3" />
                最新版
              </Badge>
            )}
          </div>

          {/* 第二行：实例选择器（仅已安装工具显示） */}
          {tool.installed && (
            <div className="flex items-center gap-2">
              <Monitor className="h-4 w-4 text-muted-foreground flex-shrink-0" />
              <Select
                value={currentInstanceId}
                onValueChange={onInstanceChange}
                disabled={instanceOptions.length === 0}
              >
                <SelectTrigger className="flex-1 h-7 text-xs">
                  <SelectValue
                    placeholder={instanceOptions.length === 0 ? '无可用实例' : undefined}
                  />
                </SelectTrigger>
                <SelectContent>
                  {instanceOptions.length === 0 ? (
                    <div className="px-2 py-1.5 text-xs text-muted-foreground">暂无配置的实例</div>
                  ) : (
                    instanceOptions.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {option.label}
                      </SelectItem>
                    ))
                  )}
                </SelectContent>
              </Select>
            </div>
          )}
        </div>

        <div className="flex items-center gap-3 mb-4">
          {tool.installed ? (
            <>
              <div className="flex items-center gap-2">
                <span className="text-xs font-semibold text-slate-600 dark:text-slate-400">
                  当前版本:
                </span>
                <span className="font-mono text-xs font-semibold text-blue-600 dark:text-blue-400 bg-blue-50 dark:bg-blue-950 px-2.5 py-1 rounded-lg shadow-sm">
                  {formatVersionLabel(displayVersion)}
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
              {isLatest && tool.latestVersion && (
                <div className="flex items-center gap-2">
                  <span className="text-xs font-semibold text-slate-600 dark:text-slate-400">
                    最新版本:
                  </span>
                  <span className="font-mono text-xs font-semibold text-green-600 dark:text-green-400 bg-green-50 dark:bg-green-950 px-2.5 py-1 rounded-lg shadow-sm">
                    {formatVersionLabel(tool.latestVersion)}
                  </span>
                </div>
              )}
            </>
          ) : (
            <div className="flex items-center gap-2">
              <span className="text-xs font-semibold text-slate-600 dark:text-slate-400">
                状态:
              </span>
              <span className="font-mono text-xs font-semibold text-gray-600 dark:text-gray-400 bg-gray-100 dark:bg-gray-800 px-2.5 py-1 rounded-lg shadow-sm">
                未安装或者没有被软件识别
              </span>
            </div>
          )}
        </div>

        <div className="flex gap-2 pt-2 border-t">
          {!tool.installed ? (
            <Button variant="outline" size="sm" onClick={onInstall} className="flex-1">
              <Package className="mr-2 h-4 w-4" />
              前往安装
            </Button>
          ) : (
            <Button
              variant="outline"
              size="sm"
              onClick={onConfigure}
              disabled={!tool.installed}
              className="flex-1"
            >
              <Key className="mr-2 h-4 w-4" />
              配置
            </Button>
          )}
          {!tool.installed ? (
            <Button variant="outline" size="sm" onClick={onAdd} className="flex-1">
              <Package className="mr-2 h-4 w-4" />
              手动添加
            </Button>
          ) : tool.hasUpdate ? (
            <Button
              size="sm"
              onClick={onUpdate}
              disabled={updating}
              className="flex-1 bg-gradient-to-r from-amber-500 to-orange-500 hover:from-amber-600 hover:to-orange-600"
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
              disabled={isChecking}
              className="flex-1"
            >
              {isChecking ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  检查中...
                </>
              ) : (
                <>
                  <RefreshCw className="mr-2 h-4 w-4" />
                  检查更新
                </>
              )}
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
