// 会话列表 Tab 组件
// 展示所有会话，支持搜索、过滤、排序

import { useState, useMemo } from 'react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Loader2, RefreshCw, ChevronRight, Trash2, Settings, Pencil, FileText } from 'lucide-react';
import { useSessionData } from '../../hooks/useSessionData';
import { SessionConfigDialog } from '../SessionConfigDialog';
import { SessionNoteDialog } from '../SessionNoteDialog';
import { isActiveSession } from '@/utils/sessionHelpers';
import type { ToolId } from '../../types/proxy-history';
import type { SessionRecord } from '@/lib/tauri-commands';

interface SessionListTabProps {
  toolId: ToolId;
  onNavigateToDetail: (sessionId: string) => void;
}

/**
 * 渲染配置显示内容
 */
function ConfigBadge({ session }: { session: SessionRecord }) {
  if (session.config_name === 'global') {
    return (
      <Badge variant="outline" className="text-green-700 bg-green-50 border-green-200">
        跟随主配置
      </Badge>
    );
  }

  const displayName = session.custom_profile_name || '自定义';
  return (
    <Badge variant="outline" className="text-blue-700 bg-blue-50 border-blue-200">
      {displayName}
    </Badge>
  );
}

/**
 * 空状态组件
 */
function EmptyState() {
  return (
    <div className="flex flex-col items-center justify-center py-16 text-center">
      <FileText className="h-12 w-12 text-muted-foreground mb-4" />
      <h3 className="text-lg font-semibold mb-2">暂无会话记录</h3>
      <p className="text-sm text-muted-foreground max-w-md">启动代理后，会话记录将显示在此处。</p>
    </div>
  );
}

/**
 * 会话列表 Tab 组件
 */
