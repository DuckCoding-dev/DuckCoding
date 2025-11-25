import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Loader2, Save } from 'lucide-react';
import { PageContainer } from '@/components/layout/PageContainer';
import { useToast } from '@/hooks/use-toast';
import { useSettingsForm } from './hooks/useSettingsForm';
import { BasicSettingsTab } from './components/BasicSettingsTab';
import { ProxySettingsTab } from './components/ProxySettingsTab';
import { LogSettingsTab } from './components/LogSettingsTab';
import { TransparentProxyMigrationNotice } from './components/TransparentProxyMigrationNotice';
import { AboutTab } from './components/AboutTab';
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
  updateInfo: _updateInfo,
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
    savingSettings,
    testingProxy,
    saveSettings,
    testProxy,
  } = useSettingsForm({ initialConfig: globalConfig, onConfigChange });

  // 监听来自App组件的导航到关于tab的事件
  useEffect(() => {
    const handleNavigateToAboutTab = () => {
      setActiveTab('about');
    };

    window.addEventListener('navigate-to-about-tab', handleNavigateToAboutTab);
    // 保持向下兼容，同时也监听 update tab
    window.addEventListener('navigate-to-update-tab', handleNavigateToAboutTab);

    return () => {
      window.removeEventListener('navigate-to-about-tab', handleNavigateToAboutTab);
      window.removeEventListener('navigate-to-update-tab', handleNavigateToAboutTab);
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
          <TabsTrigger value="log">日志配置</TabsTrigger>
          <TabsTrigger value="experimental">透明代理</TabsTrigger>
          <TabsTrigger value="about">关于</TabsTrigger>
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

        {/* 日志配置 */}
        <TabsContent value="log" className="space-y-6">
          <LogSettingsTab />
        </TabsContent>

        {/* 透明代理 (迁移提示) */}
        <TabsContent value="experimental" className="space-y-6">
          <TransparentProxyMigrationNotice />
        </TabsContent>

        {/* 关于 */}
        <TabsContent value="about" className="space-y-6">
          <AboutTab onCheckUpdate={() => onUpdateCheck?.()} />
        </TabsContent>
      </Tabs>

      {/* 保存按钮 - 仅在基本设置和代理设置时显示 */}
      {(activeTab === 'basic' || activeTab === 'proxy') && (
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
