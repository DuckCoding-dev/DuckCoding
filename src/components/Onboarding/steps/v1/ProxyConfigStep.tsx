// filepath: e:\DuckCoding\src\components\Onboarding\steps\v1\ProxyConfigStep.tsx

import { useEffect } from 'react';
import { StepProps } from '../../../../types/onboarding';
import { emit, listen } from '@tauri-apps/api/event';

export default function ProxyConfigStep({ onNext, onPrevious, isFirst }: StepProps) {
  const handleGoToSettings = async () => {
    try {
      // 先隐藏引导界面
      await emit('hide-onboarding');
      // 打开设置页面的代理 tab，并限制只能访问这个 tab
      await emit('open-settings', { tab: 'proxy', restrictToTab: true });
    } catch (error) {
      console.error('打开设置页面失败:', error);
    }
  };

  const handleNext = async () => {
    // 清除限制，确保引导界面显示
    await emit('open-settings', { tab: 'basic', restrictToTab: false });
    await emit('show-onboarding');
    onNext();
  };

  // 监听继续引导事件
  useEffect(() => {
    const unlisten = listen('continue-onboarding', async () => {
      // 用户点击悬浮按钮继续，直接进入下一步
      await emit('open-settings', { tab: 'basic', restrictToTab: false });
      await emit('show-onboarding');
      onNext();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [onNext]);

  return (
    <div className="onboarding-step proxy-config-step">
      <div className="step-content">
        <h1 className="step-title">全局代理配置（可选）</h1>

        <p className="step-description">
          如果您需要通过代理访问 AI 服务，可以点击「前往配置」打开设置页面进行配置。
        </p>

        <div className="info-box">
          <div className="info-icon">ℹ️</div>
          <div className="info-content">
            <h3>什么是全局代理？</h3>
            <p>
              全局代理会应用到所有 AI 工具的网络请求，包括工具安装、更新检查和 API 调用。
              如果您在国内网络环境下使用，配置代理可能会改善连接稳定性。
            </p>
          </div>
        </div>

        <div className="config-hint">
          <h3>支持的代理类型</h3>
          <ul>
            <li>HTTP / HTTPS 代理</li>
            <li>SOCKS5 代理</li>
            <li>支持代理认证（用户名/密码）</li>
            <li>支持代理过滤 URL 列表</li>
          </ul>
        </div>

        <div className="action-buttons">
          <button type="button" className="btn-secondary" onClick={onPrevious} disabled={isFirst}>
            上一步
          </button>

          <div className="action-right">
            <button type="button" className="btn-secondary" onClick={handleGoToSettings}>
              前往配置
            </button>
            <button type="button" className="btn-primary" onClick={handleNext}>
              下一步
            </button>
          </div>
        </div>

        <p className="step-note">
          提示：点击「前往配置」会打开设置页面，配置完成后点击「下一步」继续引导
        </p>
      </div>
    </div>
  );
}
