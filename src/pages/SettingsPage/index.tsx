import { useEffect } from 'react';
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

export function SettingsPage() {
  const { toast } = useToast();

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
    transparentProxyEnabled,
    setTransparentProxyEnabled,
    transparentProxyPort,
    setTransparentProxyPort,
    transparentProxyApiKey,
    setTransparentProxyApiKey,
    transparentProxyAllowPublic,
    setTransparentProxyAllowPublic,
    globalConfig,
    savingSettings,
    testingProxy,
    loadGlobalConfig,
    saveSettings,
    generateProxyKey,
    testProxy,
  } = useSettingsForm();

  const {
    transparentProxyStatus,
    startingProxy,
    stoppingProxy,
    loadTransparentProxyStatus,
    handleStartProxy,
    handleStopProxy,
  } = useTransparentProxy();

  // 初始加载
  useEffect(() => {
    loadGlobalConfig().catch((error) => {
      toast({
        title: '加载失败',
        description: String(error),
        variant: 'destructive',
      });
    });
    loadTransparentProxyStatus().catch((error) => {
      console.error('Failed to load transparent proxy status:', error);
    });
  }, [loadGlobalConfig, loadTransparentProxyStatus, toast]);

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

      <Tabs defaultValue="basic" className="space-y-6">
        <TabsList>
          <TabsTrigger value="basic">基本设置</TabsTrigger>
          <TabsTrigger value="proxy">代理设置</TabsTrigger>
          <TabsTrigger value="experimental">实验性功能</TabsTrigger>
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
      </Tabs>

      {/* 保存按钮 */}
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
    </PageContainer>
  );
}