export function SessionListTab({ toolId, onNavigateToDetail }: SessionListTabProps) {
  const {
    sessions,
    total,
    loading,
    deleteSession,
    refresh,
    page,
    totalPages,
    canGoPrevious,
    canGoNext,
    nextPage,
    previousPage,
  } = useSessionData(toolId, { pageSize: 20 });

  // 过滤和排序状态
  const [searchTerm, setSearchTerm] = useState('');
  const [statusFilter, setStatusFilter] = useState<'all' | 'active' | 'idle'>('all');
  const [sortBy, setSortBy] = useState<'time' | 'requests'>('time');

  // 对话框状态
  const [configDialogOpen, setConfigDialogOpen] = useState(false);
  const [noteDialogOpen, setNoteDialogOpen] = useState(false);
  const [selectedSession, setSelectedSession] = useState<SessionRecord | null>(null);

  // 过滤和排序逻辑（客户端过滤当前页数据）
  const filteredSessions = useMemo(() => {
    let result = sessions;

    // 搜索过滤
    if (searchTerm) {
      const lowerSearch = searchTerm.toLowerCase();
      result = result.filter(
        (s) =>
          s.display_id.toLowerCase().includes(lowerSearch) ||
          s.note?.toLowerCase().includes(lowerSearch),
      );
    }

    // 状态过滤
    if (statusFilter !== 'all') {
      result = result.filter((s) =>
        statusFilter === 'active'
          ? isActiveSession(s.last_seen_at)
          : !isActiveSession(s.last_seen_at),
      );
    }

    // 排序
    result = [...result].sort((a, b) => {
      if (sortBy === 'time') {
        return b.first_seen_at - a.first_seen_at;
      } else {
        return b.request_count - a.request_count;
      }
    });

    return result;
  }, [sessions, searchTerm, statusFilter, sortBy]);

  // 格式化时间
  const formatTime = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  return (
    <div className="space-y-4 mt-4">
      {/* 过滤栏 */}
      <div className="flex items-center gap-3">
        <Input
          placeholder="搜索会话（ID 或备注）..."
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          className="max-w-xs"
        />
        <Select value={statusFilter} onValueChange={(v) => setStatusFilter(v as any)}>
          <SelectTrigger className="w-[120px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">全部</SelectItem>
            <SelectItem value="active">活跃</SelectItem>
            <SelectItem value="idle">空闲</SelectItem>
          </SelectContent>
        </Select>
        <Select value={sortBy} onValueChange={(v) => setSortBy(v as any)}>
          <SelectTrigger className="w-[140px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="time">按时间排序</SelectItem>
            <SelectItem value="requests">按请求数排序</SelectItem>
          </SelectContent>
        </Select>
        <Button variant="ghost" size="sm" onClick={refresh} disabled={loading}>
          <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
        </Button>
        <div className="ml-auto text-sm text-muted-foreground">
          共 {filteredSessions.length} 个会话
        </div>
      </div>

      {/* 会话表格 */}
      {loading && sessions.length === 0 ? (
        <div className="flex justify-center py-12">
          <Loader2 className="h-8 w-8 animate-spin text-primary" />
          <span className="ml-3 text-muted-foreground">加载会话记录中...</span>
        </div>
      ) : filteredSessions.length === 0 ? (
        <EmptyState />
      ) : (
        <div className="rounded-lg border overflow-hidden">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-[220px]">会话标识符</TableHead>
                <TableHead className="w-[100px]">状态</TableHead>
                <TableHead className="w-[180px]">启动时间</TableHead>
                <TableHead className="w-[120px]">请求次数</TableHead>
                <TableHead>配置</TableHead>
                <TableHead className="w-[160px] text-right">操作</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filteredSessions.map((session) => (
                <TableRow key={session.session_id} className="hover:bg-muted/30 transition-colors">
                  <TableCell>
                    <div className="flex items-center gap-2">
                      <span className="font-semibold">{session.note || '未命名'}</span>
                      <Badge variant="outline" className="font-mono text-xs">
                        {session.display_id.slice(0, 8)}
                      </Badge>
                    </div>
                  </TableCell>
                  <TableCell>
                    {isActiveSession(session.last_seen_at) ? (
                      <Badge variant="default" className="bg-green-500 hover:bg-green-600">
                        活跃
                      </Badge>
                    ) : (
                      <Badge variant="secondary" className="text-muted-foreground">
                        空闲
                      </Badge>
                    )}
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {formatTime(session.first_seen_at)}
                  </TableCell>
                  <TableCell>
                    <Badge variant="outline" className="text-blue-700 bg-blue-50 border-blue-200">
                      {session.request_count} 次
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <ConfigBadge session={session} />
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex items-center justify-end gap-1">
                      {/* 查看详情按钮 */}
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-8"
                        onClick={() => onNavigateToDetail(session.session_id)}
                        title="查看详情"
                      >
                        <ChevronRight className="h-4 w-4" />
                      </Button>
                      {/* 编辑备注按钮 */}
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-8"
                        onClick={() => {
                          setSelectedSession(session);
                          setNoteDialogOpen(true);
                        }}
                        title="编辑备注"
                      >
                        <Pencil className="h-3 w-3" />
                      </Button>
                      {/* 配置按钮 */}
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-8"
                        onClick={() => {
                          setSelectedSession(session);
                          setConfigDialogOpen(true);
                        }}
                        title="切换配置"
                      >
                        <Settings className="h-3 w-3" />
                      </Button>
                      {/* 删除按钮 */}
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-8"
                        onClick={() => deleteSession(session.session_id)}
                        title="删除会话"
                      >
                        <Trash2 className="h-3 w-3" />
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}

      {/* 分页控制 */}
      {totalPages > 1 && filteredSessions.length > 0 && (
        <div className="flex items-center justify-between mt-4">
          <div className="text-sm text-muted-foreground">
            第 {page} 页，共 {totalPages} 页（总计 {total} 个会话）
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={previousPage}
              disabled={!canGoPrevious || loading}
            >
              <ChevronRight className="h-4 w-4 rotate-180" />
              上一页
            </Button>
            <Button variant="outline" size="sm" onClick={nextPage} disabled={!canGoNext || loading}>
              下一页
              <ChevronRight className="h-4 w-4" />
            </Button>
          </div>
        </div>
      )}

      {/* 配置切换弹窗 */}
      {selectedSession && (
        <SessionConfigDialog
          open={configDialogOpen}
          onOpenChange={setConfigDialogOpen}
          sessionId={selectedSession.session_id}
          currentConfig={selectedSession.config_name}
          currentCustomProfileName={selectedSession.custom_profile_name}
          onConfigUpdated={refresh}
        />
      )}

      {/* 备注编辑弹窗 */}
      {selectedSession && (
        <SessionNoteDialog
          open={noteDialogOpen}
          onOpenChange={setNoteDialogOpen}
          sessionId={selectedSession.session_id}
          currentNote={selectedSession.note}
          onNoteUpdated={refresh}
        />
      )}
    </div>
  );
}
