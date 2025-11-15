import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Loader2, Save } from 'lucide-react';
import { PageContainer } from '@/components/layout/PageContainer';
import { useToast } from '@/hooks/use-toast';
import { useSettingsForm } from './hooks/useSettingsForm';
import { useTransparentProxy } from './hooks/useTransparentProxy';
import { BasicSettingsTab } from './components/BasicSettingsTab';
import { ProxySettingsTab } from './components/ProxySettingsTab';
import { ExperimentalSettingsTab } from './components/ExperimentalSettingsTab';
import { UpdateTab } from './components/UpdateTab';
import type { GlobalConfig, UpdateInfo } from '@/lib/tauri-commands';

interface SettingsPageProps {
  globalConfig: GlobalConfig | null;
  configLoading: boolean;
  onConfigChange: () => void;
  updateInfo?: UpdateInfo | null;
  onUpdateCheck?: () => void;
}

export function SettingsPage({
  globalConfig,
  onConfigChange,
  updateInfo,
  onUpdateCheck,
}: SettingsPageProps) {
  const { toast } = useToast();
  const [activeTab, setActiveTab] = useState('basic');

  // 使用自定义 Hooks
  const {
    userId,
    setUserId,
    systemToken,
    setSystemToken,
    proxyEnabled,
    setProxyEnabled,
    proxyType,
    setProxyType,
    proxyHost,
    setProxyHost,
    proxyPort,
    setProxyPort,
    proxyUsername,
    setProxyUsername,
    proxyPassword,
    setProxyPassword,
    proxyTestUrl,
    setProxyTestUrl,
    proxyBypassUrls,
    setProxyBypassUrls,
    transparentProxyEnabled,
    setTransparentProxyEnabled,
    transparentProxyPort,
    setTransparentProxyPort,
    transparentProxyApiKey,
    setTransparentProxyApiKey,
    transparentProxyAllowPublic,
    setTransparentProxyAllowPublic,
    savingSettings,
    testingProxy,
    saveSettings,
    generateProxyKey,
    testProxy,
  } = useSettingsForm({ initialConfig: globalConfig, onConfigChange });

  const {
    transparentProxyStatus,
    startingProxy,
    stoppingProxy,
    loadTransparentProxyStatus,
    handleStartProxy,
    handleStopProxy,
  } = useTransparentProxy();

  // 初始加载透明代理状态
  useEffect(() => {
    loadTransparentProxyStatus().catch((error) => {
      console.error('Failed to load transparent proxy status:', error);
    });
  }, [loadTransparentProxyStatus]);

  // 监听来自App组件的导航到更新tab的事件
  useEffect(() => {
    const handleNavigateToUpdateTab = () => {
      setActiveTab('update');
    };

    window.addEventListener('navigate-to-update-tab', handleNavigateToUpdateTab);

    return () => {
      window.removeEventListener('navigate-to-update-tab', handleNavigateToUpdateTab);
    };
  }, []);

  // 测试代理连接
  const handleTestProxy = async () => {
    const result = await testProxy();

    if (result.success) {
      toast({
        title: result.message,
        description: result.details,
      });
    } else {
      toast({
        title: result.message,
        description: result.details,
        variant: 'destructive',
      });
    }
  };

  // 保存设置
  const handleSaveSettings = async () => {
    try {
      await saveSettings();
      toast({
        title: '保存成功',
        description: '全局设置已保存',
      });
    } catch (error) {
      console.error('Failed to save settings:', error);
      toast({
        title: '保存失败',
        description: String(error),
        variant: 'destructive',
      });
    }
  };

  // 启动透明代理
  const handleStartTransparentProxy = async () => {
    try {
      const result = await handleStartProxy();
      toast({
        title: '启动成功',
        description: result,
      });
    } catch (error) {
      console.error('Failed to start transparent proxy:', error);
      toast({
        title: '启动失败',
        description: String(error),
        variant: 'destructive',
      });
    }
  };

  // 停止透明代理
  const handleStopTransparentProxy = async () => {
    try {
      const result = await handleStopProxy();
      toast({
        title: '停止成功',
        description: result,
      });
    } catch (error) {
      console.error('Failed to stop transparent proxy:', error);
      toast({
        title: '停止失败',
        description: String(error),
        variant: 'destructive',
      });
    }
  };

  return (
    <PageContainer>
      <div className="mb-6">
        <h2 className="text-2xl font-semibold mb-1">全局设置</h2>
        <p className="text-sm text-muted-foreground">配置 DuckCoding 的全局参数和功能</p>
      </div>

      <Tabs value={activeTab} onValueChange={setActiveTab} className="space-y-6">
        <TabsList>
          <TabsTrigger value="basic">基本设置</TabsTrigger>
          <TabsTrigger value="proxy">代理设置</TabsTrigger>
          <TabsTrigger value="experimental">实验性功能</TabsTrigger>
          <TabsTrigger value="update">更新管理</TabsTrigger>
        </TabsList>

        {/* 基本设置 */}
        <TabsContent value="basic" className="space-y-6">
          <BasicSettingsTab
            userId={userId}
            setUserId={setUserId}
            systemToken={systemToken}
            setSystemToken={setSystemToken}
          />
        </TabsContent>

        {/* 代理设置 */}
        <TabsContent value="proxy" className="space-y-6">
          <ProxySettingsTab
            proxyEnabled={proxyEnabled}
            setProxyEnabled={setProxyEnabled}
            proxyType={proxyType}
            setProxyType={setProxyType}
            proxyHost={proxyHost}
            setProxyHost={setProxyHost}
            proxyPort={proxyPort}
            setProxyPort={setProxyPort}
            proxyUsername={proxyUsername}
            setProxyUsername={setProxyUsername}
            proxyPassword={proxyPassword}
            setProxyPassword={setProxyPassword}
            proxyTestUrl={proxyTestUrl}
            setProxyTestUrl={setProxyTestUrl}
            testingProxy={testingProxy}
            onTestProxy={handleTestProxy}
            proxyBypassUrls={proxyBypassUrls}
            setProxyBypassUrls={setProxyBypassUrls}
          />
        </TabsContent>

        {/* 实验性功能 */}
        <TabsContent value="experimental" className="space-y-6">
          <ExperimentalSettingsTab
            transparentProxyEnabled={transparentProxyEnabled}
            setTransparentProxyEnabled={setTransparentProxyEnabled}
            transparentProxyPort={transparentProxyPort}
            setTransparentProxyPort={setTransparentProxyPort}
            transparentProxyApiKey={transparentProxyApiKey}
            setTransparentProxyApiKey={setTransparentProxyApiKey}
            transparentProxyAllowPublic={transparentProxyAllowPublic}
            setTransparentProxyAllowPublic={setTransparentProxyAllowPublic}
            transparentProxyStatus={transparentProxyStatus}
            startingProxy={startingProxy}
            stoppingProxy={stoppingProxy}
            onGenerateProxyKey={generateProxyKey}
            onStartProxy={handleStartTransparentProxy}
            onStopProxy={handleStopTransparentProxy}
          />
        </TabsContent>

        {/* 更新管理 */}
        <TabsContent value="update" className="space-y-6">
          <UpdateTab updateInfo={updateInfo} onUpdateCheck={onUpdateCheck} />
        </TabsContent>
      </Tabs>

      {/* 保存按钮 - 仅在非更新管理标签页时显示 */}
      {activeTab !== 'update' && (
        <div className="flex justify-end mt-6">
          <Button
            onClick={handleSaveSettings}
            disabled={savingSettings}
            className="shadow-md hover:shadow-lg transition-all"
          >
            {savingSettings ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                保存中...
              </>
            ) : (
              <>
                <Save className="mr-2 h-4 w-4" />
                保存设置
              </>
            )}
          </Button>
        </div>
      )}
    </PageContainer>
  );
}
