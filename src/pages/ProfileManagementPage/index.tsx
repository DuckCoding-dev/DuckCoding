/**
 * Profile 配置管理页面
 */

import { useState, useEffect, useRef } from 'react';
import { RefreshCw, Loader2, HelpCircle, Plus, Download, ChevronDown } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';

import { PageContainer } from '@/components/layout/PageContainer';
import { ProfileCard } from './components/ProfileCard';
import { ProfileEditor } from './components/ProfileEditor';
import { ActiveProfileCard } from './components/ActiveProfileCard';
import { ImportFromProviderDialog } from './components/ImportFromProviderDialog';
import { CreateCustomProfileDialog } from './components/CreateCustomProfileDialog';
import { AmpProfileSelector } from './components/AmpProfileSelector';
import { HelpDialog } from './components/HelpDialog';
import { useProfileManagement } from './hooks/useProfileManagement';
import { ProfileTable } from './components/ProfileTable';
import { ViewToggle, ViewMode } from '@/components/common/ViewToggle';
import type { ProfileToolId, ProfileFormData, ProfileDescriptor, ToolId } from '@/types/profile';
import { logoMap } from '@/utils/constants';

export default function ProfileManagementPage() {
  const {
    profileGroups,
    allProfiles,
    loading,
    error,
    allProxyStatus,
    refresh,
    loadAllProxyStatus,
    createProfile,
    updateProfile,
    deleteProfile,
    activateProfile,
  } = useProfileManagement();

  const [selectedTab, setSelectedTab] = useState<ToolId>('claude-code');
  const [editorOpen, setEditorOpen] = useState(false);
  const [editorMode, setEditorMode] = useState<'create' | 'edit'>('create');
  const [editingProfile, setEditingProfile] = useState<ProfileDescriptor | null>(null);
  const [helpDialogOpen, setHelpDialogOpen] = useState(false);
  const [importDialogOpen, setImportDialogOpen] = useState(false);
  const [customProfileDialogOpen, setCustomProfileDialogOpen] = useState(false);
  const [autoTriggerGenerate, setAutoTriggerGenerate] = useState(false);
  const [viewMode, setViewMode] = useState<ViewMode>('grid');

  // ImportFromProviderDialog ref 用于触发一键生成
  const importDialogRef = useRef<{ triggerGenerate: () => void } | null>(null);

  // 初始化加载透明代理状态
  useEffect(() => {
    loadAllProxyStatus();
  }, [loadAllProxyStatus]);

  // 打开编辑对话框
  const handleEditProfile = (profile: ProfileDescriptor) => {
    setEditorMode('edit');
    setEditingProfile(profile);
    setEditorOpen(true);
  };

  // 保存 Profile
  const handleSaveProfile = async (data: ProfileFormData) => {
    if (selectedTab === 'amp-code') return; // AMP 不支持创建 profile
    if (editorMode === 'create') {
      await createProfile(selectedTab as ProfileToolId, data);
    } else if (editingProfile) {
      await updateProfile(selectedTab as ProfileToolId, editingProfile.name, data);
    }
    setEditorOpen(false);
    // 对话框关闭后刷新数据
    await refresh();
  };

  // 激活 Profile
  const handleActivateProfile = async (profileName: string) => {
    if (selectedTab === 'amp-code') return; // AMP 不支持激活 profile
    await activateProfile(selectedTab as ProfileToolId, profileName);
  };

  // 删除 Profile
  const handleDeleteProfile = async (profileName: string) => {
    if (selectedTab === 'amp-code') return; // AMP 不支持删除 profile
    await deleteProfile(selectedTab as ProfileToolId, profileName);
  };

  // 构建编辑器初始数据
  const getEditorInitialData = (): ProfileFormData | undefined => {
    if (!editingProfile) return undefined;

    return {
      name: editingProfile.name,
      api_key: '', // 编辑时留空表示不修改
      base_url: editingProfile.base_url,
      wire_api: editingProfile.wire_api || editingProfile.provider, // 兼容两个字段名
      model: editingProfile.model,
      pricing_template_id: editingProfile.pricing_template_id, // Phase 6: 价格模板
    };
  };

  const pageActions = (
    <div className="flex items-center gap-2">
      {selectedTab !== 'amp-code' && (
        <>
          <ViewToggle mode={viewMode} onChange={setViewMode} />
          <div className="h-6 w-px bg-border mx-1" />
        </>
      )}
      <Button onClick={() => setHelpDialogOpen(true)} variant="outline" size="sm">
        <HelpCircle className="mr-2 h-4 w-4" />
        帮助
      </Button>
      <Button onClick={refresh} variant="outline" size="sm" disabled={loading}>
        <RefreshCw className={`mr-2 h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
        刷新
      </Button>
    </div>
  );

  return (
    <PageContainer
      title="配置管理"
      description="管理所有工具的 Profile 配置，快速切换不同的 API 端点"
      actions={pageActions}
    >
      {/* 错误提示 */}
      {error && (
        <div className="rounded-lg border border-destructive bg-destructive/10 p-4">
          <p className="text-sm text-destructive">加载失败: {error}</p>
          <Button onClick={refresh} variant="outline" size="sm" className="mt-2">
            重试
          </Button>
        </div>
      )}

      {/* 加载状态 */}
      {loading && profileGroups.length === 0 ? (
        <div className="flex items-center justify-center py-20">
          <Loader2 className="h-8 w-8 animate-spin text-primary" />
          <span className="ml-3 text-muted-foreground">加载中...</span>
        </div>
      ) : (
        <>
          {/* 工具 Tab 切换 */}
          <Tabs value={selectedTab} onValueChange={(v) => setSelectedTab(v as ToolId)}>
            <TabsList className="grid w-full grid-cols-4 mb-4 h-9 p-1 bg-muted/50 rounded-lg">
              {profileGroups.map((group) => (
                <TabsTrigger
                  key={group.tool_id}
                  value={group.tool_id}
                  className="gap-2 data-[state=active]:bg-background data-[state=active]:shadow-sm transition-all duration-200"
                >
                  <img src={logoMap[group.tool_id]} alt={group.tool_name} className="w-4 h-4" />
                  {group.tool_name}
                </TabsTrigger>
              ))}
              {/* AMP Code Tab */}
              <TabsTrigger
                value="amp-code"
                className="gap-2 data-[state=active]:bg-background data-[state=active]:shadow-sm transition-all duration-200"
              >
                <img src={logoMap['amp-code']} alt="AMP Code" className="w-4 h-4" />
                AMP Code
              </TabsTrigger>
            </TabsList>

            {/* 每个工具的 Profile 列表 */}
            {profileGroups.map((group) => (
              <TabsContent key={group.tool_id} value={group.tool_id} className="space-y-4 mt-0">
                {/* 当前生效配置卡片 */}
                <ActiveProfileCard
                  group={group}
                  proxyRunning={allProxyStatus[group.tool_id]?.running || false}
                />

                <div className="space-y-3">
                  {/* 创建按钮 */}
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="text-sm text-muted-foreground">
                        {group.profiles.length === 0
                          ? '暂无 Profile，点击创建新配置'
                          : `共 ${group.profiles.length} 个配置`}
                        {group.active_profile && ` · 当前激活: ${group.active_profile.name}`}
                      </p>
                    </div>
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button size="sm" disabled={selectedTab !== group.tool_id}>
                          创建 Profile
                          <ChevronDown className="ml-2 h-4 w-4" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem onClick={() => setCustomProfileDialogOpen(true)}>
                          <Plus className="mr-2 h-4 w-4" />
                          手动创建
                        </DropdownMenuItem>
                        <DropdownMenuItem onClick={() => setImportDialogOpen(true)}>
                          <Download className="mr-2 h-4 w-4" />
                          从供应商导入
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </div>

                  {/* Profile 列表 (Grid 或 Table) */}
                  {group.profiles.length === 0 ? (
                    <div className="rounded-lg border border-dashed p-12 text-center bg-muted/20">
                      <p className="text-sm text-muted-foreground">暂无 Profile 配置</p>
                    </div>
                  ) : viewMode === 'grid' ? (
                    <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                      {group.profiles.map((profile) => (
                        <ProfileCard
                          key={profile.name}
                          profile={profile}
                          onActivate={() => handleActivateProfile(profile.name)}
                          onEdit={() => handleEditProfile(profile)}
                          onDelete={() => handleDeleteProfile(profile.name)}
                          proxyRunning={allProxyStatus[group.tool_id]?.running || false}
                        />
                      ))}
                    </div>
                  ) : (
                    <ProfileTable
                      profiles={group.profiles}
                      onActivate={handleActivateProfile}
                      onEdit={handleEditProfile}
                      onDelete={handleDeleteProfile}
                      proxyRunning={allProxyStatus[group.tool_id]?.running || false}
                    />
                  )}
                </div>
              </TabsContent>
            ))}

            {/* AMP Code Tab 内容 */}
            <TabsContent value="amp-code" className="space-y-4 mt-0">
              <AmpProfileSelector
                allProfiles={allProfiles}
                onSwitchTab={(toolId) => setSelectedTab(toolId)}
              />
            </TabsContent>
          </Tabs>
        </>
      )}

      {/* Profile 编辑器对话框（AMP 不需要） */}
      {selectedTab !== 'amp-code' && (
        <ProfileEditor
          open={editorOpen}
          onOpenChange={setEditorOpen}
          toolId={selectedTab as ProfileToolId}
          mode={editorMode}
          initialData={getEditorInitialData()}
          onSave={handleSaveProfile}
        />
      )}

      {/* 帮助弹窗 */}
      <HelpDialog open={helpDialogOpen} onOpenChange={setHelpDialogOpen} />

      {/* 自定义 Profile 创建对话框 */}
      <CreateCustomProfileDialog
        open={customProfileDialogOpen}
        onOpenChange={setCustomProfileDialogOpen}
        toolId={selectedTab}
        onSuccess={() => {
          setCustomProfileDialogOpen(false);
          refresh();
        }}
        onQuickSetup={() => {
          setCustomProfileDialogOpen(false);
          setAutoTriggerGenerate(true);
          setImportDialogOpen(true);
        }}
      />

      {/* 从供应商导入对话框 */}
      <ImportFromProviderDialog
        ref={importDialogRef}
        open={importDialogOpen}
        onOpenChange={(open) => {
          setImportDialogOpen(open);
          if (!open) {
            setAutoTriggerGenerate(false);
          }
        }}
        toolId={selectedTab}
        autoTriggerGenerate={autoTriggerGenerate}
        onSuccess={() => {
          setImportDialogOpen(false);
          setAutoTriggerGenerate(false);
          refresh();
        }}
      />
    </PageContainer>
  );
}
