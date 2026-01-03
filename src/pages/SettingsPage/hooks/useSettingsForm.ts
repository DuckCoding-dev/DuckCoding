import { useState, useEffect, useCallback } from 'react';
import {
  saveGlobalConfig,
  testProxyRequest,
  type GlobalConfig,
  type TestProxyResult,
  type ProxyTestConfig,
} from '@/lib/tauri-commands';

interface UseSettingsFormProps {
  initialConfig: GlobalConfig | null;
  onConfigChange: () => void;
}

export function useSettingsForm({ initialConfig, onConfigChange }: UseSettingsFormProps) {
  // 代理设置状态
  const [proxyEnabled, setProxyEnabled] = useState(false);
  const [proxyType, setProxyType] = useState<'http' | 'https' | 'socks5'>('http');
  const [proxyHost, setProxyHost] = useState('');
  const [proxyPort, setProxyPort] = useState('');
  const [proxyUsername, setProxyUsername] = useState('');
  const [proxyPassword, setProxyPassword] = useState('');
  const [proxyTestUrl, setProxyTestUrl] = useState('https://duckcoding.com/');
  const [proxyBypassUrls, setProxyBypassUrls] = useState<string[]>([
    'localhost',
    '127.0.0.1',
    '0.0.0.0',
    '::1',
    '*.local',
    '*.lan',
  ]);

  // 实验性功能 - 透明代理

  // 状态
  const [globalConfig, setGlobalConfig] = useState<GlobalConfig | null>(initialConfig);
  const [savingSettings, setSavingSettings] = useState(false);
  const [testingProxy, setTestingProxy] = useState(false);

  // 当外部 initialConfig 更新时，同步内部状态和表单
  useEffect(() => {
    if (initialConfig) {
      setGlobalConfig(initialConfig);

      // 填充表单
      setProxyEnabled(initialConfig.proxy_enabled || false);
      setProxyType(initialConfig.proxy_type || 'http');
      setProxyHost(initialConfig.proxy_host || '');
      setProxyPort(initialConfig.proxy_port || '');
      setProxyUsername(initialConfig.proxy_username || '');
      setProxyPassword(initialConfig.proxy_password || '');
      setProxyBypassUrls(
        initialConfig.proxy_bypass_urls || [
          'localhost',
          '127.0.0.1',
          '0.0.0.0',
          '::1',
          '*.local',
          '*.lan',
        ],
      );
    }
  }, [initialConfig]);

  // 保存配置
  const saveSettings = useCallback(async (): Promise<void> => {
    const proxyPortNumber = proxyPort ? parseInt(proxyPort) : 0;
    if (proxyEnabled && (!proxyHost.trim() || proxyPortNumber <= 0)) {
      throw new Error('代理地址和端口不能为空');
    }

    setSavingSettings(true);
    try {
      const configToSave: GlobalConfig = {
        proxy_enabled: proxyEnabled,
        proxy_type: proxyType,
        proxy_host: proxyHost.trim(),
        proxy_port: proxyPort,
        proxy_username: proxyUsername.trim(),
        proxy_password: proxyPassword,
        proxy_bypass_urls: proxyBypassUrls.map((url) => url.trim()).filter((url) => url.length > 0),
      };

      await saveGlobalConfig(configToSave);

      // 通知父组件刷新全局配置
      onConfigChange();
    } finally {
      setSavingSettings(false);
    }
  }, [
    proxyEnabled,
    proxyType,
    proxyHost,
    proxyPort,
    proxyUsername,
    proxyPassword,
    proxyBypassUrls,
    onConfigChange,
  ]);

  // 生成代理 API Key（已废弃，透明代理功能已移除）
  const generateProxyKey = useCallback(() => {
    // 功能已移除
    console.warn('generateProxyKey 功能已废弃');
  }, []);

  // 测试代理连接
  const testProxy = useCallback(async (): Promise<{
    success: boolean;
    message: string;
    details?: string;
  }> => {
    // 验证代理配置
    if (!proxyEnabled) {
      return {
        success: false,
        message: '请先启用代理',
      };
    }

    const proxyPortNumber = proxyPort ? parseInt(proxyPort) : 0;
    if (!proxyHost.trim() || proxyPortNumber <= 0) {
      return {
        success: false,
        message: '请填写完整的代理地址和端口',
      };
    }

    // 验证测试URL
    if (!proxyTestUrl.trim()) {
      return {
        success: false,
        message: '请输入测试URL',
      };
    }

    // 简单的URL格式验证
    try {
      new URL(proxyTestUrl);
    } catch {
      return {
        success: false,
        message: '测试URL格式不正确',
      };
    }

    setTestingProxy(true);
    try {
      // 构建代理配置（使用当前表单输入，不需要先保存）
      const proxyConfig: ProxyTestConfig = {
        enabled: proxyEnabled,
        proxy_type: proxyType,
        host: proxyHost.trim(),
        port: proxyPort,
        username: proxyUsername.trim() || undefined,
        password: proxyPassword || undefined,
      };

      // 测试代理请求
      const result: TestProxyResult = await testProxyRequest(proxyTestUrl, proxyConfig);

      if (result.success) {
        return {
          success: true,
          message: '代理连接成功！',
          details: `测试URL: ${result.url || '未知'}\n响应状态: ${result.status}`,
        };
      } else {
        return {
          success: false,
          message: '代理连接失败',
          details: result.error || '未知错误',
        };
      }
    } catch (error) {
      console.error('Failed to test proxy:', error);
      return {
        success: false,
        message: '测试失败',
        details: String(error),
      };
    } finally {
      setTestingProxy(false);
    }
  }, [proxyEnabled, proxyType, proxyHost, proxyPort, proxyUsername, proxyPassword, proxyTestUrl]);

  return {
    // Proxy settings
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

    // State
    globalConfig,
    savingSettings,
    testingProxy,

    // Actions
    saveSettings,
    generateProxyKey,
    testProxy,
  };
}
