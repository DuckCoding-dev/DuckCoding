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
  initialTab?: string;
  restrictToTab?: string; // 限制只能访问特定 tab
}

export function SettingsPage({
  globalConfig,
  onConfigChange,
  updateInfo: _updateInfo,
  onUpdateCheck,
  initialTab = 'basic',
  restrictToTab,
}: SettingsPageProps) {
  const { toast } = useToast();
  const [activeTab, setActiveTab] = useState(initialTab);

  // 如果有 restrictToTab，阻止切换到其他 tab
  const handleTabChange = (value: string) => {
    if (restrictToTab && value !== restrictToTab) {
      toast({
        title: '请先完成当前配置',
        description: '完成引导配置后即可访问其他设置',
        variant: 'default',
      });
      return;
    }
    setActiveTab(value);
  };

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
      {/* 引导模式提示 */}
      {restrictToTab && (
        <div className="mb-4 p-4 bg-primary/10 border border-primary/20 rounded-lg">
          <div className="flex items-start gap-3">
            <div className="flex-shrink-0 mt-0.5">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                className="h-5 w-5 text-primary"
                viewBox="0 0 20 20"
                fill="currentColor"
              >
                <path
                  fillRule="evenodd"
                  d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z"
                  clipRule="evenodd"
                />
              </svg>
            </div>
            <div className="flex-1">
              <h3 className="text-sm font-semibold text-primary mb-1">引导模式</h3>
              <p className="text-sm text-muted-foreground">
                您正在配置引导流程中的代理设置。完成配置后，请点击右下角的「继续引导」按钮返回引导流程。
              </p>
            </div>
          </div>
        </div>
      )}

      <div className="mb-6">
        <h2 className="text-2xl font-semibold mb-1">全局设置</h2>
        <p className="text-sm text-muted-foreground">配置 DuckCoding 的全局参数和功能</p>
      </div>

      <Tabs value={activeTab} onValueChange={handleTabChange} className="space-y-6">
        <TabsList>
          <TabsTrigger value="basic" disabled={restrictToTab && restrictToTab !== 'basic'}>
            基本设置
          </TabsTrigger>
          <TabsTrigger value="proxy" disabled={restrictToTab && restrictToTab !== 'proxy'}>
            代理设置
          </TabsTrigger>
          <TabsTrigger value="log" disabled={restrictToTab && restrictToTab !== 'log'}>
            日志配置
          </TabsTrigger>
          <TabsTrigger
            value="experimental"
            disabled={restrictToTab && restrictToTab !== 'experimental'}
          >
            透明代理
          </TabsTrigger>
          <TabsTrigger value="about" disabled={restrictToTab && restrictToTab !== 'about'}>
            关于
          </TabsTrigger>
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
