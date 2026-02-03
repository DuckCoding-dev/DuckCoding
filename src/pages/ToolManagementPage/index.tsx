import { useState, useEffect } from 'react';
import { Loader2, InfoIcon } from 'lucide-react';
import { listen } from '@tauri-apps/api/event';
import { PageContainer } from '@/components/layout/PageContainer';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Button } from '@/components/ui/button';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { ToolListSection } from './components/ToolListSection';
import { AddInstanceDialog } from './components/AddInstanceDialog/AddInstanceDialog';
import { VersionManagementDialog } from './components/VersionManagementDialog';
import { useToolManagement } from './hooks/useToolManagement';
import { useAppContext } from '@/hooks/useAppContext';
import { ViewToggle, ViewMode } from '@/components/common/ViewToggle';

export function ToolManagementPage() {
  const {
    tools: _toolsProp,
    toolsLoading: loadingProp,
    restrictedPage,
    setActiveTab,
    refreshTools: globalRefreshTools,
  } = useAppContext();

  const restrictNavigation = restrictedPage === 'tool-management';
  const [viewMode, setViewMode] = useState<ViewMode>('list');

  const {
    groupedByTool,
    loading: dataLoading,
    error,
    refreshTools,
    handleAddInstance,
    handleDeleteInstance,
    handleCheckUpdate,
    handleUpdate,
    updateInfoMap,
    checkingUpdate,
    updating,
  } = useToolManagement();

  // 通知父组件刷新工具列表
  const onRefreshTools = () => {
    // Also trigger global refresh
    globalRefreshTools();
    refreshTools();
  };

  const [showAddDialog, setShowAddDialog] = useState(false);
  const [versionManageDialog, setVersionManageDialog] = useState<{
    open: boolean;
    instanceId: string;
    toolName: string;
  }>({
    open: false,
    instanceId: '',
    toolName: '',
  });

  const handleVersionManage = (instanceId: string) => {
    const toolNameMap: Record<string, string> = {
      'claude-code': 'Claude Code',
      codex: 'CodeX',
      'gemini-cli': 'Gemini CLI',
    };

    const baseId = instanceId.split('-').slice(0, -1).join('-');
    const toolName = toolNameMap[baseId] || '未知工具';

    setVersionManageDialog({
      open: true,
      instanceId,
      toolName,
    });
  };

  // 监听来自引导页面的打开添加实例对话框事件
  useEffect(() => {
    console.log('[ToolManagement] 注册 open-add-instance-dialog 事件监听');
    const unlisten = listen('open-add-instance-dialog', () => {
      console.log('[ToolManagement] 接收到 open-add-instance-dialog 事件，打开对话框');
      setShowAddDialog(true);
    });

    return () => {
      console.log('[ToolManagement] 清理 open-add-instance-dialog 事件监听');
      unlisten.then((fn) => fn());
    };
  }, []);

  const pageActions = (
    <div className="flex gap-2 items-center">
      <ViewToggle mode={viewMode} onChange={setViewMode} />
      <div className="h-6 w-px bg-border mx-1" />
      <Button variant="default" onClick={() => setActiveTab('install')}>
        安装工具
      </Button>
      <Button variant="outline" onClick={() => setShowAddDialog(true)}>
        添加实例
      </Button>
      <Button variant="outline" onClick={onRefreshTools}>
        刷新状态
      </Button>
    </div>
  );

  return (
    <PageContainer
      title="工具管理"
      description="管理所有 AI 开发工具的安装和配置"
      actions={pageActions}
    >
      {/* 引导模式提示 */}
      {restrictNavigation && (
        <Alert className="mb-4">
          <InfoIcon className="h-4 w-4" />
          <AlertDescription>
            当前处于引导模式，请点击「添加实例」按钮配置工具。完成后点击右下角悬浮按钮继续引导。
          </AlertDescription>
        </Alert>
      )}

      {/* 加载状态 */}
      {(loadingProp || dataLoading) && (
        <div className="flex items-center justify-center py-20">
          <Loader2 className="h-8 w-8 animate-spin text-primary" />
          <span className="ml-3 text-muted-foreground">加载中...</span>
        </div>
      )}

      {/* 错误状态 */}
      {error && (
        <div className="rounded-lg border border-red-200 bg-red-50 p-4 text-sm text-red-800">
          <p className="font-medium mb-1">加载失败</p>
          <p className="text-xs mb-3">{error}</p>
          <Button size="sm" variant="outline" onClick={onRefreshTools}>
            重试
          </Button>
        </div>
      )}

      {/* Tab 按工具切换 */}
      {!loadingProp && !dataLoading && !error && (
        <Tabs defaultValue="claude-code" className="w-full">
          <TabsList className="grid w-full grid-cols-3 mb-4 h-9 p-1 bg-muted/50 rounded-lg">
            <TabsTrigger
              value="claude-code"
              className="flex items-center gap-2 data-[state=active]:bg-background data-[state=active]:shadow-sm transition-all duration-200"
            >
              Claude Code
            </TabsTrigger>
            <TabsTrigger
              value="codex"
              className="flex items-center gap-2 data-[state=active]:bg-background data-[state=active]:shadow-sm transition-all duration-200"
            >
              CodeX
            </TabsTrigger>
            <TabsTrigger
              value="gemini-cli"
              className="flex items-center gap-2 data-[state=active]:bg-background data-[state=active]:shadow-sm transition-all duration-200"
            >
              Gemini CLI
            </TabsTrigger>
          </TabsList>

          {/* Claude Code Tab */}
          <TabsContent value="claude-code" className="mt-0">
            <ToolListSection
              toolId="claude-code"
              toolName="Claude Code"
              icon=""
              instances={groupedByTool['claude-code'] || []}
              onCheckUpdate={handleCheckUpdate}
              onUpdate={handleUpdate}
              onDelete={handleDeleteInstance}
              onVersionManage={handleVersionManage}
              updateInfoMap={updateInfoMap}
              checkingUpdate={checkingUpdate}
              updating={updating}
              viewMode={viewMode}
            />
          </TabsContent>

          {/* CodeX Tab */}
          <TabsContent value="codex" className="mt-0">
            <ToolListSection
              toolId="codex"
              toolName="CodeX"
              icon=""
              instances={groupedByTool.codex || []}
              onCheckUpdate={handleCheckUpdate}
              onUpdate={handleUpdate}
              onDelete={handleDeleteInstance}
              onVersionManage={handleVersionManage}
              updateInfoMap={updateInfoMap}
              checkingUpdate={checkingUpdate}
              updating={updating}
              viewMode={viewMode}
            />
          </TabsContent>

          {/* Gemini CLI Tab */}
          <TabsContent value="gemini-cli" className="mt-0">
            <ToolListSection
              toolId="gemini-cli"
              toolName="Gemini CLI"
              icon=""
              instances={groupedByTool['gemini-cli'] || []}
              onCheckUpdate={handleCheckUpdate}
              onUpdate={handleUpdate}
              onDelete={handleDeleteInstance}
              onVersionManage={handleVersionManage}
              updateInfoMap={updateInfoMap}
              checkingUpdate={checkingUpdate}
              updating={updating}
              viewMode={viewMode}
            />
          </TabsContent>
        </Tabs>
      )}

      {/* 添加实例对话框 */}
      <AddInstanceDialog
        open={showAddDialog}
        onClose={() => setShowAddDialog(false)}
        onAdd={handleAddInstance}
      />

      {/* 版本管理对话框 */}
      <VersionManagementDialog
        open={versionManageDialog.open}
        onClose={() => setVersionManageDialog({ open: false, instanceId: '', toolName: '' })}
        instanceId={versionManageDialog.instanceId}
        toolName={versionManageDialog.toolName}
      />
    </PageContainer>
  );
}
