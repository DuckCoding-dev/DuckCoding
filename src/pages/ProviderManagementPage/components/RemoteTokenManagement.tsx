// Remote Token Management Component
//
// 远程令牌管理组件 - 显示和管理供应商的远程令牌

import { useState, useEffect, useCallback } from 'react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import {
  Loader2,
  Plus,
  Download,
  Trash2,
  RefreshCw,
  Pencil,
  ChevronLeft,
  ChevronRight,
} from 'lucide-react';
import type { Provider } from '@/types/provider';
import type { RemoteToken } from '@/types/remote-token';
import { TOKEN_STATUS_TEXT, TOKEN_STATUS_VARIANT, TokenStatus } from '@/types/remote-token';
import { fetchProviderTokens, deleteProviderToken } from '@/lib/tauri-commands/token';
import { useToast } from '@/hooks/use-toast';
import { TokenFormDialog } from './TokenFormDialog';
import { ImportTokenDialog } from './ImportTokenDialog';
import { TokenCard } from './TokenCard';
import { ViewToggle, ViewMode } from '@/components/common/ViewToggle';

interface RemoteTokenManagementProps {
  provider: Provider;
}

/**
 * 远程令牌管理组件
 */
const PAGE_SIZE = 10;

export function RemoteTokenManagement({ provider }: RemoteTokenManagementProps) {
  const { toast } = useToast();
  const [tokens, setTokens] = useState<RemoteToken[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [tokenFormMode, setTokenFormMode] = useState<'create' | 'edit'>('create');
  const [tokenFormOpen, setTokenFormOpen] = useState(false);
  const [editingToken, setEditingToken] = useState<RemoteToken | null>(null);
  const [importDialogOpen, setImportDialogOpen] = useState(false);
  const [selectedToken, setSelectedToken] = useState<RemoteToken | null>(null);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [deletingToken, setDeletingToken] = useState<RemoteToken | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [viewMode, setViewMode] = useState<ViewMode>('list');
  const [page, setPage] = useState(1);
  const [total, setTotal] = useState(0);

  const totalPages = Math.ceil(total / PAGE_SIZE);

  /**
   * 加载令牌列表
   */
  const loadTokens = useCallback(
    async (targetPage: number = page) => {
      setLoading(true);
      setError(null);
      try {
        const result = await fetchProviderTokens(provider, targetPage, PAGE_SIZE);
        setTokens(result.items);
        setTotal(result.total);
        setPage(result.page);
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        setError(errorMsg);
        toast({
          title: '加载失败',
          description: errorMsg,
          variant: 'destructive',
        });
      } finally {
        setLoading(false);
      }
    },
    [provider, page, toast],
  );

  const handlePageChange = (newPage: number) => {
    if (newPage >= 1 && newPage <= totalPages) {
      loadTokens(newPage);
    }
  };

  /**
   * 打开删除确认对话框
   */
  const handleDelete = (token: RemoteToken) => {
    setDeletingToken(token);
    setDeleteDialogOpen(true);
  };

  /**
   * 确认删除令牌
   */
  const confirmDelete = async () => {
    if (!deletingToken) return;

    setDeleting(true);
    try {
      await deleteProviderToken(provider, deletingToken.id);
      toast({
        title: '令牌已删除',
        description: `令牌「${deletingToken.name}」已成功删除`,
      });
      setDeleteDialogOpen(false);
      setDeletingToken(null);
      if (tokens.length === 1 && page > 1) {
        await loadTokens(page - 1);
      } else {
        await loadTokens();
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      toast({
        title: '删除失败',
        description: errorMsg,
        variant: 'destructive',
      });
    } finally {
      setDeleting(false);
    }
  };

  /**
   * 打开创建对话框
   */
  const handleCreate = () => {
    setTokenFormMode('create');
    setEditingToken(null);
    setTokenFormOpen(true);
  };

  /**
   * 打开编辑对话框
   */
  const handleEdit = (token: RemoteToken) => {
    setTokenFormMode('edit');
    setEditingToken(token);
    setTokenFormOpen(true);
  };

  /**
   * 打开导入对话框
   */
  const handleImport = (token: RemoteToken) => {
    setSelectedToken(token);
    setImportDialogOpen(true);
  };

  /**
   * 格式化时间戳
   */
  const formatTimestamp = (timestamp: number) => {
    if (timestamp === -1 || timestamp === 0) return '永不过期';
    return new Date(timestamp * 1000).toLocaleString('zh-CN');
  };

  /**
   * 格式化额度（剩余/总额）
   */
  const formatQuota = (remainQuota: number, usedQuota: number, unlimited: boolean) => {
    if (unlimited) return '无限';
    const remain = (remainQuota / 500000).toFixed(2);
    const total = ((remainQuota + usedQuota) / 500000).toFixed(2);
    return `$${remain} / $${total}`;
  };

  /**
   * 组件加载时获取令牌列表
   */
  useEffect(() => {
    loadTokens(1);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [provider]);

  return (
    <div className="space-y-4">
      {/* 操作栏 */}
      <div className="flex items-center justify-between">
        <h4 className="text-sm font-medium">远程令牌</h4>
        <div className="flex items-center gap-2">
          <ViewToggle mode={viewMode} onChange={setViewMode} />
          <div className="h-6 w-px bg-border mx-1" />
          <Button size="sm" variant="outline" onClick={() => loadTokens()} disabled={loading}>
            <RefreshCw className={`mr-2 h-3 w-3 ${loading ? 'animate-spin' : ''}`} />
            刷新
          </Button>
          <Button size="sm" onClick={handleCreate}>
            <Plus className="mr-2 h-3 w-3" />
            创建令牌
          </Button>
        </div>
      </div>

      {/* 错误提示 */}
      {error && (
        <div className="rounded-md border border-destructive bg-destructive/10 p-3">
          <p className="text-sm text-destructive">{error}</p>
        </div>
      )}

      {/* 加载状态 */}
      {loading ? (
        <div className="flex items-center justify-center py-6">
          <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
          <span className="ml-2 text-sm text-muted-foreground">加载中...</span>
        </div>
      ) : tokens.length === 0 ? (
        <div className="text-center py-6 text-muted-foreground">
          <p className="text-sm">暂无令牌，请点击「创建令牌」按钮添加</p>
        </div>
      ) : viewMode === 'grid' ? (
        <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
          {tokens.map((token) => (
            <TokenCard
              key={token.id}
              token={token}
              onEdit={handleEdit}
              onImport={handleImport}
              onDelete={handleDelete}
            />
          ))}
        </div>
      ) : (
        <div className="space-y-4">
          <div className="rounded-md border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>名称</TableHead>
                  <TableHead>分组</TableHead>
                  <TableHead>额度</TableHead>
                  <TableHead>状态</TableHead>
                  <TableHead>过期时间</TableHead>
                  <TableHead className="text-right">操作</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {tokens.map((token) => (
                  <TableRow key={token.id}>
                    {/* 名称 */}
                    <TableCell className="font-medium">{token.name}</TableCell>

                    {/* 分组 */}
                    <TableCell className="text-sm">{token.group}</TableCell>

                    {/* 额度 */}
                    <TableCell className="text-sm">
                      {formatQuota(token.remain_quota, token.used_quota, token.unlimited_quota)}
                    </TableCell>

                    {/* 状态 */}
                    <TableCell>
                      <Badge variant={TOKEN_STATUS_VARIANT[token.status as TokenStatus]}>
                        {TOKEN_STATUS_TEXT[token.status as TokenStatus]}
                      </Badge>
                    </TableCell>

                    {/* 过期时间 */}
                    <TableCell className="text-sm text-muted-foreground">
                      {formatTimestamp(token.expired_time)}
                    </TableCell>

                    {/* 操作 */}
                    <TableCell className="text-right">
                      <div className="flex items-center justify-end gap-2">
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={() => handleEdit(token)}
                          title="编辑令牌"
                        >
                          <Pencil className="h-3 w-3" />
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={() => handleImport(token)}
                          title="导入为 Profile"
                        >
                          <Download className="h-3 w-3" />
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={() => handleDelete(token)}
                          title="删除令牌"
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

          {/* 分页控件 */}
          {totalPages > 1 && (
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">
                共 {total} 条记录，第 {page} / {totalPages} 页
              </span>
              <div className="flex items-center gap-2">
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => handlePageChange(page - 1)}
                  disabled={page <= 1 || loading}
                >
                  <ChevronLeft className="h-4 w-4" />
                  上一页
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => handlePageChange(page + 1)}
                  disabled={page >= totalPages || loading}
                >
                  下一页
                  <ChevronRight className="h-4 w-4" />
                </Button>
              </div>
            </div>
          )}
        </div>
      )}

      {/* 令牌表单对话框（创建/编辑） */}
      <TokenFormDialog
        mode={tokenFormMode}
        provider={provider}
        open={tokenFormOpen}
        onOpenChange={setTokenFormOpen}
        token={editingToken}
        onSuccess={loadTokens}
      />

      {/* 导入令牌对话框 */}
      {selectedToken && (
        <ImportTokenDialog
          provider={provider}
          token={selectedToken}
          open={importDialogOpen}
          onOpenChange={setImportDialogOpen}
          onSuccess={() => {
            setImportDialogOpen(false);
            setSelectedToken(null);
          }}
        />
      )}

      {/* 删除确认对话框 */}
      <AlertDialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>确认删除令牌</AlertDialogTitle>
            <AlertDialogDescription>
              您确定要删除令牌 <strong>{deletingToken?.name}</strong> 吗？
              <br />
              此操作无法撤销，该令牌将被永久删除。
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={deleting}>取消</AlertDialogCancel>
            <AlertDialogAction
              onClick={confirmDelete}
              disabled={deleting}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {deleting ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  删除中...
                </>
              ) : (
                '确认删除'
              )}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}