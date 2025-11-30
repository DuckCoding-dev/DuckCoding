// filepath: e:\DuckCoding\src\components\Onboarding\steps\v2\SessionConfigStep.tsx

import type { StepProps } from '../../../../types/onboarding.ts';

export default function SessionConfigStep({ onNext, onPrevious }: StepProps) {
  return (
    <div className="onboarding-step">
      <div className="step-content">
        <h2 className="step-title">透明代理增强功能</h2>
        <p className="step-description">会话级配置和多工具支持，让代理管理更灵活</p>

        <div className="info-box">
          <div className="info-icon">🔄</div>
          <div className="info-content">
            <h3>什么是会话级配置？</h3>
            <p>
              会话级配置允许您在透明代理运行时动态切换 API
              端点和密钥，无需停止代理或修改工具配置。每次会话都会被记录，方便追溯和复用。
            </p>
          </div>
        </div>

        <div className="config-hint">
          <h3>新增功能特性</h3>
          <ul>
            <li>会话级 Endpoint 和 API Key 切换，无需重启代理</li>
            <li>同时管理三个工具：Claude Code、Codex、Gemini CLI</li>
            <li>每个工具独立的端口、配置和会话历史</li>
            <li>支持多个代理同时运行，互不干扰</li>
            <li>会话历史记录，快速复用之前的配置</li>
          </ul>
        </div>

        <div className="tools-grid">
          <div className="tool-card">
            <div className="tool-header">
              <div className="tool-logo">CC</div>
              <h3>Claude Code</h3>
            </div>
            <p className="tool-description">官方 CLI 工具</p>
            <ul className="tool-features">
              <li>默认端口：8787</li>
              <li>独立配置管理</li>
            </ul>
          </div>

          <div className="tool-card">
            <div className="tool-header">
              <div className="tool-logo">CX</div>
              <h3>Codex</h3>
            </div>
            <p className="tool-description">第三方客户端</p>
            <ul className="tool-features">
              <li>默认端口：8788</li>
              <li>独立配置管理</li>
            </ul>
          </div>

          <div className="tool-card">
            <div className="tool-header">
              <div className="tool-logo">GM</div>
              <h3>Gemini CLI</h3>
            </div>
            <p className="tool-description">Google AI 工具</p>
            <ul className="tool-features">
              <li>默认端口：8789</li>
              <li>独立配置管理</li>
            </ul>
          </div>
        </div>

        <p className="step-note">前往「透明代理」页面即可查看和管理所有工具的代理配置</p>

        <div className="action-buttons">
          <button type="button" onClick={onPrevious} className="btn-text">
            上一步
          </button>
          <div className="action-right">
            <button type="button" onClick={onNext} className="btn-primary">
              下一步
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
