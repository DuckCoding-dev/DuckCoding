import { useState, useEffect } from 'react';
import { Loader2 } from 'lucide-react';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { PageContainer } from '@/components/layout/PageContainer';
import { DeleteConfirmDialog } from '@/components/dialogs/DeleteConfirmDialog';
import { logoMap } from '@/utils/constants';
import { useToast } from '@/hooks/use-toast';
import { ProxyStatusBanner } from './components/ProxyStatusBanner';
import { ToolProfileTabContent } from './components/ToolProfileTabContent';
import { RestartWarningBanner } from './components/RestartWarningBanner';
import { EmptyToolsState } from './components/EmptyToolsState';
import { useProfileSorting } from './hooks/useProfileSorting';
import { useProfileManagement } from './hooks/useProfileManagement';
import {
  ClaudeConfigManager,
  CodexConfigManager,
  GeminiConfigManager,
} from '@/components/ToolConfigManager';
import type { ToolStatus } from '@/lib/tauri-commands';

interface ProfileSwitchPageProps {
  tools: ToolStatus[];
  loading: boolean;
}

export function ProfileSwitchPage({
  tools: toolsProp,
  loading: loadingProp,
}: ProfileSwitchPageProps) {
  const { toast } = useToast();
  const [tools, setTools] = useState<ToolStatus[]>(toolsProp);
  const [loading, setLoading] = useState(loadingProp);
  const [selectedSwitchTab, setSelectedSwitchTab] = useState<string>('');
  const [configRefreshToken, setConfigRefreshToken] = useState<Record<string, number>>({});
  const [deleteConfirmDialog, setDeleteConfirmDialog] = useState<{
    open: boolean;
    toolId: string;
    profile: string;
  }>({ open: false, toolId: '', profile: '' });

  // 使用拖拽排序Hook
  const { sensors, applySavedOrder, createDragEndHandler } = useProfileSorting();

  // 使用配置管理Hook
  const {
    switching,
    deletingProfiles,
    profiles,
    setProfiles,
    activeConfigs,
    globalConfig,
    transparentProxyStatus,
    startingProxy,
    stoppingProxy,
    loadGlobalConfig,
    loadTransparentProxyStatus,
    loadAllProfiles,
    handleSwitchProfile,
    handleDeleteProfile,
    handleStartTransparentProxy,
    handleStopTransparentProxy,
  } = useProfileManagement(tools, applySavedOrder);

  // 同步外部 tools 数据
  useEffect(() => {
    setTools(toolsProp);
    setLoading(loadingProp);
  }, [toolsProp, loadingProp]);

  // 初始加载
  useEffect(() => {
    loadGlobalConfig();
    loadTransparentProxyStatus();
  }, [loadGlobalConfig, loadTransparentProxyStatus]);

  // 当工具加载完成后，加载配置
  useEffect(() => {
    const installedTools = tools.filter((t) => t.installed);
    if (installedTools.length > 0) {
      loadAllProfiles();
      // 设置默认选中的Tab（第一个已安装的工具）
      if (!selectedSwitchTab) {
        setSelectedSwitchTab(installedTools[0].id);
      }
    }
    // 移除 loadAllProfiles 和 selectedSwitchTab 依赖，避免循环依赖
    // loadAllProfiles 已经正确依赖了 tools，无需重复添加
    // selectedSwitchTab 的设置只需在初始化时执行一次
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tools]);

  // 切换配置处理
  const onSwitchProfile = async (toolId: string, profile: string) => {
    const result = await handleSwitchProfile(toolId, profile);
    toast({
      title: result.success ? '切换成功' : '切换失败',
      description: result.message,
      variant: result.success ? 'default' : 'destructive',
    });

    if (result.success) {
      setConfigRefreshToken((prev) => ({
        ...prev,
        [toolId]: (prev[toolId] ?? 0) + 1,
      }));
    }
  };

  // 显示删除确认对话框
  const onDeleteProfile = (toolId: string, profile: string) => {
    setDeleteConfirmDialog({
      open: true,
      toolId,
      profile,
    });
  };

  // 执行删除配置
  const performDeleteProfile = async (toolId: string, profile: string) => {
    const result = await handleDeleteProfile(toolId, profile);
    setDeleteConfirmDialog({ open: false, toolId: '', profile: '' });

    toast({
      title: result.success ? '删除成功' : '删除失败',
      description: result.message,
      variant: result.success ? 'default' : 'destructive',
    });
  };

  // 启动透明代理处理
  const onStartTransparentProxy = async () => {
    const result = await handleStartTransparentProxy();
    toast({
      title: result.success ? '启动成功' : '启动失败',
      description: result.message,
      variant: result.success ? 'default' : 'destructive',
    });
  };

  // 停止透明代理处理
  const onStopTransparentProxy = async () => {
    const result = await handleStopTransparentProxy();
    toast({
      title: result.success ? '停止成功' : '停止失败',
      description: result.message,
      variant: result.success ? 'default' : 'destructive',
    });
  };

  // 切换到安装页面
  const switchToInstall = () => {
    window.dispatchEvent(new CustomEvent('navigate-to-install'));
  };

  // 切换到设置页面
  const switchToSettings = () => {
    window.dispatchEvent(new CustomEvent('navigate-to-settings'));
  };

  const installedTools = tools.filter((t) => t.installed);
  const effectiveTransparentEnabled = Boolean(globalConfig?.transparent_proxy_enabled);
  const shouldShowRestartForAllTools = !effectiveTransparentEnabled;

  return (
    <PageContainer>
      <div className="mb-6">
        <h2 className="text-2xl font-semibold mb-1">切换配置</h2>
        <p className="text-sm text-muted-foreground">在不同的配置文件之间快速切换</p>
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-20">
          <Loader2 className="h-8 w-8 animate-spin text-primary" />
          <span className="ml-3 text-muted-foreground">加载中...</span>
        </div>
      ) : (
        <>
          {/* 透明代理状态显示 - 仅在ClaudeCode选项卡显示 */}
          {selectedSwitchTab === 'claude-code' && (
            <ProxyStatusBanner
              isEnabled={effectiveTransparentEnabled}
              isRunning={transparentProxyStatus?.running || false}
              startingProxy={startingProxy}
              stoppingProxy={stoppingProxy}
              onStartProxy={onStartTransparentProxy}
              onStopProxy={onStopTransparentProxy}
              onNavigateToSettings={switchToSettings}
            />
          )}

          {/* 重启提示（在所有工具显示） */}
          <RestartWarningBanner
            show={shouldShowRestartForAllTools || selectedSwitchTab != 'claude-code'}
          />

          {installedTools.length > 0 ? (
            <Tabs value={selectedSwitchTab} onValueChange={setSelectedSwitchTab}>
              <TabsList className="grid w-full grid-cols-3 mb-6">
                {installedTools.map((tool) => (
                  <TabsTrigger key={tool.id} value={tool.id} className="gap-2">
                    <img src={logoMap[tool.id]} alt={tool.name} className="w-4 h-4" />
                    {tool.name}
                  </TabsTrigger>
                ))}
              </TabsList>

              {installedTools.map((tool) => {
                const toolProfiles = profiles[tool.id] || [];
                const activeConfig = activeConfigs[tool.id];
                return (
                  <TabsContent key={tool.id} value={tool.id}>
                    <ToolProfileTabContent
                      tool={tool}
                      profiles={toolProfiles}
                      activeConfig={activeConfig}
                      globalConfig={globalConfig}
                      transparentProxyEnabled={effectiveTransparentEnabled}
                      switching={switching}
                      deletingProfiles={deletingProfiles}
                      sensors={sensors}
                      onSwitch={onSwitchProfile}
                      onDelete={onDeleteProfile}
                      onDragEnd={createDragEndHandler(tool.id, setProfiles)}
                    />
                  </TabsContent>
                );
              })}
            </Tabs>
          ) : (
            <EmptyToolsState onNavigateToInstall={switchToInstall} />
          )}

          {selectedSwitchTab && installedTools.find((tool) => tool.id === selectedSwitchTab) && (
            <div className="mt-10 space-y-4">
              <div className="flex items-center gap-3">
                <div>
                  <h3 className="text-lg font-semibold">高级配置管理</h3>
                  <p className="text-sm text-muted-foreground">
                    直接读取并编辑{' '}
                    {selectedSwitchTab === 'claude-code'
                      ? 'Claude Code'
                      : selectedSwitchTab === 'codex'
                        ? 'CodeX'
                        : 'Gemini CLI'}{' '}
                    的配置文件
                  </p>
                </div>
              </div>
              {selectedSwitchTab === 'claude-code' && (
                <ClaudeConfigManager refreshSignal={configRefreshToken['claude-code']} />
              )}
              {selectedSwitchTab === 'codex' && (
                <CodexConfigManager refreshSignal={configRefreshToken['codex']} />
              )}
              {selectedSwitchTab === 'gemini-cli' && (
                <GeminiConfigManager refreshSignal={configRefreshToken['gemini-cli']} />
              )}
            </div>
          )}
        </>
      )}

      {/* 删除确认对话框 */}
      <DeleteConfirmDialog
        open={deleteConfirmDialog.open}
        toolId={deleteConfirmDialog.toolId}
        profile={deleteConfirmDialog.profile}
        onClose={() => setDeleteConfirmDialog({ open: false, toolId: '', profile: '' })}
        onConfirm={() => {
          performDeleteProfile(deleteConfirmDialog.toolId, deleteConfirmDialog.profile);
          setDeleteConfirmDialog({ open: false, toolId: '', profile: '' });
        }}
      />
    </PageContainer>
  );
}
