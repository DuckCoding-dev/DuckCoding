import { useState } from 'react';
import { Loader2 } from 'lucide-react';
import { PageContainer } from '@/components/layout/PageContainer';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Button } from '@/components/ui/button';
import { ToolListSection } from './components/ToolListSection';
import { AddInstanceDialog } from './components/AddInstanceDialog';
import { VersionManagementDialog } from './components/VersionManagementDialog';
import { useToolManagement } from './hooks/useToolManagement';
import type { ToolStatus } from '@/lib/tauri-commands';

interface ToolManagementPageProps {
  tools: ToolStatus[];
  loading: boolean;
}

export function ToolManagementPage({
  tools: _toolsProp,
  loading: loadingProp,
}: ToolManagementPageProps) {
  // _toolsProp 和 loadingProp 用于全局缓存，但工具管理需要更详细的 ToolInstance 数据
  // 所以仍然需要加载完整的工具实例信息
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
    window.dispatchEvent(new CustomEvent('refresh-tools'));
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

  return (
    <PageContainer>
      {/* 页面标题和操作按钮 */}
      <div className="mb-6 flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-semibold mb-1">工具管理</h2>
          <p className="text-sm text-muted-foreground">管理所有 AI 开发工具的安装和配置</p>
        </div>
        <div className="flex gap-2">
          <Button
            variant="default"
            onClick={() => {
              // 导航到安装页面
              window.dispatchEvent(new CustomEvent('navigate-to-install'));
            }}
          >
            安装工具
          </Button>
          <Button
            variant="outline"
            disabled
            title="功能开发中"
            onClick={() => setShowAddDialog(true)}
          >
            添加实例
          </Button>
          <Button variant="outline" onClick={onRefreshTools}>
            刷新状态
          </Button>
        </div>
      </div>

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
          <TabsList className="grid w-full grid-cols-3 mb-6">
            <TabsTrigger value="claude-code" className="flex items-center gap-2">
              Claude Code
            </TabsTrigger>
            <TabsTrigger value="codex" className="flex items-center gap-2">
              CodeX
            </TabsTrigger>
            <TabsTrigger value="gemini-cli" className="flex items-center gap-2">
              Gemini CLI
            </TabsTrigger>
          </TabsList>

          {/* Claude Code Tab */}
          <TabsContent value="claude-code">
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
            />
          </TabsContent>

          {/* CodeX Tab */}
          <TabsContent value="codex">
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
            />
          </TabsContent>

          {/* Gemini CLI Tab */}
          <TabsContent value="gemini-cli">
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
