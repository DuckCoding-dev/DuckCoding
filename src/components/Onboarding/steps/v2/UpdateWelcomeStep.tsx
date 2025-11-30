// filepath: e:\DuckCoding\src\components\Onboarding\steps\v2\UpdateWelcomeStep.tsx

import type { StepProps } from '../../../../types/onboarding.ts';
import duckLogo from '@/assets/duck-logo.png';

export default function UpdateWelcomeStep({ onNext }: StepProps) {
  return (
    <div className="onboarding-step welcome-step">
      <div className="step-content">
        <div className="welcome-icon">
          <img src={duckLogo} alt="DuckCoding Logo" className="duck-logo" />
        </div>

        <h1 className="welcome-title">DuckCoding 更新了！</h1>
        <p className="welcome-subtitle">让我们快速了解新增的功能</p>

        <div className="feature-list">
          <div className="feature-item">
            <div className="feature-icon">📝</div>
            <div className="feature-text">
              <h3>日志配置管理</h3>
              <p>支持动态调整日志级别、输出目标，实时生效无需重启</p>
            </div>
          </div>

          <div className="feature-item">
            <div className="feature-icon">🔄</div>
            <div className="feature-text">
              <h3>会话级配置</h3>
              <p>透明代理支持会话级端点切换，无需修改工具配置</p>
            </div>
          </div>

          <div className="feature-item">
            <div className="feature-icon">🛠️</div>
            <div className="feature-text">
              <h3>多工具支持</h3>
              <p>同时管理 Claude Code、Codex、Gemini CLI 三个工具的代理</p>
            </div>
          </div>

          <div className="feature-item">
            <div className="feature-icon">⚡</div>
            <div className="feature-text">
              <h3>性能优化</h3>
              <p>改进的配置管理和更快的响应速度</p>
            </div>
          </div>
        </div>

        <div className="action-buttons">
          <div></div>
          <div className="action-right">
            <button type="button" onClick={onNext} className="btn-primary btn-large">
              开始了解
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
