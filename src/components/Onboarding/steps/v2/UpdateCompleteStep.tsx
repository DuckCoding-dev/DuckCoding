// filepath: e:\DuckCoding\src\components\Onboarding\steps\v2\UpdateCompleteStep.tsx

import type { StepProps } from '../../../../types/onboarding.ts';

export default function UpdateCompleteStep({ onNext, onPrevious }: StepProps) {
  return (
    <div className="onboarding-step complete-step">
      <div className="step-content">
        <div className="complete-icon">
          <div
            style={{
              fontSize: '3rem',
              background: 'hsl(var(--primary))',
              width: '80px',
              height: '80px',
              borderRadius: '50%',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
          >
            ✨
          </div>
        </div>

        <h1 className="complete-title">欢迎回来！</h1>
        <p className="complete-subtitle">您已了解所有新功能，现在可以开始使用了</p>

        <div className="quick-start-guide">
          <h3>快速开始使用新功能</h3>
          <div className="guide-steps">
            <div className="guide-step">
              <div className="guide-number">1</div>
              <div className="guide-content">
                <h4>配置日志</h4>
                <p>前往「设置 → 日志设置」调整日志级别和输出方式</p>
              </div>
            </div>

            <div className="guide-step">
              <div className="guide-number">2</div>
              <div className="guide-content">
                <h4>启用会话级配置</h4>
                <p>在「透明代理」页面开启会话级 Endpoint 配置</p>
              </div>
            </div>

            <div className="guide-step">
              <div className="guide-number">3</div>
              <div className="guide-content">
                <h4>管理多个工具</h4>
                <p>为 Claude Code、Codex、Gemini CLI 分别配置代理</p>
              </div>
            </div>
          </div>
        </div>

        <div className="helpful-tips">
          <h3>💡 提示</h3>
          <ul>
            <li>遇到问题可以通过调整日志级别获取更多诊断信息</li>
            <li>会话历史会自动保存，方便快速切换常用配置</li>
            <li>多个代理可以同时运行，互不影响</li>
          </ul>
        </div>

        <div className="action-buttons">
          <button type="button" onClick={onPrevious} className="btn-text">
            上一步
          </button>
          <div className="action-right">
            <button type="button" onClick={onNext} className="btn-primary btn-large">
              开始使用
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
