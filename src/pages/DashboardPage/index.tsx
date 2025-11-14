import { useState, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { RefreshCw, Loader2, Package } from 'lucide-react';
import { PageContainer } from '@/components/layout/PageContainer';
import { DashboardToolCard } from './components/DashboardToolCard';
import { UpdateCheckBanner } from './components/UpdateCheckBanner';
import { useDashboard } from './hooks/useDashboard';
import { getToolDisplayName } from '@/utils/constants';
import { useToast } from '@/hooks/use-toast';
import type { ToolStatus } from '@/lib/tauri-commands';

interface DashboardPageProps {
  tools: ToolStatus[];
  loading: boolean;
}

export function DashboardPage({ tools: toolsProp, loading: loadingProp }: DashboardPageProps) {
  const { toast } = useToast();
  const [loading, setLoading] = useState(loadingProp);

  // 使用仪表板 Hook
  const {
    tools,
    updating,
    checkingUpdates,
    updateCheckMessage,
    checkForUpdates,
    handleUpdate,
    updateTools,
  } = useDashboard(toolsProp);

  // 同步外部 tools 数据
  useEffect(() => {
    updateTools(toolsProp);
    setLoading(loadingProp);
  }, [toolsProp, loadingProp, updateTools]);

  // 通知父组件刷新工具列表
  const refreshTools = () => {
    window.dispatchEvent(new CustomEvent('refresh-tools'));
  };

  // 更新工具处理
  const onUpdate = async (toolId: string) => {
    const result = await handleUpdate(toolId);

    if (result.isUpdating) {
      toast({
        title: '请稍候',
        description: result.message,
        variant: 'destructive',
      });
      return;
    }

    if (result.success) {
      refreshTools();
      toast({
        title: '更新成功',
        description: `${getToolDisplayName(toolId)} ${result.message}`,
      });
    } else {
      toast({
        title: '更新失败',
        description: result.message,
        variant: 'destructive',
      });
    }
  };

  // 切换到配置页面
  const switchToConfig = (toolId?: string) => {
    window.dispatchEvent(new CustomEvent('navigate-to-config', { detail: { toolId } }));
  };

  // 切换到安装页面
  const switchToInstall = () => {
    window.dispatchEvent(new CustomEvent('navigate-to-install'));
  };

  const installedTools = tools.filter((t) => t.installed);

  return (
    <PageContainer>
      <div className="mb-6">
        <h2 className="text-2xl font-semibold mb-1">仪表板</h2>
        <p className="text-sm text-muted-foreground">管理已安装的 AI 开发工具和配置</p>
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-20">
          <Loader2 className="h-8 w-8 animate-spin text-primary" />
          <span className="ml-3 text-muted-foreground">加载中...</span>
        </div>
      ) : (
        <>
          {/* 更新检查提示 */}
          {updateCheckMessage && <UpdateCheckBanner message={updateCheckMessage} />}

          {installedTools.length === 0 ? (
            <Card className="shadow-sm border">
              <CardContent className="pt-6">
                <div className="text-center py-12">
                  <Package className="h-16 w-16 mx-auto mb-4 text-muted-foreground opacity-30" />
                  <h3 className="text-lg font-semibold mb-2">暂无已安装的工具</h3>
                  <p className="text-sm text-muted-foreground mb-4">
                    请先前往安装页面安装 AI 开发工具
                  </p>
                  <Button
                    onClick={switchToInstall}
                    className="shadow-md hover:shadow-lg transition-all"
                  >
                    <Package className="mr-2 h-4 w-4" />
                    去安装工具
                  </Button>
                </div>
              </CardContent>
            </Card>
          ) : (
            <>
              {/* 检查更新按钮 */}
              <div className="flex justify-end mb-4">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={checkForUpdates}
                  disabled={checkingUpdates}
                  className="shadow-sm hover:shadow-md transition-all"
                >
                  {checkingUpdates ? (
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
              </div>

              {/* 工具卡片列表 */}
              <div className="grid gap-4">
                {installedTools.map((tool) => (
                  <DashboardToolCard
                    key={tool.id}
                    tool={tool}
                    updating={updating === tool.id}
                    checkingUpdates={checkingUpdates}
                    onUpdate={() => onUpdate(tool.id)}
                    onCheckUpdates={checkForUpdates}
                    onConfigure={() => switchToConfig(tool.id)}
                  />
                ))}
              </div>
            </>
          )}
        </>
      )}
    </PageContainer>
  );
}
