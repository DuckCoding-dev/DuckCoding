import { PageContainer } from '@/components/layout/PageContainer';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Building2, Plus, Pencil, Trash2, Loader2 } from 'lucide-react';
import { useState } from 'react';
import type { Provider } from '@/lib/tauri-commands';
import { useToast } from '@/hooks/use-toast';
import { useProviderManagement } from './hooks/useProviderManagement';
import { ProviderFormDialog } from './components/ProviderFormDialog';
import { DeleteConfirmDialog } from './components/DeleteConfirmDialog';

/**
 * 供应商管理页面
 * 独立的顶级页面,用于管理所有 AI 服务供应商
 */
export function ProviderManagementPage() {
  const { toast } = useToast();
  const { providers, loading, error, createProvider, updateProvider, deleteProvider } =
    useProviderManagement();

  const [formDialogOpen, setFormDialogOpen] = useState(false);
  const [editingProvider, setEditingProvider] = useState<Provider | null>(null);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [deletingProvider, setDeletingProvider] = useState<Provider | null>(null);
  const [deleting, setDeleting] = useState(false);

  /**
   * 打开新增对话框
   */
  const handleAdd = () => {
    setEditingProvider(null);
    setFormDialogOpen(true);
  };

  /**
   * 打开编辑对话框
   */
  const handleEdit = (provider: Provider) => {
    setEditingProvider(provider);
    setFormDialogOpen(true);
  };

  /**
   * 提交表单（创建或更新）
   */
  const handleFormSubmit = async (provider: Provider) => {
    const result = editingProvider
      ? await updateProvider(editingProvider.id, provider)
      : await createProvider(provider);

    if (result.success) {
      toast({
        title: editingProvider ? '供应商已更新' : '供应商已创建',
        description: `供应商「${provider.name}」已成功${editingProvider ? '更新' : '创建'}`,
      });
      setFormDialogOpen(false);
    } else {
      toast({
        title: editingProvider ? '更新失败' : '创建失败',
        description: result.error,
        variant: 'destructive',
      });
    }
  };

  /**
   * 删除供应商
   */
  const handleDelete = async (id: string) => {
    setDeleting(true);
    const result = await deleteProvider(id);

    if (result.success) {
      toast({
        title: '供应商已删除',
        description: '供应商已成功删除',
      });
      setDeleteDialogOpen(false);
      setDeletingProvider(null);
    } else {
      toast({
        title: '删除失败',
        description: result.error,
        variant: 'destructive',
      });
    }
    setDeleting(false);
  };

  /**
   * 格式化时间戳
   */
  const formatTimestamp = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString('zh-CN');
  };

  return (
    <PageContainer>
      <div className="space-y-4 rounded-lg border p-6">
        {/* 顶部标题栏 */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Building2 className="h-5 w-5" />
            <h3 className="text-lg font-semibold">供应商管理</h3>
          </div>
          <Button onClick={handleAdd} size="sm">
            <Plus className="mr-2 h-4 w-4" />
            新增供应商
          </Button>
        </div>
        <Separator />

        {/* 错误提示 */}
        {error && (
          <div className="rounded-md border border-destructive bg-destructive/10 p-4">
            <p className="text-sm text-destructive">加载失败: {error}</p>
          </div>
        )}

        {/* 加载状态 */}
        {loading ? (
          <div className="flex items-center justify-center py-8">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            <span className="ml-2 text-sm text-muted-foreground">加载中...</span>
          </div>
        ) : providers.length === 0 ? (
          <div className="text-center py-8 text-muted-foreground">
            <Building2 className="h-12 w-12 mx-auto mb-2 opacity-20" />
            <p className="text-sm">暂无供应商，请点击「新增供应商」按钮添加</p>
          </div>
        ) : (
          <div className="rounded-md border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>名称</TableHead>
                  <TableHead>官网地址</TableHead>
                  <TableHead>用户名</TableHead>
                  <TableHead>更新时间</TableHead>
                  <TableHead className="text-right">操作</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {providers.map((provider) => {
                  return (
                    <TableRow key={provider.id}>
                      {/* 名称 */}
                      <TableCell className="font-medium">{provider.name}</TableCell>

                      {/* 官网地址 */}
                      <TableCell>
                        <a
                          href={provider.website_url}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-sm text-primary hover:underline"
                        >
                          {provider.website_url}
                        </a>
                      </TableCell>

                      {/* 用户名 */}
                      <TableCell className="text-sm">{provider.username || '-'}</TableCell>

                      {/* 更新时间 */}
                      <TableCell className="text-sm text-muted-foreground">
                        {formatTimestamp(provider.updated_at)}
                      </TableCell>

                      {/* 操作 */}
                      <TableCell className="text-right">
                        <div className="flex items-center justify-end gap-2">
                          <Button size="sm" variant="ghost" onClick={() => handleEdit(provider)}>
                            <Pencil className="h-4 w-4" />
                          </Button>
                          <Button
                            size="sm"
                            variant="ghost"
                            onClick={() => {
                              setDeletingProvider(provider);
                              setDeleteDialogOpen(true);
                            }}
                            disabled={provider.is_default}
                          >
                            <Trash2 className="h-4 w-4" />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          </div>
        )}
      </div>

      {/* 表单对话框 */}
      <ProviderFormDialog
        open={formDialogOpen}
        onOpenChange={setFormDialogOpen}
        provider={editingProvider}
        onSubmit={handleFormSubmit}
        isEditing={!!editingProvider}
      />

      {/* 删除确认对话框 */}
      <DeleteConfirmDialog
        open={deleteDialogOpen}
        onOpenChange={setDeleteDialogOpen}
        providerName={deletingProvider?.name || ''}
        onConfirm={async () => {
          if (deletingProvider) {
            await handleDelete(deletingProvider.id);
          }
        }}
        deleting={deleting}
      />
    </PageContainer>
  );
}
