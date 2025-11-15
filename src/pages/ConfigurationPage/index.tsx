import { useState, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Info, ExternalLink, Loader2, Package } from 'lucide-react';
import { PageContainer } from '@/components/layout/PageContainer';
import { ConfigOverrideDialog } from '@/components/dialogs/ConfigOverrideDialog';
import { ApiConfigForm } from './components/ApiConfigForm';
import { useConfigManagement } from './hooks/useConfigManagement';
import { groupNameMap } from '@/utils/constants';
import { openExternalLink } from '@/utils/formatting';
import { useToast } from '@/hooks/use-toast';
import type { ToolStatus } from '@/lib/tauri-commands';

interface ConfigurationPageProps {
  tools: ToolStatus[];
  loading: boolean;
}

export function ConfigurationPage({
  tools: toolsProp,
  loading: loadingProp,
}: ConfigurationPageProps) {
  const { toast } = useToast();
  const [tools, setTools] = useState<ToolStatus[]>(toolsProp);
  const [loading, setLoading] = useState(loadingProp);
  const [configOverrideDialog, setConfigOverrideDialog] = useState<{
    open: boolean;
    targetProfile: string;
    willOverride: boolean;
  }>({ open: false, targetProfile: '', willOverride: false });

  // 使用配置管理 Hook
  const {
    selectedTool,
    setSelectedTool,
    provider,
    setProvider,
    apiKey,
    setApiKey,
    baseUrl,
    setBaseUrl,
    profileName,
    setProfileName,
    configuring,
    generatingKey,
    handleGenerateApiKey,
    handleConfigureApi,
    saveConfig,
    clearForm,
  } = useConfigManagement(tools);

  // 同步外部 tools 数据
  useEffect(() => {
    setTools(toolsProp);
    setLoading(loadingProp);
  }, [toolsProp, loadingProp]);

  // 一键生成 API Key
  const onGenerateKey = async () => {
    const result = await handleGenerateApiKey();
    toast({
      title: result.success ? '生成成功' : result.success === undefined ? '缺少配置' : '生成失败',
      description: result.message,
      variant: result.success ? 'default' : 'destructive',
    });

    // 如果缺少配置，导航到设置页面
    if (!result.success && result.message.includes('全局设置')) {
      window.dispatchEvent(new CustomEvent('navigate-to-settings'));
    }
  };

  // 保存配置（带覆盖检测）
  const onSaveConfig = async () => {
    const result = await handleConfigureApi();

    // 如果需要确认覆盖，显示对话框
    if (result.needsConfirmation) {
      setConfigOverrideDialog({
        open: true,
        targetProfile: profileName || '主配置',
        willOverride: true,
      });
      return;
    }

    // 显示结果
    if (!result.success) {
      toast({
        title: '配置失败',
        description: result.message,
        variant: 'destructive',
      });
    } else {
      toast({
        title: '配置保存成功',
        description: result.message,
      });
    }
  };

  // 确认覆盖后保存
  const performConfigSave = async () => {
    const result = await saveConfig();
    setConfigOverrideDialog({ open: false, targetProfile: '', willOverride: false });

    toast({
      title: result.success ? '配置保存成功' : '配置失败',
      description: result.message,
      variant: result.success ? 'default' : 'destructive',
    });
  };

  // 切换到安装页面
  const switchToInstall = () => {
    window.dispatchEvent(new CustomEvent('navigate-to-install'));
  };

  const installedTools = tools.filter((t) => t.installed);

  return (
    <PageContainer>
      <div className="mb-6">
        <h2 className="text-2xl font-semibold mb-1">配置 API</h2>
        <p className="text-sm text-muted-foreground">配置 DuckCoding API 或自定义 API 端点</p>
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-20">
          <Loader2 className="h-8 w-8 animate-spin text-primary" />
          <span className="ml-3 text-muted-foreground">加载中...</span>
        </div>
      ) : installedTools.length > 0 ? (
        <div className="grid gap-4">
          {/* 重要提示 */}
          {provider === 'duckcoding' && (
            <div className="p-4 bg-gradient-to-r from-amber-50 to-orange-50 dark:from-amber-950 dark:to-orange-950 rounded-lg border border-amber-200 dark:border-amber-800">
              <div className="flex items-start gap-2 mb-3">
                <Info className="h-4 w-4 flex-shrink-0 mt-0.5" />
                <div className="space-y-2">
                  <h4 className="font-semibold text-amber-900 dark:text-amber-100">重要提示</h4>
                  <div className="text-sm text-amber-800 dark:text-amber-200 space-y-2">
                    <div>
                      <p className="font-semibold mb-1">DuckCoding API Key 分组:</p>
                      <ul className="list-disc list-inside space-y-1 ml-2">
                        {selectedTool && groupNameMap[selectedTool] && (
                          <li>
                            当前工具需要使用{' '}
                            <span className="font-mono bg-amber-100 dark:bg-amber-900 px-1.5 py-0.5 rounded">
                              {groupNameMap[selectedTool]}
                            </span>{' '}
                            的 API Key
                          </li>
                        )}
                        <li>每个工具必须使用其专用分组的 API Key</li>
                        <li>API Key 不能混用</li>
                      </ul>
                    </div>
                    <div>
                      <p className="font-semibold mb-1">获取 API Key:</p>
                      <button
                        onClick={() => openExternalLink('https://duckcoding.com/console/token')}
                        className="inline-flex items-center gap-1 text-amber-700 dark:text-amber-300 hover:underline font-medium cursor-pointer bg-transparent border-0 p-0"
                      >
                        访问 DuckCoding 控制台 <ExternalLink className="h-3 w-3" />
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* API 配置表单 */}
          <ApiConfigForm
            selectedTool={selectedTool}
            setSelectedTool={setSelectedTool}
            provider={provider}
            setProvider={setProvider}
            apiKey={apiKey}
            setApiKey={setApiKey}
            baseUrl={baseUrl}
            setBaseUrl={setBaseUrl}
            profileName={profileName}
            setProfileName={setProfileName}
            installedTools={installedTools}
            configuring={configuring}
            generatingKey={generatingKey}
            onGenerateKey={onGenerateKey}
            onSaveConfig={onSaveConfig}
            onClearForm={clearForm}
          />
        </div>
      ) : (
        <Card className="shadow-sm border">
          <CardContent className="pt-6">
            <div className="text-center py-12">
              <Package className="h-16 w-16 mx-auto mb-4 text-muted-foreground opacity-30" />
              <h3 className="text-lg font-semibold mb-2">暂无已安装的工具</h3>
              <p className="text-sm text-muted-foreground mb-4">请先安装工具后再进行配置</p>
              <Button
                onClick={switchToInstall}
                className="shadow-md hover:shadow-lg transition-all"
              >
                <Package className="mr-2 h-4 w-4" />
                前往安装
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      {/* 配置覆盖确认对话框 */}
      <ConfigOverrideDialog
        open={configOverrideDialog.open}
        targetProfile={configOverrideDialog.targetProfile}
        configuring={configuring}
        onClose={() =>
          setConfigOverrideDialog({ open: false, targetProfile: '', willOverride: false })
        }
        onConfirmOverride={performConfigSave}
      />
    </PageContainer>
  );
}
