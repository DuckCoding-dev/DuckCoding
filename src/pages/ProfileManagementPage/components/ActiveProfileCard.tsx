/**
 * 当前生效 Profile 卡片组件 - Clean / Professional Design
 */

import { useState, useEffect } from 'react';
import { ChevronDown, ChevronUp, Loader2, Server, Terminal, Laptop, Settings, RefreshCw, CheckCircle2, Zap, Download } from 'lucide-react';
import type { ProfileGroup } from '@/types/profile';
import type { ToolInstance, ToolType } from '@/types/tool-management';
import { getToolInstances, checkUpdate, updateToolInstance } from '@/lib/tauri-commands';
import { useToast } from '@/hooks/use-toast';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { ToolAdvancedConfigDialog } from '@/components/ToolAdvancedConfigDialog';
import { Separator } from '@/components/ui/separator';
import { cn } from '@/lib/utils';

interface ActiveProfileCardProps {
  group: ProfileGroup;
  proxyRunning: boolean;
}

const TOOL_ICONS: Record<ToolType, any> = {
  Local: Laptop,
  WSL: Terminal,
  SSH: Server,
};

export function ActiveProfileCard({ group, proxyRunning }: ActiveProfileCardProps) {
  const { toast } = useToast();
  const activeProfile = group.active_profile;
  const [toolInstances, setToolInstances] = useState<ToolInstance[]>([]);
  const [selectedInstanceId, setSelectedInstanceId] = useState<string | null>(null);
  const [detailsExpanded, setDetailsExpanded] = useState(false);
  const [loading, setLoading] = useState(true);

  const [hasUpdate, setHasUpdate] = useState(false);
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const [updating, setUpdating] = useState(false);
  const [latestVersion, setLatestVersion] = useState<string | null>(null);
  const [advancedConfigOpen, setAdvancedConfigOpen] = useState(false);

  useEffect(() => {
    const loadInstances = async () => {
      try {
        setLoading(true);
        const allInstances = await getToolInstances();
        const instances = allInstances[group.tool_id] || [];
        setToolInstances(instances);
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

  const selectedInstance = toolInstances.find((i) => i.instance_id === selectedInstanceId);

  const handleInstanceChange = (instanceId: string) => {
    setSelectedInstanceId(instanceId);
    setHasUpdate(false);
    setLatestVersion(null);
  };

  const handleCheckUpdate = async () => {
    if (!selectedInstance) return;
    try {
      setCheckingUpdate(true);
      const result = await checkUpdate(group.tool_id);
      if (result.has_update) {
        setHasUpdate(true);
        setLatestVersion(result.latest_version || null);
        toast({ title: '发现新版本', description: `v${result.latest_version}` });
      } else {
        setHasUpdate(false);
        setLatestVersion(null);
        toast({ title: '已是最新版本' });
      }
    } catch (error) {
      toast({ title: '检测失败', variant: 'destructive' });
    } finally {
      setCheckingUpdate(false);
    }
  };

  const handleUpdate = async () => {
    if (!selectedInstance) return;
    try {
      setUpdating(true);
      const result = await updateToolInstance(selectedInstance.instance_id);
      if (result.success) {
        setHasUpdate(false);
        toast({ title: '更新成功' });
        const allInstances = await getToolInstances();
        const instances = allInstances[group.tool_id] || [];
        setToolInstances(instances);
      } else {
        toast({ title: '更新失败', description: result.message, variant: 'destructive' });
      }
    } catch (error) {
      toast({ title: '更新失败', variant: 'destructive' });
    } finally {
      setUpdating(false);
    }
  };

  return (
    <Card className={cn(
      "mb-6 shadow-sm overflow-hidden border transition-colors",
      proxyRunning ? "border-indigo-200/50 dark:border-indigo-800/50 bg-indigo-50/10" : 
      activeProfile ? "border-border bg-card" : 
      "border-border bg-muted/5"
    )}>
      <CardContent className="p-0">
        <div className="flex flex-col lg:flex-row lg:items-center lg:h-20">
          
          {/* Left: Info Area */}
          <div className="flex-1 p-4 lg:pl-6 flex items-center gap-4">
            {/* Icon Box */}
            <div className={cn(
              "w-10 h-10 rounded-xl flex items-center justify-center border shadow-sm",
              proxyRunning ? "bg-indigo-50 border-indigo-100 text-indigo-600 dark:bg-indigo-950 dark:border-indigo-900 dark:text-indigo-400" :
              activeProfile ? "bg-background border-border text-foreground" :
              "bg-muted border-transparent text-muted-foreground"
            )}>
              {proxyRunning ? <Zap className="w-5 h-5 fill-current" /> : <CheckCircle2 className="w-5 h-5 text-green-500" />}
            </div>
            
            <div className="space-y-1">
              <div className="flex items-center gap-2">
                <h3 className="font-bold text-lg leading-none tracking-tight">{group.tool_name}</h3>
                {hasUpdate && (
                  <span className="relative flex h-2 w-2">
                    <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-amber-400 opacity-75"></span>
                    <span className="relative inline-flex rounded-full h-2 w-2 bg-amber-500"></span>
                  </span>
                )}
              </div>
              <div className="flex items-center gap-2 text-sm">
                {proxyRunning ? (
                  <div className="flex items-center gap-1.5 text-indigo-600 dark:text-indigo-400 font-medium">
                    <span>透明代理模式</span>
                  </div>
                ) : activeProfile ? (
                  <div className="flex items-center gap-1.5 text-muted-foreground">
                    <span>当前配置:</span>
                    <span className="font-medium text-foreground">{activeProfile.name}</span>
                  </div>
                ) : (
                  <span className="text-muted-foreground">未激活任何配置</span>
                )}
                
                {selectedInstance?.version && (
                  <div className="hidden sm:flex items-center text-muted-foreground/60 text-xs">
                    <span className="mx-2 h-3 w-px bg-border"></span>
                    <span>v{selectedInstance.version}</span>
                  </div>
                )}
                
                {hasUpdate && latestVersion && (
                  <div className="hidden sm:flex items-center text-xs font-medium text-amber-600 bg-amber-50 dark:bg-amber-950/30 px-1.5 py-0.5 rounded">
                    New v{latestVersion}
                  </div>
                )}
              </div>
            </div>
          </div>

          {/* Right: Controls Area */}
          <div className="px-4 pb-4 lg:p-0 lg:pr-6 flex items-center gap-3 justify-end lg:border-l lg:border-border/40 lg:h-12 lg:pl-6">
            
            {/* Instance Selector */}
            {!loading && toolInstances.length > 0 && (
              <Select value={selectedInstanceId || ''} onValueChange={handleInstanceChange}>
                <SelectTrigger className="w-full lg:w-[180px] h-9 bg-background border-input shadow-sm hover:bg-accent/50 focus:ring-1">
                  <SelectValue placeholder="选择实例" />
                </SelectTrigger>
                <SelectContent align="end">
                  {toolInstances.map((instance) => {
                    const Icon = TOOL_ICONS[instance.tool_type];
                    return (
                      <SelectItem key={instance.instance_id} value={instance.instance_id}>
                        <div className="flex items-center gap-2">
                          <Icon className="h-3.5 w-3.5 text-muted-foreground" />
                          <span className="truncate">
                            {instance.tool_type === 'WSL' ? instance.wsl_distro : instance.tool_type === 'SSH' ? instance.ssh_config?.display_name : 'Local'}
                          </span>
                        </div>
                      </SelectItem>
                    );
                  })}
                </SelectContent>
              </Select>
            )}

            {/* Actions */}
            <div className="flex items-center gap-1">
              <Button
                variant="ghost"
                size="icon"
                className="h-9 w-9 text-muted-foreground hover:text-foreground"
                onClick={() => setAdvancedConfigOpen(true)}
                title="高级配置"
              >
                <Settings className="w-4 h-4" />
              </Button>
              <Button
                variant="ghost"
                size="icon"
                className={cn(
                  "h-9 w-9",
                  hasUpdate ? "text-amber-600 hover:text-amber-700 hover:bg-amber-50" : "text-muted-foreground hover:text-foreground"
                )}
                onClick={hasUpdate ? handleUpdate : handleCheckUpdate}
                disabled={checkingUpdate || updating || !selectedInstance}
                title={hasUpdate ? "立即更新" : "检查更新"}
              >
                {checkingUpdate || updating ? (
                  <Loader2 className="w-4 h-4 animate-spin" />
                ) : hasUpdate ? (
                  <Download className="w-4 h-4" />
                ) : (
                  <RefreshCw className="w-4 h-4" />
                )}
              </Button>
              
              {activeProfile && (
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-9 w-9 text-muted-foreground hover:text-foreground"
                  onClick={() => setDetailsExpanded(!detailsExpanded)}
                  title={detailsExpanded ? "收起详情" : "展开详情"}
                >
                  {detailsExpanded ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
                </Button>
              )}
            </div>
          </div>
        </div>

        {/* Details Panel */}
        {detailsExpanded && activeProfile && !proxyRunning && (
          <>
            <Separator />
            <div className="bg-muted/10 px-6 py-4 grid grid-cols-1 md:grid-cols-3 gap-6 animate-in slide-in-from-top-1">
               <DetailItem label="API Key" value={activeProfile.api_key_preview} />
               <DetailItem label="Base URL" value={activeProfile.base_url} />
               <DetailItem label="来源" value={activeProfile.source.type === 'Custom' ? '自定义' : activeProfile.source.provider_name} />
            </div>
          </>
        )}
      </CardContent>

      <ToolAdvancedConfigDialog
        toolId={group.tool_id}
        open={advancedConfigOpen}
        onOpenChange={setAdvancedConfigOpen}
      />
    </Card>
  );
}

function DetailItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="space-y-1">
      <span className="text-[10px] uppercase tracking-wider font-semibold text-muted-foreground/70">{label}</span>
      <p className="text-sm font-medium text-foreground truncate" title={value}>{value}</p>
    </div>
  );
}
