// filepath: e:\DuckCoding\src\components\Onboarding\steps\v3\ToolDetectionStep.tsx

import { useEffect } from 'react';
import { StepProps } from '../../../../types/onboarding';
import type { OnboardingNavigationPayload } from '../../../../types/onboarding';
import { emit, listen } from '@tauri-apps/api/event';

export default function ToolDetectionStep({ onNext, onPrevious, isFirst }: StepProps) {
  const handleGoToToolManagement = async () => {
    try {
      // 隐藏引导界面
      await emit('hide-onboarding');
      // 使用标准化导航事件
      await emit('onboarding-navigate', {
        targetPage: 'tool-management',
        autoAction: 'open-add-instance-dialog',
      } as OnboardingNavigationPayload);
    } catch (error) {
      console.error('打开工具管理页面失败:', error);
    }
  };

  const handleNext = async () => {
    // 清除限制并恢复引导界面
    await emit('clear-onboarding-restriction');
    await emit('show-onboarding');
    onNext();
  };

  // 监听继续引导事件
  useEffect(() => {
    const unlisten = listen('continue-onboarding', async () => {
      await emit('clear-onboarding-restriction');
      await emit('show-onboarding');
      onNext();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [onNext]);

  return (
    <div className="onboarding-step tool-detection-step">
      <div className="step-content">
        <div className="step-icon">
          <span className="icon-large">🛠️</span>
        </div>

        <h2 className="step-title">配置工具实例</h2>

        <p className="step-description">
          DuckCoding 支持管理多个 AI 编程工具实例，您可以在工具管理页面添加和配置它们
        </p>

        <div className="info-box">
          <div className="info-icon">💡</div>
          <div className="info-content">
            <h3>如何添加工具实例？</h3>
            <p>点击下方的「前往配置」按钮，在工具管理页面点击「添加实例」，然后选择：</p>
            <ul>
              <li>
                <strong>本地环境</strong>：支持自动扫描或手动指定路径
              </li>
              <li>
                <strong>WSL 环境</strong>：在 Windows 子系统 Linux 中使用工具
              </li>
            </ul>
          </div>
        </div>

        <div className="config-hint">
          <h3>支持的添加方式</h3>
          <ul>
            <li>
              <strong>自动扫描</strong>：自动检测系统中已安装的工具（npm、Homebrew 等）
            </li>
            <li>
              <strong>手动指定</strong>：选择工具可执行文件路径
            </li>
          </ul>
        </div>

        <div className="action-buttons">
          <button type="button" className="btn-secondary" onClick={onPrevious} disabled={isFirst}>
            上一步
          </button>

          <div className="action-right">
            <button type="button" className="btn-secondary" onClick={handleGoToToolManagement}>
              前往配置
            </button>
            <button type="button" className="btn-primary" onClick={handleNext}>
              下一步
            </button>
          </div>
        </div>

        <p className="step-note">
          提示：点击「前往配置」会打开工具管理页面，配置完成后点击「下一步」继续引导
        </p>
      </div>
    </div>
  );
}
