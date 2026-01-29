// 透明代理管理页面
// 提供三工具透明代理的统一管理界面（重构为 Tab 架构）

import { useState, useEffect } from 'react';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Loader2 } from 'lucide-react';
import { PageContainer } from '@/components/layout/PageContainer';
import { useToast } from '@/hooks/use-toast';
import { logoMap } from '@/utils/constants';
import { ProxyControlBar } from './components/ProxyControlBar';
import { SessionListTab } from './components/tabs/SessionListTab';
import { GlobalStatsTab } from './components/tabs/GlobalStatsTab';
import { GlobalLogsTab } from './components/tabs/GlobalLogsTab';
import { SessionDetailPage } from './components/session-detail/SessionDetailPage';
import { useToolProxyData } from './hooks/useToolProxyData';
import { useProxyControl } from './hooks/useProxyControl';
import { DEFAULT_VIEW_STATE, MAIN_TABS, type ViewState } from './types/tab-types';
import type { ToolId, ToolMetadata } from './types/proxy-history';

// 支持的工具列表
const SUPPORTED_TOOLS: ToolMetadata[] = [
  { id: 'claude-code', name: 'Claude Code', icon: logoMap['claude-code'] },
  { id: 'codex', name: 'CodeX', icon: logoMap.codex },
  { id: 'gemini-cli', name: 'Gemini CLI', icon: logoMap['gemini-cli'] },
  { id: 'amp-code', name: 'AMP Code', icon: logoMap['amp-code'] },
];

interface TransparentProxyPageProps {
  selectedToolId?: string; // 从外部传入的默认选中工具 ID
}

/**
 * 透明代理管理页面（重构版）
 *
 * 功能：
 * - 三工具 Tab 切换（Claude Code / Codex / Gemini CLI）
 * - 每个工具独立的代理控制（启动/停止）
 * - 三个主 Tab：会话列表、全局统计、全局日志
 * - 会话详情页：会话统计、会话日志、会话设置
 *
 * 架构特点：
 * - Tab 驱动：扁平化 Tab 架构，清晰直观
 * - 路由管理：主页面 ↔ 会话详情页无缝切换
 * - 组件复用：统计和日志组件在多处复用
 */
