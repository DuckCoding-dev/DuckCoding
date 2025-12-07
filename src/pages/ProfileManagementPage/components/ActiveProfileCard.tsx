/**
 * 当前生效 Profile 卡片组件
 */

import { useState, useEffect } from 'react';
import { ChevronDown, ChevronUp, Loader2 } from 'lucide-react';
import type { ProfileGroup } from '@/types/profile';
import type { ToolInstance, ToolType } from '@/types/tool-management';
import { getToolInstances, checkUpdate, updateTool } from '@/lib/tauri-commands';
import { useToast } from '@/hooks/use-toast';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ToolAdvancedConfigDialog } from '@/components/ToolAdvancedConfigDialog';

interface ActiveProfileCardProps {
  group: ProfileGroup;
  proxyRunning: boolean;
}

// 工具类型显示名称映射
const TOOL_TYPE_LABELS: Record<ToolType, string> = {
  Local: '本地',
  WSL: 'WSL',
  SSH: 'SSH',
};

// 工具类型 Badge 颜色
const TOOL_TYPE_VARIANTS: Record<ToolType, 'default' | 'secondary' | 'outline'> = {
  Local: 'default',
  WSL: 'secondary',
  SSH: 'outline',
};

export function ActiveProfileCard({ group, proxyRunning }: ActiveProfileCardProps) {
  const { toast } = useToast();
  const activeProfile = group.active_profile;
  const [toolInstances, setToolInstances] = useState<ToolInstance[]>([]);
  const [selectedInstanceId, setSelectedInstanceId] = useState<string | null>(null);
  const [detailsExpanded, setDetailsExpanded] = useState(false);
  const [loading, setLoading] = useState(true);

  // 更新相关状态
  const [hasUpdate, setHasUpdate] = useState(false);
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const [updating, setUpdating] = useState(false);
  const [latestVersion, setLatestVersion] = useState<string | null>(null);

  // 高级配置 Dialog 状态
  const [advancedConfigOpen, setAdvancedConfigOpen] = useState(false);

  // 加载工具实例
  useEffect(() => {
    const loadInstances = async () => {
      try {
        setLoading(true);
        const allInstances = await getToolInstances();
        const instances = allInstances[group.tool_id] || [];
        setToolInstances(instances);

        // 默认选中 Local 实例（如果存在）
        const localInstance = instances.find((i) => i.tool_type === 'Local');
        if (localInstance) {
          setSelectedInstanceId(localInstance.instance_id);
        } else if (instances.length > 0) {
          setSelectedInstanceId(instances[0].instance_id);
        }
      } catch (error) {
        console.error('加载工具实例失败:', error);
      } finally {
        setLoading(false);
      }
    };

    loadInstances();
  }, [group.tool_id]);

  // 获取当前选中的实例
  const selectedInstance = toolInstances.find((i) => i.instance_id === selectedInstanceId);

  // 处理实例切换
  const handleInstanceChange = (instanceId: string) => {
    setSelectedInstanceId(instanceId);
    setHasUpdate(false); // 切换实例后重置更新状态
    setLatestVersion(null); // 清除最新版本信息
  };

  // 检测更新
  const handleCheckUpdate = async () => {
    if (!selectedInstance) return;

    try {
      setCheckingUpdate(true);
      const result = await checkUpdate(group.tool_id);

      if (result.has_update) {
        setHasUpdate(true);
        setLatestVersion(result.latest_version || null);
        toast({
          title: '发现新版本',
          description: `${group.tool_name}: ${result.current_version || '未知'} → ${result.latest_version || '未知'}`,
        });
      } else {
        setHasUpdate(false);
        setLatestVersion(result.latest_version || null);
        toast({
          title: '已是最新版本',
          description: `${group.tool_name} 当前版本: ${result.current_version || '未知'}`,
        });
      }
    } catch (error) {
      toast({
        title: '检测失败',
        description: error instanceof Error ? error.message : '检测更新失败',
        variant: 'destructive',
      });
    } finally {
      setCheckingUpdate(false);
    }
  };

  // 执行更新
  const handleUpdate = async () => {
    if (!selectedInstance) return;

    try {
      setUpdating(true);
      toast({
        title: '正在更新',
        description: `正在更新 ${group.tool_name}...`,
      });

      const result = await updateTool(group.tool_id);

      if (result.success) {
        setHasUpdate(false);
        toast({
          title: '更新成功',
          description: `${group.tool_name} 已更新到 ${result.latest_version || '最新版本'}`,
        });

        // 重新加载工具实例以获取新版本号
        const allInstances = await getToolInstances();
        const instances = allInstances[group.tool_id] || [];
        setToolInstances(instances);
      } else {
        toast({
          title: '更新失败',
          description: result.message || '未知错误',
          variant: 'destructive',
        });
      }
    } catch (error) {
      toast({
        title: '更新失败',
        description: error instanceof Error ? error.message : '更新失败',
        variant: 'destructive',
      });
    } finally {
      setUpdating(false);
    }
  };

  return (
    <div
      className={`p-4 rounded-lg border-2 mb-6 transition-all ${
        activeProfile || proxyRunning
          ? 'bg-gradient-to-r from-blue-50 to-indigo-50 dark:from-blue-950 dark:to-indigo-950 border-blue-300 dark:border-blue-700'
          : 'bg-muted/30 border-border'
      }`}
    >
      <div className="flex items-center justify-between">
        {/* 左侧：状态信息 */}
        <div className="flex items-center gap-3">
          <div>
            <div className="flex items-center gap-2 mb-1">
              <h4 className="font-semibold">{group.tool_name}</h4>
              <Badge
                variant={activeProfile || proxyRunning ? 'default' : 'destructive'}
                className="text-xs"
              >
                {activeProfile
                  ? proxyRunning
                    ? '透明代理模式'
                    : '激活中'
                  : proxyRunning
                    ? '透明代理模式'
                    : '未激活'}
              </Badge>
              {(activeProfile || proxyRunning) && (
                <>
                  <Badge variant="outline" className="text-xs font-normal">
                    {!proxyRunning ? `配置:${activeProfile?.name}` : '配置:透明代理'}
                  </Badge>
                  {hasUpdate && (
                    <Badge variant="destructive" className="text-xs">
                      有更新
                    </Badge>
                  )}
                </>
              )}
            </div>
            <div className="flex items-center gap-2">
              {selectedInstance?.version ? (
                <>
                  <Badge variant="outline" className="text-xs">
                    当前版本：{selectedInstance.version}
                  </Badge>
                  {latestVersion && (
                    <Badge variant="secondary" className="text-xs">
                      最新版本：{latestVersion}
                    </Badge>
                  )}
                </>
              ) : (
                <span className="text-xs text-muted-foreground">未检测到版本信息</span>
              )}
            </div>
          </div>
        </div>

        {/* 右侧控制区域 */}
        <div className="flex flex-col items-end gap-2">
          {/* 第一行：工具实例选择器 + 详情按钮 */}
          <div className="flex items-center gap-2">
            {/* 工具实例选择器 */}
            {!loading && toolInstances.length > 0 && (
              <Select
                value={selectedInstanceId || ''}
                onValueChange={handleInstanceChange}
                disabled={toolInstances.length === 0}
              >
                <SelectTrigger className="w-56 h-8 bg-white/80 dark:bg-slate-900/80">
                  <SelectValue placeholder="选择工具实例" />
                </SelectTrigger>
                <SelectContent>
                  {toolInstances.map((instance) => (
                    <SelectItem key={instance.instance_id} value={instance.instance_id}>
                      <div className="flex items-center gap-2">
                        <Badge variant={TOOL_TYPE_VARIANTS[instance.tool_type]} className="text-xs">
                          {TOOL_TYPE_LABELS[instance.tool_type]}
                        </Badge>
                        <span>
                          {instance.tool_type === 'WSL' && instance.wsl_distro
                            ? instance.wsl_distro
                            : instance.tool_type === 'SSH' && instance.ssh_config?.display_name
                              ? instance.ssh_config.display_name
                              : instance.tool_name}
                        </span>
                        {instance.version && (
                          <span className="text-xs text-muted-foreground">v{instance.version}</span>
                        )}
                      </div>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}

            {/* 详情展开/折叠按钮 */}
            {activeProfile && (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => setDetailsExpanded(!detailsExpanded)}
                className="px-2"
                title={detailsExpanded ? '收起详情' : '展开详情'}
              >
                {detailsExpanded ? (
                  <ChevronUp className="h-4 w-4" />
                ) : (
                  <ChevronDown className="h-4 w-4" />
                )}
              </Button>
            )}
          </div>

          {/* 第二行：小按钮组 */}
          <div className="flex items-center gap-2">
            {/* 高级配置按钮 */}
            <Button
              variant="outline"
              size="sm"
              className="h-7 text-xs"
              title="高级配置"
              onClick={() => setAdvancedConfigOpen(true)}
            >
              高级配置
            </Button>

            {/* 检测更新/立即更新按钮 */}
            <Button
              variant="outline"
              size="sm"
              className="h-7 text-xs"
              onClick={hasUpdate ? handleUpdate : handleCheckUpdate}
              disabled={checkingUpdate || updating || !selectedInstance}
            >
              {checkingUpdate ? (
                <>
                  <Loader2 className="h-3 w-3 mr-1 animate-spin" />
                  检测中...
                </>
              ) : updating ? (
                <>
                  <Loader2 className="h-3 w-3 mr-1 animate-spin" />
                  更新中...
                </>
              ) : hasUpdate ? (
                '立即更新'
              ) : (
                '检测更新'
              )}
            </Button>
          </div>
        </div>
      </div>

      {/* 配置详情 */}
      {activeProfile ? (
        <>
          {/* 详细信息（可折叠） */}
          {detailsExpanded && (
            <div className="mt-4 pt-4 border-t border-border/50">
              {proxyRunning ? (
                <div className="text-center py-6 text-muted-foreground">
                  <p className="text-sm">透明代理运行中</p>
                  <p className="text-xs mt-1">配置详情已由透明代理接管</p>
                </div>
              ) : (
                <div className="grid grid-cols-1 md:grid-cols-2 gap-3 text-sm">
                  <ConfigField label="API Key" value={activeProfile.api_key_preview} />
                  <ConfigField label="Base URL" value={activeProfile.base_url} />
                  {selectedInstance && (
                    <>
                      <ConfigField
                        label="实例类型"
                        value={TOOL_TYPE_LABELS[selectedInstance.tool_type]}
                      />
                      {selectedInstance.version && (
                        <ConfigField label="版本" value={selectedInstance.version} />
                      )}
                      {selectedInstance.tool_type === 'WSL' && selectedInstance.wsl_distro && (
                        <ConfigField label="WSL 发行版" value={selectedInstance.wsl_distro} />
                      )}
                      {selectedInstance.tool_type === 'SSH' && selectedInstance.ssh_config && (
                        <>
                          <ConfigField
                            label="SSH 主机"
                            value={`${selectedInstance.ssh_config.user}@${selectedInstance.ssh_config.host}:${selectedInstance.ssh_config.port}`}
                          />
                          <ConfigField
                            label="显示名称"
                            value={selectedInstance.ssh_config.display_name}
                          />
                        </>
                      )}
                    </>
                  )}
                  {activeProfile.switched_at && (
                    <div className="col-span-full pt-2 text-xs text-muted-foreground">
                      最后切换: {new Date(activeProfile.switched_at).toLocaleString('zh-CN')}
                    </div>
                  )}
                </div>
              )}
            </div>
          )}
        </>
      ) : null}

      {/* 高级配置 Dialog */}
      <ToolAdvancedConfigDialog
        toolId={group.tool_id}
        open={advancedConfigOpen}
        onOpenChange={setAdvancedConfigOpen}
      />
    </div>
  );
}

// 配置字段显示组件（参考 ProxyControlBar 的 ProxyDetails）
interface ConfigFieldProps {
  label: string;
  value: string;
}

function ConfigField({ label, value }: ConfigFieldProps) {
  return (
    <div className="space-y-1">
      <span className="text-xs text-muted-foreground">{label}</span>
      <code className="block px-2 py-1 bg-muted rounded text-xs font-mono truncate">{value}</code>
    </div>
  );
}
