// filepath: e:\DuckCoding\src\components\Onboarding\steps\v1\WelcomeStep.tsx

import { StepProps } from '../../../../types/onboarding';
import duckLogo from '@/assets/duck-logo.png';

export default function WelcomeStep({ onNext }: StepProps) {
  return (
    <div className="onboarding-step welcome-step">
      <div className="step-content">
        <div className="welcome-icon">
          <img src={duckLogo} alt="DuckCoding Logo" className="duck-logo" />
        </div>

        <h1 className="welcome-title">欢迎使用 DuckCoding</h1>

        <p className="welcome-subtitle">AI 编程工具统一管理平台</p>

        <div className="feature-list">
          <div className="feature-item">
            <div className="feature-icon">✨</div>
            <div className="feature-text">
              <h3>统一管理</h3>
              <p>集中管理多种 AI 编程工具</p>
            </div>
          </div>

          <div className="feature-item">
            <div className="feature-icon">🚀</div>
            <div className="feature-text">
              <h3>快速安装</h3>
              <p>一键安装和更新工具</p>
            </div>
          </div>

          <div className="feature-item">
            <div className="feature-icon">🔐</div>
            <div className="feature-text">
              <h3>透明代理</h3>
              <p>会话级端点切换与安全保护</p>
            </div>
          </div>

          <div className="feature-item">
            <div className="feature-icon">📊</div>
            <div className="feature-text">
              <h3>使用统计</h3>
              <p>实时查看使用量和配额</p>
            </div>
          </div>
        </div>

        <div className="welcome-actions">
          <button type="button" className="btn-primary btn-large" onClick={() => onNext()}>
            开始配置
          </button>
        </div>
      </div>
    </div>
  );
}