export function TransparentProxyPage({ selectedToolId: initialToolId }: TransparentProxyPageProps) {
  const { toast } = useToast();
  const [selectedToolId, setSelectedToolId] = useState<ToolId>('claude-code');

  // 视图状态管理
  const [viewState, setViewState] = useState<ViewState>(DEFAULT_VIEW_STATE);

  // 当外部传入 toolId 时，更新选中状态
  useEffect(() => {
    if (
      initialToolId &&
      (initialToolId === 'claude-code' ||
        initialToolId === 'codex' ||
        initialToolId === 'gemini-cli' ||
        initialToolId === 'amp-code')
    ) {
      setSelectedToolId(initialToolId as ToolId);
    }
  }, [initialToolId]);

  // 使用数据管理 Hook
  const { getToolData, configLoading, refreshData, saveToolConfig } = useToolProxyData();

  // 使用代理控制 Hook
  const { startProxy, stopProxy, isLoading, isRunning, getPort } = useProxyControl();

  /**
   * 导航到会话详情页
   */
  const navigateToSessionDetail = (sessionId: string) => {
    setViewState({
      ...viewState,
      mode: 'session-detail',
      selectedSessionId: sessionId,
      sessionDetailTab: 'session-stats', // 重置为第一个 Tab
    });
  };

  /**
   * 返回主页面
   */
  const navigateToMain = () => {
    setViewState({
      ...viewState,
      mode: 'main',
      selectedSessionId: null,
    });
  };

  /**
   * 启动代理处理
   */
  const handleStartProxy = async (toolId: ToolId) => {
    const result = await startProxy(toolId);
    toast({
      title: result.success ? '启动成功' : '启动失败',
      description: result.message,
      variant: result.success ? 'default' : 'destructive',
    });
  };

  /**
   * 停止代理处理
   */
  const handleStopProxy = async (toolId: ToolId) => {
    const result = await stopProxy(toolId);
    toast({
      title: result.success ? '停止成功' : '停止失败',
      description: result.message,
      variant: result.success ? 'default' : 'destructive',
    });
  };

  // 加载状态
  if (configLoading) {
    return (
      <PageContainer>
        <div className="flex items-center justify-center py-20">
          <Loader2 className="h-8 w-8 animate-spin text-primary" />
          <span className="ml-3 text-muted-foreground">加载配置中...</span>
        </div>
      </PageContainer>
    );
  }

  return (
    <PageContainer
      title="透明代理"
      description="为不同 AI 编程工具提供统一的透明代理服务，支持配置热切换"
    >
      {/* 四工具 Tab 切换 */}
      <Tabs value={selectedToolId} onValueChange={(val) => setSelectedToolId(val as ToolId)}>
        <TabsList className="grid w-full grid-cols-4 mb-4 h-9 p-1 bg-muted/50 rounded-lg">
          {SUPPORTED_TOOLS.map((tool) => (
            <TabsTrigger
              key={tool.id}
              value={tool.id}
              className="gap-2 data-[state=active]:bg-background data-[state=active]:shadow-sm transition-all duration-200"
            >
              <img src={tool.icon} alt={tool.name} className="w-4 h-4" />
              {tool.name}
            </TabsTrigger>
          ))}
        </TabsList>

        {/* 每个工具的内容 */}
        {SUPPORTED_TOOLS.map((tool) => {
          const toolData = getToolData(tool.id);
          const toolIsRunning = isRunning(tool.id);
          const toolPort = getPort(tool.id);
          const toolIsLoading = isLoading(tool.id);
          const toolIsConfigured = !!(toolData.config?.local_api_key && toolData.config?.enabled);

          return (
            <TabsContent key={tool.id} value={tool.id} className="space-y-0">
              {/* 代理控制条 - 仅在主页面显示 */}
              {viewState.mode === 'main' && (
                <ProxyControlBar
                  tool={tool}
                  isRunning={toolIsRunning}
                  port={toolPort}
                  isLoading={toolIsLoading}
                  isConfigured={toolIsConfigured}
                  config={toolData.config}
                  onStart={() => handleStartProxy(tool.id)}
                  onStop={() => handleStopProxy(tool.id)}
                  onConfigUpdated={refreshData}
                  onSaveSettings={(updates) => saveToolConfig(tool.id, updates)}
                />
              )}

              {/* 根据视图模式渲染内容 */}
              {viewState.mode === 'main' ? (
                // 主页面：三个 Tab（会话列表、全局统计、全局日志）
                <Tabs
                  value={viewState.mainTab}
                  onValueChange={(val) => setViewState({ ...viewState, mainTab: val as any })}
                >
                  <TabsList className="grid w-full grid-cols-3 mt-3">
                    {MAIN_TABS.map((tab) => (
                      <TabsTrigger key={tab.id} value={tab.id}>
                        {tab.icon} {tab.label}
                      </TabsTrigger>
                    ))}
                  </TabsList>

                  <TabsContent value="session-list">
                    <SessionListTab toolId={tool.id} onNavigateToDetail={navigateToSessionDetail} />
                  </TabsContent>

                  <TabsContent value="global-stats">
                    <GlobalStatsTab toolId={tool.id} />
                  </TabsContent>

                  <TabsContent value="global-logs">
                    <GlobalLogsTab toolId={tool.id} />
                  </TabsContent>
                </Tabs>
              ) : (
                // 会话详情页
                <SessionDetailPage
                  sessionId={viewState.selectedSessionId!}
                  toolId={tool.id}
                  onBack={navigateToMain}
                />
              )}
            </TabsContent>
          );
        })}
      </Tabs>
    </PageContainer>
  );
}
