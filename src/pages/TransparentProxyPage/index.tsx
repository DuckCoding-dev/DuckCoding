// 透明代理管理页面
// 提供三工具透明代理的统一管理界面

import { useState, useEffect } from 'react';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Loader2 } from 'lucide-react';
import { PageContainer } from '@/components/layout/PageContainer';
import { useToast } from '@/hooks/use-toast';
import { logoMap } from '@/utils/constants';
import { ProxyControlBar } from './components/ProxyControlBar';
import { ToolContent } from './components/ToolContent';
import { useToolProxyData } from './hooks/useToolProxyData';
import { useProxyControl } from './hooks/useProxyControl';
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
 * 透明代理管理页面
 *
 * 功能：
 * - 三工具 Tab 切换（Claude Code / Codex / Gemini CLI）
 * - 每个工具独立的代理控制（启动/停止）
 * - Claude Code 显示会话历史表格
 * - Codex / Gemini CLI 显示占位文本
 *
 * 架构特点：
 * - 工厂模式：ToolContent 根据 toolId 渲染不同组件
 * - 单一职责：每个组件负责特定功能
 * - 开放封闭：新增工具只需扩展工厂，无需修改主逻辑
 */
export function TransparentProxyPage({ selectedToolId: initialToolId }: TransparentProxyPageProps) {
  const { toast } = useToast();
  const [selectedToolId, setSelectedToolId] = useState<ToolId>('claude-code');

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
    <PageContainer>
      {/* 页面标题 */}
      <div className="mb-6">
        <h2 className="text-2xl font-semibold mb-1">透明代理</h2>
        <p className="text-sm text-muted-foreground">
          为不同 AI 编程工具提供统一的透明代理服务，支持配置热切换
        </p>
      </div>

      {/* 四工具 Tab 切换 */}
      <Tabs value={selectedToolId} onValueChange={(val) => setSelectedToolId(val as ToolId)}>
        <TabsList className="grid w-full grid-cols-4 mb-6">
          {SUPPORTED_TOOLS.map((tool) => (
            <TabsTrigger key={tool.id} value={tool.id} className="gap-2">
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
              {/* 代理控制条 */}
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

              {/* 工具特定内容（工厂渲染） */}
              <ToolContent toolId={tool.id} />
            </TabsContent>
          );
        })}
      </Tabs>
    </PageContainer>
  );
}
