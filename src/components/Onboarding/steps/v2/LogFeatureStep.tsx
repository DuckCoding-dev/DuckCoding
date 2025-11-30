// filepath: e:\DuckCoding\src\components\Onboarding\steps\v2\LogFeatureStep.tsx

import type { StepProps } from '../../../../types/onboarding.ts';

export default function LogFeatureStep({ onNext, onPrevious }: StepProps) {
  return (
    <div className="onboarding-step">
      <div className="step-content">
        <h2 className="step-title">日志配置管理</h2>
        <p className="step-description">灵活控制应用日志，便于问题诊断和性能分析</p>

        <div className="info-box">
          <div className="info-icon">📝</div>
          <div className="info-content">
            <h3>为什么需要日志管理？</h3>
            <p>
              日志是诊断问题的重要工具。通过动态配置日志级别和输出目标，您可以在需要时获取详细信息，平时保持简洁输出。
            </p>
          </div>
        </div>

        <div className="config-hint">
          <h3>日志配置功能</h3>
          <ul>
            <li>动态调整日志级别（TRACE、DEBUG、INFO、WARN、ERROR）</li>
            <li>灵活配置输出目标（控制台、文件、两者同时）</li>
            <li>热重载支持，配置更改立即生效无需重启</li>
            <li>自动日志文件管理，按日期归档</li>
          </ul>
        </div>

        <div className="tools-grid" style={{ gridTemplateColumns: 'repeat(2, 1fr)' }}>
          <div className="tool-card">
            <div className="tool-header">
              <div className="tool-logo">🎯</div>
              <h3>日志级别</h3>
            </div>
            <p className="tool-description">从详细到简洁五个级别可选</p>
            <ul className="tool-features">
              <li>TRACE：最详细的调试信息</li>
              <li>DEBUG：开发调试使用</li>
              <li>INFO：常规运行信息</li>
            </ul>
          </div>

          <div className="tool-card">
            <div className="tool-header">
              <div className="tool-logo">📁</div>
              <h3>输出目标</h3>
            </div>
            <p className="tool-description">选择日志输出位置</p>
            <ul className="tool-features">
              <li>控制台：实时查看</li>
              <li>文件：长期保存</li>
              <li>两者：全面记录</li>
            </ul>
          </div>
        </div>

        <p className="step-note">您可以在「设置 → 日志设置」中随时调整这些配置</p>

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
