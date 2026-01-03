// filepath: e:\DuckCoding\src\components\Onboarding\steps\v1\CompleteStep.tsx

import { StepProps } from '../../../../types/onboarding';

export default function CompleteStep({ onNext, onPrevious, isFirst }: StepProps) {
  return (
    <div className="onboarding-step complete-step">
      <div className="step-content">
        <div className="complete-icon">
          <svg
            width="80"
            height="80"
            viewBox="0 0 80 80"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
            className="text-primary"
          >
            <circle cx="40" cy="40" r="40" fill="currentColor" opacity="0.1" />
            <path
              d="M56 28L34 50L24 40"
              stroke="currentColor"
              strokeWidth="5"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </div>

        <h1 className="complete-title">准备就绪！</h1>

        <p className="complete-subtitle">您已完成基础配置，现在可以开始使用 DuckCoding 了</p>

        <div className="quick-start-guide">
          <h3>快速开始</h3>
          <div className="guide-steps">
            <div className="guide-step">
              <div className="guide-number">1</div>
              <div className="guide-content">
                <h4>安装工具</h4>
                <p>在「工具管理」页面中选择并安装需要的 AI 编程工具</p>
              </div>
            </div>

            <div className="guide-step">
              <div className="guide-number">2</div>
              <div className="guide-content">
                <h4>配置 API</h4>
                <p>为每个工具配置 API Key 和端点，开始使用 AI 功能</p>
              </div>
            </div>

            <div className="guide-step">
              <div className="guide-number">3</div>
              <div className="guide-content">
                <h4>体验透明代理</h4>
                <p>启用透明代理功能，实现会话级端点切换和 API Key 保护</p>
              </div>
            </div>
          </div>
        </div>

        <div className="helpful-tips">
          <h3>温馨提示</h3>
          <ul>
            <li>推荐在「设置 → 应用设置」中启用开机自启动，方便快速访问</li>
            <li>您可以随时在「设置」中修改全局代理配置</li>
            <li>在「使用统计」页面查看 API 使用量和配额</li>
            <li>遇到问题？查看「帮助」或访问我们的 GitHub 仓库</li>
            <li>可以在设置中重新打开此引导</li>
          </ul>
        </div>

        <div className="action-buttons">
          <button type="button" className="btn-secondary" onClick={onPrevious} disabled={isFirst}>
            上一步
          </button>

          <button type="button" className="btn-primary btn-large" onClick={() => onNext()}>
            开始使用 DuckCoding
          </button>
        </div>
      </div>
    </div>
  );
}
