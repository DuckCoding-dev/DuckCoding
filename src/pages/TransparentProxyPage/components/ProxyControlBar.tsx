// 代理控制条组件
// 提供代理启动/停止控制按钮

import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Loader2, Power, AlertCircle } from 'lucide-react';
import type { ToolMetadata } from '../types/proxy-history';

interface ProxyControlBarProps {
  /** 工具元数据 */
  tool: ToolMetadata;
  /** 代理是否运行中 */
  isRunning: boolean;
  /** 代理端口 */
  port: number | null;
  /** 是否加载中（启动中或停止中） */
  isLoading: boolean;
  /** 是否已配置（有 API Key） */
  isConfigured: boolean;
  /** 启动代理回调 */
  onStart: () => void;
  /** 停止代理回调 */
  onStop: () => void;
}

/**
 * 代理控制条组件
 *
 * 功能：
 * - 显示当前工具的代理运行状态
 * - 提供启动/停止代理按钮
 * - 复用 MultiToolProxySettings 的按钮样式
 */
export function ProxyControlBar({
  tool,
  isRunning,
  port,
  isLoading,
  isConfigured,
  onStart,
  onStop,
}: ProxyControlBarProps) {
  return (
    <div
      className={`p-4 rounded-lg border-2 mb-6 transition-all ${
        isRunning
          ? 'bg-gradient-to-r from-blue-50 to-indigo-50 dark:from-blue-950 dark:to-indigo-950 border-blue-300 dark:border-blue-700'
          : 'bg-muted/30 border-border'
      }`}
    >
      <div className="flex items-center justify-between">
        {/* 左侧：状态信息 */}
        <div className="flex items-center gap-3">
          <div>
            <div className="flex items-center gap-2 mb-1">
              <h4 className="font-semibold">{tool.name} 透明代理</h4>
              <Badge variant={isRunning ? 'default' : 'secondary'} className="text-xs">
                {isRunning ? `运行中 (端口 ${port})` : '已停止'}
              </Badge>
            </div>
            <p className="text-xs text-muted-foreground">
              {isRunning
                ? `代理地址：http://127.0.0.1:${port}`
                : isConfigured
                  ? '点击「启动代理」开始使用'
                  : '请先在全局设置中配置透明代理'}
            </p>
          </div>
        </div>

        {/* 右侧：控制按钮 */}
        <div className="flex items-center gap-2">
          {!isConfigured && !isRunning && (
            <div className="flex items-center gap-1 text-xs text-amber-600 dark:text-amber-500 mr-2">
              <AlertCircle className="h-3 w-3" />
              <span>未配置</span>
            </div>
          )}

          {isRunning ? (
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={onStop}
              disabled={isLoading}
              className="h-8"
            >
              {isLoading ? (
                <>
                  <Loader2 className="h-3 w-3 mr-1 animate-spin" />
                  停止中...
                </>
              ) : (
                <>
                  <Power className="h-3 w-3 mr-1" />
                  停止代理
                </>
              )}
            </Button>
          ) : (
            <Button
              type="button"
              variant="default"
              size="sm"
              onClick={onStart}
              disabled={isLoading || !isConfigured}
              className="h-8"
            >
              {isLoading ? (
                <>
                  <Loader2 className="h-3 w-3 mr-1 animate-spin" />
                  启动中...
                </>
              ) : (
                <>
                  <Power className="h-3 w-3 mr-1" />
                  启动代理
                </>
              )}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
