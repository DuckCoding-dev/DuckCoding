import { useState, useEffect, useRef } from 'react';
import { Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
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
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  ClaudeConfigManager,
  CodexConfigManager,
  GeminiConfigManager,
} from '@/components/ToolConfigManager';
import { saveGlobalConfig } from '@/lib/tauri-commands';
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
  const [hideProxyTip, setHideProxyTip] = useState(false); // 临时关闭推荐提示
  const [neverShowProxyTip, setNeverShowProxyTip] = useState(false); // 永久隐藏推荐提示
  const [externalDialogOpen, setExternalDialogOpen] = useState(false);
  const seenExternalChangesRef = useRef<Set<string>>(new Set());

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
    loadGlobalConfig,
    loadAllProxyStatus,
    loadAllProfiles,
    handleSwitchProfile,
    handleDeleteProfile,
    externalChanges,
    notifyEnabled,
    isToolProxyEnabled,
    isToolProxyRunning,
  } = useProfileManagement(tools, applySavedOrder);

  // 同步外部 tools 数据
  useEffect(() => {
    setTools(toolsProp);
    setLoading(loadingProp);
  }, [toolsProp, loadingProp]);

  // 初始加载
  useEffect(() => {
    loadGlobalConfig();
    loadAllProxyStatus();
  }, [loadGlobalConfig, loadAllProxyStatus]);

  // 从全局配置读取永久隐藏状态
  useEffect(() => {
    if (globalConfig?.hide_transparent_proxy_tip) {
      setNeverShowProxyTip(true);
    }
  }, [globalConfig]);

  // 当工具加载完成后，加载配置
  useEffect(() => {
    if (tools.length > 0) {
      loadAllProfiles();
      // 设置默认选中的Tab（第一个工具）
      if (!selectedSwitchTab) {
        setSelectedSwitchTab(tools[0].id);
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

  // 临时关闭推荐提示
  const handleCloseProxyTip = () => {
    setHideProxyTip(true);
  };

  // 永久隐藏推荐提示
  const handleNeverShowProxyTip = async () => {
    if (!globalConfig) return;

    try {
      await saveGlobalConfig({
        ...globalConfig,
        hide_transparent_proxy_tip: true,
      });
      setNeverShowProxyTip(true);
      toast({
        title: '设置已保存',
        description: '透明代理推荐提示已永久隐藏',
      });
    } catch (error) {
      toast({
        title: '保存失败',
        description: String(error),
        variant: 'destructive',
      });
    }
  };

  // 跳转到透明代理页并选中工具
  const navigateToProxyPage = (toolId: string) => {
    window.dispatchEvent(
      new CustomEvent('navigate-to-transparent-proxy', {
        detail: { toolId },
      }),
    );
  };

  // 切换到安装页面
  const switchToInstall = () => {
    window.dispatchEvent(new CustomEvent('navigate-to-install'));
  };

  // 跳转到设置页的配置管理 tab
  const navigateToConfigManagement = () => {
    window.dispatchEvent(
      new CustomEvent('navigate-to-settings', { detail: { tab: 'config-management' } }),
    );
    setExternalDialogOpen(false);
  };

  // 获取当前选中工具的代理状态
  const currentToolProxyEnabled = isToolProxyEnabled(selectedSwitchTab);
  const currentToolProxyRunning = isToolProxyRunning(selectedSwitchTab);
  // 获取当前选中工具的名称
  const getCurrentToolName = () => {
    const tool = tools.find((t) => t.id === selectedSwitchTab);
    return tool?.name || selectedSwitchTab;
  };

  const getToolDisplayName = (toolId: string) => {
    const tool = tools.find((t) => t.id === toolId);
    return tool?.name || toolId;
  };

  // 检测到新的外部改动时弹出提醒对话框（去重）
  useEffect(() => {
    if (!notifyEnabled) {
      setExternalDialogOpen(false);
      return;
    }
    if (externalChanges.length === 0) {
      seenExternalChangesRef.current = new Set();
      setExternalDialogOpen(false);
      return;
    }
    let hasNew = false;
    const seen = seenExternalChangesRef.current;
    for (const change of externalChanges) {
      const key = `${change.tool_id}|${change.path}`;
      if (!seen.has(key)) {
        seen.add(key);
        hasNew = true;
      }
    }
    if (hasNew) {
      setExternalDialogOpen(true);
    }
  }, [externalChanges, notifyEnabled]);

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
      ) : tools.length === 0 ? (
        <EmptyToolsState onNavigateToInstall={switchToInstall} />
      ) : (
        <>
          {/* 工具切换 Tab 放在顶部（第三行） */}
          <Tabs value={selectedSwitchTab} onValueChange={setSelectedSwitchTab} className="mb-6">
            <TabsList className="grid w-full grid-cols-3">
              {tools.map((tool) => (
                <TabsTrigger key={tool.id} value={tool.id} className="gap-2">
                  <img src={logoMap[tool.id]} alt={tool.name} className="w-4 h-4" />
                  {tool.name}
                </TabsTrigger>
              ))}
            </TabsList>

            {tools.map((tool) => {
              const toolProfiles = profiles[tool.id] || [];
              const activeConfig = activeConfigs[tool.id];
              const toolProxyEnabled = isToolProxyEnabled(tool.id);
              return (
                <TabsContent key={tool.id} value={tool.id}>
                  <ToolProfileTabContent
                    tool={tool}
                    profiles={toolProfiles}
                    activeConfig={activeConfig}
                    globalConfig={globalConfig}
                    transparentProxyEnabled={toolProxyEnabled}
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

          {/* 透明代理状态显示 - 当前选中工具 */}
          {selectedSwitchTab && (
            <ProxyStatusBanner
              toolId={selectedSwitchTab}
              toolName={getCurrentToolName()}
              isEnabled={currentToolProxyEnabled}
              isRunning={currentToolProxyRunning}
              hidden={neverShowProxyTip || hideProxyTip}
              onNavigateToProxy={() => navigateToProxyPage(selectedSwitchTab)}
              onClose={handleCloseProxyTip}
              onNeverShow={handleNeverShowProxyTip}
            />
          )}

          {/* 重启提示（在未启用透明代理时显示） */}
          <RestartWarningBanner show={!currentToolProxyEnabled || !currentToolProxyRunning} />

          {selectedSwitchTab && tools.find((tool) => tool.id === selectedSwitchTab) && (
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

      <Dialog open={externalDialogOpen} onOpenChange={setExternalDialogOpen}>
        <DialogContent className="sm:max-w-[520px]">
          <DialogHeader>
            <DialogTitle>检测到外部配置改动</DialogTitle>
            <DialogDescription>
              发现 {externalChanges.length}{' '}
              项可能由外部修改的配置，请前往「配置管理」处理以避免覆盖冲突。
            </DialogDescription>
          </DialogHeader>
          <div className="max-h-60 space-y-2 overflow-auto rounded-md border bg-muted/40 p-3 text-sm">
            {externalChanges.slice(0, 4).map((change) => (
              <div key={`${change.tool_id}-${change.path}`} className="space-y-0.5">
                <div className="font-medium">
                  {getToolDisplayName(change.tool_id)} / {change.tool_id}
                </div>
                <div className="text-xs break-all text-muted-foreground">{change.path}</div>
                <div className="text-xs text-muted-foreground">
                  检测时间：{new Date(change.detected_at).toLocaleString()}
                </div>
              </div>
            ))}
            {externalChanges.length > 4 && (
              <div className="text-xs text-muted-foreground">
                还有 {externalChanges.length - 4} 项未列出，详情请在「配置管理」查看。
              </div>
            )}
          </div>
          <DialogFooter className="gap-2">
            <Button variant="outline" onClick={() => setExternalDialogOpen(false)}>
              稍后处理
            </Button>
            <Button onClick={navigateToConfigManagement}>前往配置管理</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

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
