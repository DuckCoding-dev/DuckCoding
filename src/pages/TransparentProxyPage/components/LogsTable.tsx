// Token 日志历史表格组件
// 展示历史请求日志，支持分页和过滤

import { useState, useEffect, useCallback, Fragment } from 'react';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Loader2, ChevronLeft, ChevronRight, ChevronDown, ChevronUp, Search } from 'lucide-react';
import { CustomTimeRangeDialog } from '@/components/dialogs/CustomTimeRangeDialog';
import { queryTokenLogs } from '@/lib/tauri-commands';
import type { TokenLog, TokenLogsPage } from '@/types/token-stats';
import {
  TOOL_TYPE_NAMES,
  TIME_RANGE_OPTIONS,
  DEFAULT_PAGE_SIZE,
  RESPONSE_TYPE_NAMES,
  type ToolType,
} from '@/types/token-stats';

interface LogsTableProps {
  /** 初始工具类型过滤 */
  initialToolType?: ToolType;
  /** 初始会话 ID 过滤 */
  initialSessionId?: string;
  /** 是否隐藏会话ID搜索框（默认 false） */
  hideSessionIdFilter?: boolean;
}

/**
 * 格式化时间戳为可读时间
 */
function formatTimestamp(timestamp: number): string {
  return new Date(timestamp).toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

/**
 * 格式化 Token 数量
 */
function formatTokens(count: number): string {
  return count.toLocaleString('zh-CN');
}

/**
 * Token 日志历史表格组件
 */
export function LogsTable({
  initialToolType,
  initialSessionId,
  hideSessionIdFilter = false,
}: LogsTableProps) {
  // 查询参数
  const [page, setPage] = useState(0);
  const [pageSize] = useState(DEFAULT_PAGE_SIZE);
  const [toolTypeFilter, setToolTypeFilter] = useState<string | undefined>(initialToolType);
  const [sessionIdFilter, setSessionIdFilter] = useState<string>(initialSessionId ?? '');
  const [configNameFilter, setConfigNameFilter] = useState<string>('');
  const [timeRangeFilter, setTimeRangeFilter] = useState<string>('all');

  // 自定义时间范围状态
  const [timeRangeMode, setTimeRangeMode] = useState<'preset' | 'custom'>('preset');
  const [customStartTime, setCustomStartTime] = useState<Date | null>(
    new Date(Date.now() - 7 * 24 * 60 * 60 * 1000),
  );
  const [customEndTime, setCustomEndTime] = useState<Date | null>(new Date());
  const [showCustomDialog, setShowCustomDialog] = useState(false);

  // 视图状态
  const [expandedRows, setExpandedRows] = useState<Set<number>>(new Set()); // 展开的行ID集合

  // 切换行展开状态
  const toggleRowExpansion = (logId: number) => {
    setExpandedRows((prev) => {
      const newSet = new Set(prev);
      if (newSet.has(logId)) {
        newSet.delete(logId);
      } else {
        newSet.add(logId);
      }
      return newSet;
    });
  };

  // 数据状态
  const [data, setData] = useState<TokenLogsPage | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // 获取日志数据
  const fetchLogs = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      // 构建查询参数 - 支持预设和自定义时间范围
      let start_time: number | undefined;
      let end_time: number | undefined;

      if (timeRangeMode === 'preset') {
        const timeRange = TIME_RANGE_OPTIONS.find((opt) => opt.value === timeRangeFilter);
        const range = timeRange?.getRange() ?? {};
        start_time = range.start_time;
        end_time = range.end_time;
      } else {
        // 自定义时间范围
        start_time = Math.floor(customStartTime.getTime() / 1000);
        end_time = Math.floor(customEndTime.getTime() / 1000);
      }

      const result = await queryTokenLogs({
        tool_type: toolTypeFilter,
        session_id: sessionIdFilter || undefined,
        config_name: configNameFilter || undefined,
        start_time,
        end_time,
        page,
        page_size: pageSize,
      });

      setData(result);
    } catch (err) {
      console.error('Failed to fetch token logs:', err);
      setError(err instanceof Error ? err.message : '加载日志失败');
    } finally {
      setIsLoading(false);
    }
  }, [
    page,
    pageSize,
    toolTypeFilter,
    sessionIdFilter,
    configNameFilter,
    timeRangeFilter,
    timeRangeMode,
    customStartTime,
    customEndTime,
  ]);

  // 初始加载和过滤器变更时重新加载
  useEffect(() => {
    fetchLogs();
  }, [fetchLogs]);

  // 重置过滤器
  const handleResetFilters = () => {
    setToolTypeFilter(undefined);
    setSessionIdFilter('');
    setConfigNameFilter('');
    setTimeRangeFilter('all');
    setTimeRangeMode('preset');
    setPage(0);
  };

  // 处理时间范围变更
  const handleTimeRangeChange = (value: string) => {
    if (value === 'custom') {
      setShowCustomDialog(true);
    } else {
      setTimeRangeMode('preset');
      setTimeRangeFilter(value);
    }
  };

  // 确认自定义时间范围
  const handleConfirmCustomTime = () => {
    setTimeRangeMode('custom');
    setShowCustomDialog(false);
  };

  // 分页控制
  const totalPages = data ? Math.ceil(data.total / pageSize) : 0;
  const canGoPrevious = page > 0;
  const canGoNext = page < totalPages - 1;

  const handlePreviousPage = () => {
    if (canGoPrevious) setPage(page - 1);
  };

  const handleNextPage = () => {
    if (canGoNext) setPage(page + 1);
  };

  return (
    <Card>
      <CardContent className="pt-6">
        {/* 过滤器 */}
        <div className="mb-4 space-y-3">
          {/* 第一行：时间范围选择 */}
          <div className="flex items-center gap-3">
            <span className="text-sm font-medium text-muted-foreground">时间范围</span>
            <div className="flex items-center gap-2">
              <Tabs
                value={timeRangeMode === 'preset' ? timeRangeFilter : 'custom'}
                onValueChange={handleTimeRangeChange}
                className="flex-1"
              >
                <TabsList>
                  <TabsTrigger value="today">今天</TabsTrigger>
                  <TabsTrigger value="week">7天</TabsTrigger>
                  <TabsTrigger value="month">30天</TabsTrigger>

                  {/* 更多预设范围 */}
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button
                        variant={
                          timeRangeMode === 'preset' &&
                          !['today', 'week', 'month', 'all', 'custom'].includes(timeRangeFilter)
                            ? 'default'
                            : 'ghost'
                        }
                        size="sm"
                        className="h-9 px-3"
                      >
                        更多
                        <ChevronDown className="ml-1 h-4 w-4" />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="start">
                      <DropdownMenuItem onClick={() => handleTimeRangeChange('all')}>
                        全部
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>

                  <TabsTrigger value="custom">自定义</TabsTrigger>
                </TabsList>
              </Tabs>

              {/* 刷新按钮 */}
              <Button variant="ghost" size="sm" onClick={fetchLogs} disabled={isLoading}>
                <Loader2 className={`h-4 w-4 ${isLoading ? 'animate-spin' : ''}`} />
              </Button>
            </div>
          </div>

          {/* 第二行：搜索过滤 */}
          <div className="flex items-center gap-3">
            {/* 会话 ID 搜索 */}
            {!hideSessionIdFilter && (
              <Input
                placeholder="搜索会话 ID..."
                value={sessionIdFilter}
                onChange={(e) => setSessionIdFilter(e.target.value)}
                className="max-w-xs"
              />
            )}

            {/* 配置名称搜索 */}
            <Input
              placeholder="搜索配置名称..."
              value={configNameFilter}
              onChange={(e) => setConfigNameFilter(e.target.value)}
              className="max-w-xs"
            />

            {/* 查询按钮 */}
            <Button variant="outline" size="sm" onClick={fetchLogs} disabled={isLoading}>
              {isLoading ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Search className="h-4 w-4" />
              )}
            </Button>

            {/* 重置按钮 */}
            <Button variant="ghost" size="sm" onClick={handleResetFilters}>
              重置
            </Button>

            {/* 统计信息 */}
            {data && (
              <div className="ml-auto text-sm text-muted-foreground">共 {data.total} 条记录</div>
            )}
          </div>
        </div>

        {/* 加载状态 */}
        {isLoading && (
          <div className="flex items-center justify-center py-8">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            <span className="ml-2 text-sm text-muted-foreground">加载中...</span>
          </div>
        )}

        {/* 错误状态 */}
        {error && !isLoading && (
          <div className="flex items-center justify-center py-8 text-destructive">
            <p className="text-sm">{error}</p>
          </div>
        )}

        {/* 表格内容 */}
        {!isLoading && !error && data && (
          <>
            <div className="rounded-md border">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-[50px]"></TableHead>
                    <TableHead>时间</TableHead>
                    <TableHead>工具</TableHead>
                    <TableHead>状态</TableHead>
                    <TableHead>类型</TableHead>
                    <TableHead>会话ID</TableHead>
                    <TableHead>配置</TableHead>
                    <TableHead>模型</TableHead>
                    <TableHead className="text-right">总计</TableHead>
                    <TableHead className="text-right">总成本</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {data.logs.length === 0 ? (
                    <TableRow>
                      <TableCell colSpan={10} className="text-center text-muted-foreground py-8">
                        暂无日志记录
                      </TableCell>
                    </TableRow>
                  ) : (
                    data.logs.map((log: TokenLog) => {
                      const isExpanded = expandedRows.has(log.id ?? 0);
                      return (
                        <Fragment key={log.id}>
                          <TableRow className="cursor-pointer hover:bg-muted/50">
                            <TableCell>
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => toggleRowExpansion(log.id ?? 0)}
                                className="h-6 w-6 p-0"
                              >
                                {isExpanded ? (
                                  <ChevronUp className="h-4 w-4" />
                                ) : (
                                  <ChevronDown className="h-4 w-4" />
                                )}
                              </Button>
                            </TableCell>
                            <TableCell className="text-xs whitespace-nowrap">
                              {formatTimestamp(log.timestamp)}
                            </TableCell>
                            <TableCell>
                              <Badge variant="outline" className="text-xs">
                                {TOOL_TYPE_NAMES[log.tool_type as ToolType] ?? log.tool_type}
                              </Badge>
                            </TableCell>
                            <TableCell>
                              <Badge
                                variant="outline"
                                className={`text-xs ${
                                  log.request_status === 'success'
                                    ? 'text-green-700 bg-green-50 border-green-200'
                                    : 'text-red-700 bg-red-50 border-red-200'
                                }`}
                              >
                                {log.request_status === 'success' ? '成功' : '失败'}
                              </Badge>
                            </TableCell>
                            <TableCell>
                              <span className="text-xs text-muted-foreground">
                                {RESPONSE_TYPE_NAMES[
                                  log.response_type as 'sse' | 'json' | 'unknown'
                                ] || '未知'}
                              </span>
                            </TableCell>
                            <TableCell className="text-xs font-mono">
                              {log.session_id.substring(0, 8)}
                            </TableCell>
                            <TableCell className="text-xs">{log.config_name}</TableCell>
                            <TableCell className="text-xs max-w-[150px] truncate" title={log.model}>
                              {log.model}
                            </TableCell>
                            <TableCell className="text-right text-xs tabular-nums font-medium">
                              {formatTokens(
                                log.input_tokens +
                                  log.output_tokens +
                                  log.cache_creation_tokens +
                                  log.cache_read_tokens,
                              )}
                            </TableCell>
                            <TableCell className="text-right text-xs tabular-nums font-medium">
                              ${log.total_cost.toFixed(6)}
                            </TableCell>
                          </TableRow>
                          {isExpanded && (
                            <TableRow key={`details-${log.id}`}>
                              <TableCell colSpan={10} className="bg-muted/30 p-4">
                                <div className="space-y-3 text-sm">
                                  <div className="grid grid-cols-2 gap-x-4 gap-y-2">
                                    <div className="flex justify-between">
                                      <span className="text-muted-foreground">会话ID:</span>
                                      <span className="font-mono text-xs">{log.session_id}</span>
                                    </div>
                                    <div className="flex justify-between">
                                      <span className="text-muted-foreground">客户端IP:</span>
                                      <span className="font-mono text-xs">{log.client_ip}</span>
                                    </div>
                                    <div className="flex justify-between">
                                      <span className="text-muted-foreground">输入 Token:</span>
                                      <span className="font-mono">
                                        {formatTokens(log.input_tokens)}
                                      </span>
                                    </div>
                                    <div className="flex justify-between">
                                      <span className="text-muted-foreground">输出 Token:</span>
                                      <span className="font-mono">
                                        {formatTokens(log.output_tokens)}
                                      </span>
                                    </div>
                                    <div className="flex justify-between">
                                      <span className="text-muted-foreground">缓存创建:</span>
                                      <span className="font-mono">
                                        {formatTokens(log.cache_creation_tokens)}
                                      </span>
                                    </div>
                                    <div className="flex justify-between">
                                      <span className="text-muted-foreground">缓存读取:</span>
                                      <span className="font-mono">
                                        {formatTokens(log.cache_read_tokens)}
                                      </span>
                                    </div>
                                  </div>
                                  {/* 成本信息 */}
                                  {log.request_status === 'success' && log.total_cost > 0 && (
                                    <div className="pt-2 border-t">
                                      <div className="grid grid-cols-2 gap-x-4 gap-y-2">
                                        <div className="flex justify-between">
                                          <span className="text-muted-foreground">输入成本:</span>
                                          <span className="font-mono text-xs">
                                            ${log.input_price?.toFixed(6) ?? '0.000000'}
                                          </span>
                                        </div>
                                        <div className="flex justify-between">
                                          <span className="text-muted-foreground">输出成本:</span>
                                          <span className="font-mono text-xs">
                                            ${log.output_price?.toFixed(6) ?? '0.000000'}
                                          </span>
                                        </div>
                                        <div className="flex justify-between">
                                          <span className="text-muted-foreground">
                                            缓存写入成本:
                                          </span>
                                          <span className="font-mono text-xs">
                                            ${log.cache_write_price?.toFixed(6) ?? '0.000000'}
                                          </span>
                                        </div>
                                        <div className="flex justify-between">
                                          <span className="text-muted-foreground">
                                            缓存读取成本:
                                          </span>
                                          <span className="font-mono text-xs">
                                            ${log.cache_read_price?.toFixed(6) ?? '0.000000'}
                                          </span>
                                        </div>
                                        <div className="flex justify-between col-span-2 font-semibold">
                                          <span className="text-muted-foreground">总成本:</span>
                                          <span className="font-mono text-xs">
                                            ${log.total_cost.toFixed(6)}
                                          </span>
                                        </div>
                                      </div>
                                    </div>
                                  )}
                                  {log.message_id && (
                                    <div className="flex justify-between col-span-2">
                                      <span className="text-muted-foreground">消息ID:</span>
                                      <span className="font-mono text-xs">{log.message_id}</span>
                                    </div>
                                  )}
                                  {log.request_status === 'failed' && log.error_type && (
                                    <div className="pt-2 border-t">
                                      <div className="flex items-start gap-2">
                                        <Badge variant="destructive" className="text-xs">
                                          {log.error_type === 'parse_error'
                                            ? '解析失败'
                                            : log.error_type === 'request_interrupted'
                                              ? '请求中断'
                                              : log.error_type === 'upstream_error'
                                                ? '上游错误'
                                                : log.error_type}
                                        </Badge>
                                        {log.error_detail && (
                                          <span className="text-xs text-muted-foreground flex-1">
                                            {log.error_detail}
                                          </span>
                                        )}
                                      </div>
                                    </div>
                                  )}
                                </div>
                              </TableCell>
                            </TableRow>
                          )}
                        </Fragment>
                      );
                    })
                  )}
                </TableBody>
              </Table>
            </div>

            {/* 分页控制 */}
            {totalPages > 1 && (
              <div className="flex items-center justify-between mt-4">
                <div className="text-sm text-muted-foreground">
                  第 {page + 1} 页，共 {totalPages} 页
                </div>
                <div className="flex items-center gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={handlePreviousPage}
                    disabled={!canGoPrevious}
                  >
                    <ChevronLeft className="h-4 w-4" />
                    上一页
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={handleNextPage}
                    disabled={!canGoNext}
                  >
                    下一页
                    <ChevronRight className="h-4 w-4" />
                  </Button>
                </div>
              </div>
            )}
          </>
        )}
      </CardContent>

      {/* 自定义时间范围对话框 */}
      <CustomTimeRangeDialog
        open={showCustomDialog}
        onOpenChange={setShowCustomDialog}
        startTime={customStartTime}
        endTime={customEndTime}
        onStartTimeChange={setCustomStartTime}
        onEndTimeChange={setCustomEndTime}
        onConfirm={handleConfirmCustomTime}
      />
    </Card>
  );
}
