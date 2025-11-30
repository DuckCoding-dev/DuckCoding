// filepath: e:\DuckCoding\src\components\Onboarding\steps\v1\ToolIntroStep.tsx

import { StepProps } from '../../../../types/onboarding';

export default function ToolIntroStep({ onNext, onPrevious, isFirst }: StepProps) {
  return (
    <div className="onboarding-step tool-intro-step">
      <div className="step-content">
        <h1 className="step-title">支持的 AI 编程工具</h1>

        <p className="step-description">
          DuckCoding 支持以下主流 AI 编程工具的管理，您可以在主界面中一键安装和配置
        </p>

        <div className="tools-grid">
          <div className="tool-card">
            <div className="tool-header">
              <div className="tool-logo">C</div>
              <h3>Claude Code</h3>
            </div>
            <p className="tool-description">
              Anthropic 官方 CLI 工具，提供强大的代码生成和分析能力
            </p>
            <ul className="tool-features">
              <li>智能代码补全</li>
              <li>代码解释和重构</li>
              <li>多文件项目理解</li>
            </ul>
          </div>

          <div className="tool-card">
            <div className="tool-header">
              <div className="tool-logo">Cx</div>
              <h3>Codex</h3>
            </div>
            <p className="tool-description">基于大语言模型的代码助手，支持多种编程语言</p>
            <ul className="tool-features">
              <li>自然语言转代码</li>
              <li>代码审查和优化</li>
              <li>测试用例生成</li>
            </ul>
          </div>

          <div className="tool-card">
            <div className="tool-header">
              <div className="tool-logo">G</div>
              <h3>Gemini CLI</h3>
            </div>
            <p className="tool-description">Google Gemini 的命令行工具，多模态 AI 编程助手</p>
            <ul className="tool-features">
              <li>图像识别辅助编程</li>
              <li>长上下文支持</li>
              <li>跨语言项目理解</li>
            </ul>
          </div>
        </div>

        <div className="info-box">
          <div className="info-icon">💡</div>
          <div className="info-content">
            <h3>下一步</h3>
            <p>
              完成引导后，您可以在主界面的「工具管理」页面中安装和配置这些工具。 DuckCoding
              会自动检测您的系统环境，并提供一键安装功能。
            </p>
          </div>
        </div>

        <div className="action-buttons">
          <button type="button" className="btn-secondary" onClick={onPrevious} disabled={isFirst}>
            上一步
          </button>

          <button type="button" className="btn-primary" onClick={() => onNext()}>
            下一步
          </button>
        </div>
      </div>
    </div>
  );
}
