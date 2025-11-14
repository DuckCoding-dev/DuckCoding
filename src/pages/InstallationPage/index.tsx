import { useState, useEffect } from 'react';
import { Loader2 } from 'lucide-react';
import { PageContainer } from '@/components/layout/PageContainer';
import { MirrorStaleDialog } from '@/components/dialogs/MirrorStaleDialog';
import { ToolCard } from './components/ToolCard';
import { useInstallation } from './hooks/useInstallation';
import { useToast } from '@/hooks/use-toast';
import type { ToolStatus } from '@/lib/tauri-commands';

interface InstallationPageProps {
  tools: ToolStatus[];
  loading: boolean;
}

export function InstallationPage({ tools: toolsProp, loading: loadingProp }: InstallationPageProps) {
  const { toast } = useToast();
  const [tools, setTools] = useState<ToolStatus[]>(toolsProp);
  const [loading, setLoading] = useState(loadingProp);

  // 使用安装管理 Hook
  const {
    installing,
    nodeEnv,
    installMethods,
    setInstallMethods,
    mirrorStaleDialog,
    getAvailableInstallMethods,
    handleInstall,
    handleContinueMirror,
    handleUseNpm,
    closeMirrorDialog,
  } = useInstallation(tools);

  // 同步外部 tools 数据
  useEffect(() => {
    setTools(toolsProp);
    setLoading(loadingProp);
  }, [toolsProp, loadingProp]);

  // 通知父组件刷新工具列表
  const refreshTools = () => {
    window.dispatchEvent(new CustomEvent('refresh-tools'));
  };

  // 安装工具处理
  const onInstall = async (toolId: string) => {
    const result = await handleInstall(toolId);

    // 如果是镜像滞后错误，不显示toast（由对话框处理）
    if (result.mirrorStale) {
      return;
    }

    if (result.success) {
      refreshTools();
      toast({
        title: '安装成功',
        description: result.message,
      });
    } else {
      toast({
        title: '安装失败',
        description: result.message,
        variant: 'destructive',
      });
    }
  };

  // 继续使用镜像
  const onContinueMirror = async (
    toolId: string,
    source: 'install' | 'update',
    mirrorVersion: string,
  ) => {
    const result = await handleContinueMirror(toolId, source, mirrorVersion);
    if (result.success) {
      refreshTools();
      toast({
        title: '安装成功',
        description: result.message,
      });
    } else {
      toast({
        title: '安装失败',
        description: result.message,
        variant: 'destructive',
      });
    }
  };

  // 改用 npm
  const onUseNpm = async (toolId: string, officialVersion: string) => {
    const result = await handleUseNpm(toolId, officialVersion);
    if (result.success) {
      refreshTools();
      toast({
        title: '安装成功',
        description: result.message,
      });
    } else {
      toast({
        title: 'npm 安装失败',
        description: result.message,
        variant: 'destructive',
      });
    }
  };

  return (
    <PageContainer>
      <div className="mb-6">
        <h2 className="text-2xl font-semibold mb-1">安装工具</h2>
        <p className="text-sm text-muted-foreground">选择并安装您需要的 AI 开发工具</p>
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-20">
          <Loader2 className="h-8 w-8 animate-spin text-primary" />
          <span className="ml-3 text-muted-foreground">加载中...</span>
        </div>
      ) : (
        <div className="grid gap-4">
          {tools.map((tool) => (
            <ToolCard
              key={tool.id}
              tool={tool}
              installMethod={installMethods[tool.id]}
              installing={installing === tool.id}
              availableMethods={getAvailableInstallMethods(tool.id)}
              onInstall={() => onInstall(tool.id)}
              onMethodChange={(method) =>
                setInstallMethods({ ...installMethods, [tool.id]: method })
              }
            />
          ))}
        </div>
      )}

      {/* 镜像滞后对话框 */}
      <MirrorStaleDialog
        open={mirrorStaleDialog.open}
        toolId={mirrorStaleDialog.toolId}
        mirrorVersion={mirrorStaleDialog.mirrorVersion}
        officialVersion={mirrorStaleDialog.officialVersion}
        source={mirrorStaleDialog.source}
        nodeEnv={nodeEnv}
        onClose={closeMirrorDialog}
        onContinueMirror={onContinueMirror}
        onUseNpm={onUseNpm}
      />
    </PageContainer>
  );
}
