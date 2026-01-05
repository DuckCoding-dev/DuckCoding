/**
 * 从供应商导入 Profile 对话框（完全重写版本）
 *
 * 支持两种导入方式：
 * - Tab A：选择现有令牌并导入
 * - Tab B：创建新令牌并直接导入
 */

import { useState, useEffect, forwardRef, useImperativeHandle } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Button } from '@/components/ui/button';
import { DialogFooter } from '@/components/ui/dialog';
import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';
import { Checkbox } from '@/components/ui/checkbox';
import { Loader2, Download } from 'lucide-react';
import { useToast } from '@/hooks/use-toast';
import type { ToolId } from '@/types/profile';
import type { Provider } from '@/types/provider';
import type { RemoteToken, RemoteTokenGroup, CreateRemoteTokenRequest } from '@/types/remote-token';
import { listProviders } from '@/lib/tauri-commands/provider';
import {
  fetchProviderTokens,
  fetchProviderGroups,
  importTokenAsProfile,
  createProviderToken,
} from '@/lib/tauri-commands/token';
import { pmListToolProfiles } from '@/lib/tauri-commands/profile';
import { generateApiKeyForTool, getGlobalConfig } from '@/lib/tauri-commands';
import { DuckCodingGroupHint } from './DuckCodingGroupHint';
import { TokenDetailCard } from './TokenDetailCard';
import { ProfileNameInput } from './ProfileNameInput';

interface ImportFromProviderDialogProps {
  /** 对话框打开状态 */
  open: boolean;
  /** 对话框状态变更回调 */
  onOpenChange: (open: boolean) => void;
  /** 当前工具 ID */
  toolId: ToolId;
  /** 导入成功回调 */
  onSuccess: () => void;
  /** 自动触发一键生成（从手动创建跳转时） */
  autoTriggerGenerate?: boolean;
}

export interface ImportFromProviderDialogRef {
  triggerGenerate: () => void;
}

/**
 * 从供应商导入 Profile 对话框
 */
export const ImportFromProviderDialog = forwardRef<
  ImportFromProviderDialogRef,
  ImportFromProviderDialogProps
