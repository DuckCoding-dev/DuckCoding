import { PageContainer } from '@/components/layout/PageContainer';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Building2, Plus, Pencil, Trash2, Loader2, Coins, CalendarCheck } from 'lucide-react';
import { useState, useEffect, useCallback } from 'react';
import type { Provider } from '@/lib/tauri-commands';
import { useToast } from '@/hooks/use-toast';
import { useProviderManagement } from './hooks/useProviderManagement';
import { ProviderFormDialog } from './components/ProviderFormDialog';
import { DeleteConfirmDialog } from './components/DeleteConfirmDialog';
import { TokenManagementTab } from './components/TokenManagementTab';
import { ProviderCard } from './components/ProviderCard';
import { ViewToggle, ViewMode } from '@/components/common/ViewToggle';
import { CheckinDialog } from './components/CheckinDialog';
import { hasProviderAuth, getCheckinStatus } from '@/services/checkin';

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
  const [activeTab, setActiveTab] = useState<'providers' | 'tokens'>('providers');
  const [selectedProviderId, setSelectedProviderId] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>('list');
  const [checkinDialogOpen, setCheckinDialogOpen] = useState(false);
  const [checkinProvider, setCheckinProvider] = useState<Provider | null>(null);
  // 记录各供应商签到支持状态: true=支持, false=不支持, undefined=未检测
  const [checkinSupportMap, setCheckinSupportMap] = useState<Record<string, boolean>>({});

  // 页面加载后，对有认证信息的供应商并行检测签到支持
  const checkCheckinSupport = useCallback(async (providerList: Provider[]) => {
    const authProviders = providerList.filter(hasProviderAuth);
    if (authProviders.length === 0) return;

    const results = await Promise.allSettled(
      authProviders.map(async (p) => {
        const result = await getCheckinStatus(p);
        // 只有 API 明确返回 success: true 才视为支持签到
        return { id: p.id, supported: result.success };
      }),
    );

    const map: Record<string, boolean> = {};
    for (const r of results) {
      if (r.status === 'fulfilled') {
        map[r.value.id] = r.value.supported;
      }
    }
    setCheckinSupportMap(map);
  }, []);

  useEffect(() => {
    if (providers.length > 0) {
      checkCheckinSupport(providers);
    }
  }, [providers, checkCheckinSupport]);

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

  /**
   * 查看供应商的令牌（切换到令牌管理 Tab）
   */
  const handleViewTokens = (providerId: string) => {
    setSelectedProviderId(providerId);
    setActiveTab('tokens');
  };

  /**
   * 打开签到对话框
   */
  const handleCheckin = (provider: Provider) => {
    setCheckinProvider(provider);
    setCheckinDialogOpen(true);
  };

  /**
   * 更新签到配置
   */
  const handleCheckinUpdate = async (updatedProvider: Provider) => {
    const result = await updateProvider(updatedProvider.id, updatedProvider);
    if (!result.success) {
      toast({
        title: '更新失败',
        description: result.error,
        variant: 'destructive',
      });
    }
  };

  const pageActions =
    activeTab === 'providers' ? (
      <div className="flex gap-2 items-center">
        <ViewToggle mode={viewMode} onChange={setViewMode} />
        <div className="h-6 w-px bg-border mx-1" />
        <Button onClick={handleAdd} size="sm">
          <Plus className="mr-2 h-4 w-4" />
          新增供应商
        </Button>
      </div>
    ) : null;

  return (
    <PageContainer
      title="供应商管理"
      description="管理所有 AI 服务供应商及其令牌信息"
      actions={pageActions}
    >
      <div className="space-y-4">
        {/* Tabs 组件 */}
        <Tabs
          value={activeTab}
          onValueChange={(value) => setActiveTab(value as 'providers' | 'tokens')}
        >
          <TabsList className="grid w-full grid-cols-2">
            <TabsTrigger value="providers">
              <Building2 className="mr-2 h-4 w-4" />
              供应商列表
            </TabsTrigger>
            <TabsTrigger value="tokens">
              <Coins className="mr-2 h-4 w-4" />
              令牌管理
            </TabsTrigger>
          </TabsList>

          {/* Tab 1: 供应商管理 */}
          <TabsContent value="providers" className="mt-4 space-y-4">
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
            ) : viewMode === 'grid' ? (
              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
                {providers.map((provider) => (
                  <ProviderCard
                    key={provider.id}
                    provider={provider}
                    onEdit={handleEdit}
                    onDelete={(p) => {
                      setDeletingProvider(p);
                      setDeleteDialogOpen(true);
                    }}
                    onViewTokens={handleViewTokens}
                    onCheckin={handleCheckin}
                    checkinSupported={checkinSupportMap[provider.id]}
                  />
                ))}
              </div>
            ) : (
              <div className="rounded-md border max-h-[500px] overflow-auto">
                <Table>
                  <TableHeader className="sticky top-0 bg-background z-10">
                    <TableRow>
                      <TableHead>名称</TableHead>
                      <TableHead>官网地址</TableHead>
                      <TableHead>用户名</TableHead>
                      <TableHead>更新时间</TableHead>
                      <TableHead className="text-right">操作</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {providers.map((provider) => (
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
                            <Button
                              size="sm"
                              variant="outline"
                              onClick={() => handleViewTokens(provider.id)}
                            >
                              <Coins className="mr-2 h-4 w-4" />
                              查看令牌
                            </Button>
                            <Button
                              size="sm"
                              variant="outline"
                              disabled={checkinSupportMap[provider.id] !== true}
                              onClick={() => handleCheckin(provider)}
                            >
                              <CalendarCheck className="mr-2 h-4 w-4" />
                              签到
                            </Button>
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
                    ))}
                  </TableBody>
                </Table>
              </div>
            )}
          </TabsContent>

          {/* Tab 2: 令牌管理 */}
          <TabsContent value="tokens" className="mt-4">
            <TokenManagementTab
              providers={providers}
              selectedProviderId={selectedProviderId}
              onProviderChange={setSelectedProviderId}
            />
          </TabsContent>
        </Tabs>
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

      {/* 签到对话框 */}
      {checkinProvider && (
        <CheckinDialog
          open={checkinDialogOpen}
          onOpenChange={setCheckinDialogOpen}
          provider={checkinProvider}
          onUpdate={handleCheckinUpdate}
        />
      )}
    </PageContainer>
  );
}
