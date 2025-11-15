import { useState, useEffect } from 'react';
import {
  configureApi,
  generateApiKeyForTool,
  getGlobalConfig,
  type ToolStatus,
  type GlobalConfig,
} from '@/lib/tauri-commands';
import { useProfileLoader } from '@/hooks/useProfileLoader';

export function useConfigManagement(tools: ToolStatus[]) {
  const [selectedTool, setSelectedTool] = useState<string>('');
  const [provider, setProvider] = useState<string>('duckcoding');
  const [apiKey, setApiKey] = useState<string>('');
  const [baseUrl, setBaseUrl] = useState<string>('');
  const [profileName, setProfileName] = useState<string>('');
  const [configuring, setConfiguring] = useState(false);
  const [generatingKey, setGeneratingKey] = useState(false);
  const [globalConfig, setGlobalConfig] = useState<GlobalConfig | null>(null);

  // 使用共享配置加载 Hook
  const { profiles, activeConfigs, loadAllProfiles } = useProfileLoader(tools);

  // 加载全局配置
  useEffect(() => {
    const loadConfig = async () => {
      try {
        const config = await getGlobalConfig();
        setGlobalConfig(config);
      } catch (error) {
        console.error('Failed to load global config:', error);
      }
    };

    loadConfig();
  }, []);

  // 当工具加载完成后，设置默认选中的工具并加载配置
  useEffect(() => {
    const installedTools = tools.filter((t) => t.installed);
    if (!selectedTool && installedTools.length > 0) {
      setSelectedTool(installedTools[0].id);
    }
    if (installedTools.length > 0) {
      loadAllProfiles();
    }
    // 移除 loadAllProfiles 依赖，避免循环依赖
    // loadAllProfiles 已经正确依赖了 tools，无需重复添加
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tools, selectedTool]);

  // 生成 API Key
  const handleGenerateApiKey = async (): Promise<{ success: boolean; message: string }> => {
    if (!selectedTool) {
      return { success: false, message: '请先选择工具' };
    }

    if (!globalConfig?.user_id || !globalConfig?.system_token) {
      return { success: false, message: '请先在全局设置中配置用户ID和系统访问令牌' };
    }

    try {
      setGeneratingKey(true);
      const result = await generateApiKeyForTool(selectedTool);

      if (result.success && result.api_key) {
        setApiKey(result.api_key);
        return { success: true, message: 'API Key生成成功！已自动填入配置框' };
      } else {
        return { success: false, message: result.message || '未知错误' };
      }
    } catch (error) {
      return { success: false, message: String(error) };
    } finally {
      setGeneratingKey(false);
    }
  };

  // 检查是否会覆盖现有配置
  const handleConfigureApi = async (): Promise<{
    success: boolean;
    message: string;
    needsConfirmation?: boolean;
  }> => {
    if (!selectedTool || !apiKey) {
      const errors = [];
      if (!selectedTool) errors.push('• 请选择工具');
      if (!apiKey) errors.push('• 请输入 API Key');
      return { success: false, message: errors.join('\n') };
    }

    if (provider === 'custom' && !baseUrl.trim()) {
      return { success: false, message: '选择自定义端点时必须填写有效的 Base URL' };
    }

    // 确保拥有最新的配置数据
    let currentConfig = activeConfigs[selectedTool];
    if (!currentConfig) {
      // 重新加载以获取最新配置
      await loadAllProfiles();
      currentConfig = activeConfigs[selectedTool];
    }

    // 检查是否会覆盖现有配置
    const existingProfiles = profiles[selectedTool] || [];
    const hasRealConfig =
      currentConfig && currentConfig.api_key !== '未配置' && currentConfig.base_url !== '未配置';
    const willOverride = profileName ? existingProfiles.includes(profileName) : hasRealConfig;

    if (willOverride) {
      return { success: false, message: '', needsConfirmation: true };
    }

    return await saveConfig();
  };

  // 执行保存配置
  const saveConfig = async (): Promise<{ success: boolean; message: string }> => {
    try {
      setConfiguring(true);

      await configureApi(
        selectedTool,
        provider,
        apiKey,
        provider === 'custom' ? baseUrl.trim() : undefined,
        profileName || undefined,
      );

      // 清空表单
      setApiKey('');
      setBaseUrl('');
      setProfileName('');

      // 重新加载配置列表
      await loadAllProfiles();

      return {
        success: true,
        message: `配置保存成功！${profileName ? `\n配置名称: ${profileName}` : ''}`,
      };
    } catch (error) {
      return { success: false, message: String(error) };
    } finally {
      setConfiguring(false);
    }
  };

  // 清空表单
  const clearForm = () => {
    setApiKey('');
    setBaseUrl('');
    setProfileName('');
  };

  return {
    // State
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
    activeConfigs,
    profiles,
    globalConfig,

    // Actions
    handleGenerateApiKey,
    handleConfigureApi,
    saveConfig,
    clearForm,
  };
}