>(({ open, onOpenChange, toolId, onSuccess, autoTriggerGenerate }, ref) => {
  const { toast } = useToast();

  // ==================== 数据状态 ====================
  const [providers, setProviders] = useState<Provider[]>([]);
  const [tokens, setTokens] = useState<RemoteToken[]>([]);
  const [tokenGroups, setTokenGroups] = useState<RemoteTokenGroup[]>([]);

  // ==================== 选择状态 ====================
  const [providerId, setProviderId] = useState<string>('');
  const [tokenId, setTokenId] = useState<number | null>(null);
  const [activeTab, setActiveTab] = useState<'select' | 'create'>('select');

  // ==================== Tab B 表单状态 ====================
  const [newTokenName, setNewTokenName] = useState('');
  const [groupId, setGroupId] = useState('');
  const [quota, setQuota] = useState(-1);
  const [expireDays, setExpireDays] = useState(0);
  const [unlimitedQuota, setUnlimitedQuota] = useState(true);
  const [unlimitedExpire, setUnlimitedExpire] = useState(true);

  // ==================== 共享状态 ====================
  const [profileName, setProfileName] = useState('');

  // ==================== 加载状态 ====================
  const [loadingProviders, setLoadingProviders] = useState(false);
  const [loadingTokens, setLoadingTokens] = useState(false);
  const [loadingGroups, setLoadingGroups] = useState(false);
  const [generatingKey, setGeneratingKey] = useState(false);
  const [importing, setImporting] = useState(false);
  const [creating, setCreating] = useState(false);

  // 获取当前选中的供应商和令牌
  const selectedProvider = providers.find((p) => p.id === providerId);
  const selectedToken = tokens.find((t) => t.id === tokenId);

  /**
   * 加载供应商列表
   */
  const loadProviders = async () => {
    try {
      setLoadingProviders(true);
      const result = await listProviders();
      setProviders(result);

      // 默认选中 duckcoding 供应商
      const duckcodingProvider = result.find((p) => p.id === 'duckcoding');
      if (duckcodingProvider) {
        setProviderId('duckcoding');
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      toast({
        title: '加载供应商失败',
        description: errorMsg,
        variant: 'destructive',
      });
    } finally {
      setLoadingProviders(false);
    }
  };

  /**
   * 加载令牌列表
   */
  const loadTokens = async (provider: Provider) => {
    try {
      setLoadingTokens(true);
      const result = await fetchProviderTokens(provider);
      // 自动为没有 sk- 前缀的令牌添加前缀
      const normalizedTokens = result.map((token) => ({
        ...token,
        key: token.key.startsWith('sk-') ? token.key : `sk-${token.key}`,
      }));
      setTokens(normalizedTokens);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      toast({
        title: '加载令牌失败',
        description: errorMsg,
        variant: 'destructive',
      });
      setTokens([]);
    } finally {
      setLoadingTokens(false);
    }
  };

  /**
   * 加载分组列表（Tab B 使用）
   */
  const loadGroups = async (provider: Provider) => {
    try {
      setLoadingGroups(true);
      const result = await fetchProviderGroups(provider);
      setTokenGroups(result);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      toast({
        title: '加载分组失败',
        description: errorMsg,
        variant: 'destructive',
      });
      setTokenGroups([]);
    } finally {
      setLoadingGroups(false);
    }
  };

  /**
   * Dialog 打开时初始化
   */
  useEffect(() => {
    if (open) {
      // 重置所有状态
      setProviderId('');
      setTokenId(null);
      setProfileName('');
      setTokens([]);
      setTokenGroups([]);
      setActiveTab('select');
      setNewTokenName('');
      setGroupId('');
      setQuota(-1);
      setExpireDays(0);
      setUnlimitedQuota(true);
      setUnlimitedExpire(true);

      // 加载供应商列表
      loadProviders();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  /**
   * 供应商变更时加载令牌和分组
   */
  useEffect(() => {
    if (selectedProvider) {
      loadTokens(selectedProvider);
      loadGroups(selectedProvider);
      setTokenId(null);
    } else {
      setTokens([]);
      setTokenGroups([]);
      setTokenId(null);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [providerId]);

  /**
   * 令牌变更时自动填充 Profile 名称
   */
  useEffect(() => {
    if (selectedToken && !profileName) {
      setProfileName(selectedToken.name + '_profile');
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tokenId]);

  /**
   * 暴露给父组件的方法
   */
  useImperativeHandle(ref, () => ({
    triggerGenerate: handleGenerateApiKey,
  }));

  /**
   * 自动触发一键生成（从手动创建跳转时）
   */
  useEffect(() => {
    if (open && autoTriggerGenerate && selectedProvider?.id === 'duckcoding') {
      // 延迟执行，确保对话框已完全渲染
      const timer = setTimeout(() => {
        handleGenerateApiKey();
      }, 300);
      return () => clearTimeout(timer);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, autoTriggerGenerate]);

  /**
   * 一键生成 API Key
   */
  const handleGenerateApiKey = async () => {
    if (!selectedProvider) return;

    try {
      setGeneratingKey(true);

      // 检查全局配置
      const config = await getGlobalConfig();
      if (!config?.user_id || !config?.system_token) {
        toast({
          title: '缺少配置',
          description: '请先在设置中配置用户 ID 和系统访问令牌',
          variant: 'destructive',
        });
        window.dispatchEvent(new CustomEvent('navigate-to-settings'));
        return;
      }

      // 生成 API Key
      const result = await generateApiKeyForTool(toolId);

      if (result.success && result.api_key) {
        toast({
          title: '生成成功',
          description: 'API Key 已自动创建，正在刷新令牌列表...',
        });

        // 重新加载令牌列表并获取最新数据
        const updatedTokens = await fetchProviderTokens(selectedProvider);
        // 自动为没有 sk- 前缀的令牌添加前缀
        const normalizedTokens = updatedTokens.map((token) => ({
          ...token,
          key: token.key.startsWith('sk-') ? token.key : `sk-${token.key}`,
        }));
        setTokens(normalizedTokens);

        // 自动选中新生成的令牌（根据返回的 API Key 匹配）
        // 注意：返回的 api_key 可能没有 sk- 前缀，需要标准化后再匹配
        const normalizedApiKey = result.api_key.startsWith('sk-')
          ? result.api_key
          : `sk-${result.api_key}`;
        if (normalizedTokens.length > 0) {
          const newToken = normalizedTokens.find((t) => t.key === normalizedApiKey);
          if (newToken) {
            setTokenId(newToken.id);
          } else {
            // 回退：选择列表中的第一个
            setTokenId(normalizedTokens[0].id);
          }
        }
      } else {
        toast({
          title: '生成失败',
          description: result.message || '未知错误',
          variant: 'destructive',
        });
      }
    } catch (error) {
      toast({
        title: '生成失败',
        description: String(error),
        variant: 'destructive',
      });
    } finally {
      setGeneratingKey(false);
    }
  };

  /**
   * Tab A 导入逻辑
   */
  const handleImportFromSelect = async () => {
    if (!selectedProvider || !selectedToken) {
      toast({
        title: '请选择供应商和令牌',
        variant: 'destructive',
      });
      return;
    }

    if (!profileName.trim()) {
      toast({
        title: '请输入 Profile 名称',
        variant: 'destructive',
      });
      return;
    }

    // 检查保留前缀
    if (profileName.startsWith('dc_proxy_')) {
      toast({
        title: '验证失败',
        description: 'Profile 名称不能以 dc_proxy_ 开头（系统保留）',
        variant: 'destructive',
      });
      return;
    }

    setImporting(true);
    try {
      // 检查是否已存在同名 Profile
      const existingProfiles = await pmListToolProfiles(toolId);
      if (existingProfiles.includes(profileName)) {
        toast({
          title: '验证失败',
          description: '该 Profile 名称已存在，请使用其他名称',
          variant: 'destructive',
        });
        setImporting(false);
        return;
      }

      await importTokenAsProfile(selectedProvider, selectedToken, toolId, profileName);
      toast({
        title: '导入成功',
        description: `令牌「${selectedToken.name}」已成功导入为 Profile「${profileName}」`,
      });
      onSuccess();
      onOpenChange(false);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      toast({
        title: '导入失败',
        description: errorMsg,
        variant: 'destructive',
      });
    } finally {
      setImporting(false);
    }
  };

  /**
   * Tab B 令牌名称变更时自动填充 Profile 名称
   */
  useEffect(() => {
    if (activeTab === 'create' && newTokenName && !profileName) {
      setProfileName(newTokenName + '_profile');
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [newTokenName, activeTab]);

  /**
   * Tab B 创建并导入逻辑
   */
  const handleCreateAndImport = async () => {
    if (!selectedProvider) {
      toast({
        title: '请选择供应商',
        variant: 'destructive',
      });
      return;
    }

    // 验证必填项
    if (!newTokenName.trim()) {
      toast({
        title: '请输入令牌名称',
        variant: 'destructive',
      });
      return;
    }

    if (!groupId) {
      toast({
        title: '请选择分组',
        variant: 'destructive',
      });
      return;
    }

    if (!profileName.trim()) {
      toast({
        title: '请输入 Profile 名称',
        variant: 'destructive',
      });
      return;
    }

    // 检查保留前缀
    if (profileName.startsWith('dc_proxy_')) {
      toast({
        title: '验证失败',
        description: 'Profile 名称不能以 dc_proxy_ 开头（系统保留）',
        variant: 'destructive',
      });
      return;
    }

    setCreating(true);
    try {
      // 检查是否已存在同名 Profile
      const existingProfiles = await pmListToolProfiles(toolId);
      if (existingProfiles.includes(profileName)) {
        toast({
          title: '验证失败',
          description: '该 Profile 名称已存在，请使用其他名称',
          variant: 'destructive',
        });
        setCreating(false);
        return;
      }

      // 计算过期时间（Unix 时间戳）
      const expiredTime = unlimitedExpire
        ? -1 // -1 表示永不过期
        : Math.floor(Date.now() / 1000) + expireDays * 24 * 60 * 60;

      // 计算额度（token）
      const remainQuota = unlimitedQuota ? 500000 : quota * 500000;

      // 构建创建请求（所有字段都是必需的）
      const request: CreateRemoteTokenRequest = {
        name: newTokenName,
        group: groupId,
        remain_quota: remainQuota,
        unlimited_quota: unlimitedQuota,
        expired_time: expiredTime,
        model_limits_enabled: false,
        model_limits: '',
        allow_ips: '',
      };

      // 调用创建令牌 API（返回 void）
      await createProviderToken(selectedProvider, request);

      toast({
        title: '创建成功',
        description: `令牌「${newTokenName}」已成功创建，正在获取令牌详情...`,
      });

      // 等待 500ms 确保服务器处理完成
      await new Promise((resolve) => setTimeout(resolve, 500));

      // 重新获取令牌列表
      const updatedTokens = await fetchProviderTokens(selectedProvider);
      // 自动为没有 sk- 前缀的令牌添加前缀
      const normalizedTokens = updatedTokens.map((token) => ({
        ...token,
        key: token.key.startsWith('sk-') ? token.key : `sk-${token.key}`,
      }));

      // 按 ID 降序排序，找到名称匹配的第一个（最新创建的）
      const sortedTokens = normalizedTokens
        .filter((t) => t.name === newTokenName)
        .sort((a, b) => b.id - a.id);

      if (sortedTokens.length === 0) {
        toast({
          title: '查找失败',
          description: `无法找到刚创建的令牌「${newTokenName}」，请手动刷新列表`,
          variant: 'destructive',
        });
        setCreating(false);
        return;
      }

      const newToken = sortedTokens[0];

      // 直接导入为 Profile
      await importTokenAsProfile(selectedProvider, newToken, toolId, profileName);

      toast({
        title: '导入成功',
        description: `令牌「${newToken.name}」已成功导入为 Profile「${profileName}」`,
      });

      onSuccess();
      onOpenChange(false);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      toast({
        title: '创建失败',
        description: errorMsg,
        variant: 'destructive',
      });
    } finally {
      setCreating(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[600px] max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>从供应商导入 Profile</DialogTitle>
          <DialogDescription>
            选择供应商和令牌，或创建新令牌并一键导入为本地 Profile 配置
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* 供应商选择器 */}
          <div className="space-y-2">
            <Label htmlFor="provider-select">选择供应商 *</Label>
            <Select value={providerId} onValueChange={setProviderId}>
              <SelectTrigger id="provider-select">
                <SelectValue placeholder="请选择供应商" />
              </SelectTrigger>
              <SelectContent>
                {loadingProviders ? (
                  <div className="p-2 text-sm text-muted-foreground text-center flex items-center justify-center gap-2">
                    <Loader2 className="h-4 w-4 animate-spin" />
                    加载中...
                  </div>
                ) : providers.length === 0 ? (
                  <div className="p-2 text-sm text-muted-foreground text-center">
                    暂无可用供应商
                  </div>
                ) : (
                  providers.map((provider) => (
                    <SelectItem key={provider.id} value={provider.id}>
                      {provider.name}
                    </SelectItem>
                  ))
                )}
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">选择要从哪个供应商导入令牌</p>
          </div>

          {/* DuckCoding 分组说明（独立于 Tabs） */}
          {selectedProvider?.id === 'duckcoding' && (
            <DuckCodingGroupHint
              toolId={toolId}
              onGenerateClick={handleGenerateApiKey}
              generating={generatingKey}
            />
          )}

          {/* Tabs 切换 */}
          <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as 'select' | 'create')}>
            <TabsList className="grid w-full grid-cols-2">
              <TabsTrigger value="select">选择令牌</TabsTrigger>
              <TabsTrigger value="create">创建令牌</TabsTrigger>
            </TabsList>

            {/* Tab A: 选择令牌 */}
            <TabsContent value="select" className="space-y-4 mt-4">
              {/* 令牌选择器 */}
              <div className="space-y-2">
                <Label htmlFor="token-select">选择令牌 *</Label>
                <Select
                  value={tokenId?.toString() || ''}
                  onValueChange={(v) => setTokenId(Number(v))}
                  disabled={!selectedProvider || loadingTokens}
                >
                  <SelectTrigger id="token-select">
                    <SelectValue
                      placeholder={
                        !selectedProvider
                          ? '请先选择供应商'
                          : loadingTokens
                            ? '加载中...'
                            : '请选择令牌'
                      }
                    />
                  </SelectTrigger>
                  <SelectContent>
                    {loadingTokens ? (
                      <div className="p-2 text-sm text-muted-foreground text-center flex items-center justify-center gap-2">
                        <Loader2 className="h-4 w-4 animate-spin" />
                        加载中...
                      </div>
                    ) : tokens.length === 0 ? (
                      <div className="p-2 text-sm text-muted-foreground text-center">
                        该供应商暂无可用令牌
                      </div>
                    ) : (
                      tokens.map((token) => (
                        <SelectItem key={token.id} value={token.id.toString()}>
                          <div className="flex items-center justify-between gap-4 w-full">
                            <span>{token.name}</span>
                            <span className="text-xs text-muted-foreground">
                              {token.unlimited_quota
                                ? '无限'
                                : `$${(token.remain_quota / 1000000).toFixed(2)}`}
                            </span>
                          </div>
                        </SelectItem>
                      ))
                    )}
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">选择要导入的令牌</p>
              </div>

              {/* 令牌详情卡片 */}
              {selectedToken && (
                <TokenDetailCard
                  token={selectedToken}
                  group={tokenGroups.find((g) => g.id === selectedToken.group)}
                />
              )}

              {/* Profile 名称输入 */}
              <ProfileNameInput
                value={profileName}
                onChange={setProfileName}
                placeholder="例如: my_token_profile"
              />

              {/* 导入按钮 */}
              <DialogFooter>
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => onOpenChange(false)}
                  disabled={importing}
                >
                  取消
                </Button>
                <Button
                  onClick={handleImportFromSelect}
                  disabled={importing || !selectedProvider || !selectedToken}
                >
                  {importing && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                  {!importing && <Download className="mr-2 h-4 w-4" />}
                  导入
                </Button>
              </DialogFooter>
            </TabsContent>

            {/* Tab B: 创建令牌 */}
            <TabsContent value="create" className="space-y-4 mt-4">
              {/* 令牌名称 */}
              <div className="space-y-2">
                <Label htmlFor="new-token-name">令牌名称 *</Label>
                <Input
                  id="new-token-name"
                  value={newTokenName}
                  onChange={(e) => setNewTokenName(e.target.value)}
                  placeholder="例如: my_api_key"
                  disabled={!selectedProvider}
                />
                <p className="text-xs text-muted-foreground">为新令牌设置一个名称</p>
              </div>

              {/* 分组选择器 */}
              <div className="space-y-2">
                <Label htmlFor="group-select">分组 *</Label>
                <Select
                  value={groupId}
                  onValueChange={setGroupId}
                  disabled={!selectedProvider || loadingGroups}
                >
                  <SelectTrigger id="group-select">
                    <SelectValue
                      placeholder={
                        !selectedProvider
                          ? '请先选择供应商'
                          : loadingGroups
                            ? '加载中...'
                            : '请选择分组'
                      }
                    />
                  </SelectTrigger>
                  <SelectContent>
                    {loadingGroups ? (
                      <div className="p-2 text-sm text-muted-foreground text-center flex items-center justify-center gap-2">
                        <Loader2 className="h-4 w-4 animate-spin" />
                        加载中...
                      </div>
                    ) : tokenGroups.length === 0 ? (
                      <div className="p-2 text-sm text-muted-foreground text-center">
                        该供应商暂无分组
                      </div>
                    ) : (
                      tokenGroups.map((group) => (
                        <SelectItem key={group.id} value={group.id}>
                          <div className="flex items-center justify-between gap-4 w-full">
                            <span>{group.id}</span>
                            <span className="text-xs text-muted-foreground">
                              {group.desc} (倍率: {group.ratio})
                            </span>
                          </div>
                        </SelectItem>
                      ))
                    )}
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">选择令牌所属分组</p>
              </div>

              {/* 额度设置 */}
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <Label htmlFor="quota">限额 (美元)</Label>
                  <div className="flex items-center space-x-2">
                    <Checkbox
                      id="unlimited-quota"
                      checked={unlimitedQuota}
                      onCheckedChange={(checked) => setUnlimitedQuota(checked === true)}
                    />
                    <label
                      htmlFor="unlimited-quota"
                      className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                    >
                      无限额度
                    </label>
                  </div>
                </div>
                <Input
                  id="quota"
                  type="number"
                  value={quota}
                  onChange={(e) => setQuota(Number(e.target.value))}
                  placeholder="例如: 100"
                  disabled={unlimitedQuota}
                />
                <p className="text-xs text-muted-foreground">
                  设置令牌的使用限额
                </p>
              </div>

              {/* 有效期设置 */}
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <Label htmlFor="expire-days">有效期 (天)</Label>
                  <div className="flex items-center space-x-2">
                    <Checkbox
                      id="unlimited-expire"
                      checked={unlimitedExpire}
                      onCheckedChange={(checked) => setUnlimitedExpire(checked === true)}
                    />
                    <label
                      htmlFor="unlimited-expire"
                      className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                    >
                      永不过期
                    </label>
                  </div>
                </div>
                <Input
                  id="expire-days"
                  type="number"
                  value={expireDays}
                  onChange={(e) => setExpireDays(Number(e.target.value))}
                  placeholder="例如: 365"
                  disabled={unlimitedExpire}
                />
                <p className="text-xs text-muted-foreground">设置令牌的有效期（0 表示永不过期）</p>
              </div>

              {/* Profile 名称输入 */}
              <ProfileNameInput
                value={profileName}
                onChange={setProfileName}
                placeholder="例如: my_token_profile"
              />

              {/* 创建并导入按钮 */}
              <DialogFooter>
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => onOpenChange(false)}
                  disabled={creating}
                >
                  取消
                </Button>
                <Button
                  onClick={handleCreateAndImport}
                  disabled={creating || !selectedProvider || !newTokenName || !groupId}
                >
                  {creating && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                  {!creating && <Download className="mr-2 h-4 w-4" />}
                  创建并导入
                </Button>
              </DialogFooter>
            </TabsContent>
          </Tabs>
        </div>
      </DialogContent>
    </Dialog>
  );
});

ImportFromProviderDialog.displayName = 'ImportFromProviderDialog';
