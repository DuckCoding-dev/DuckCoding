// filepath: e:\DuckCoding\src\components\Onboarding\steps\v3\ToolDetectionStep.tsx

import { useState, useEffect, useRef } from 'react';
import { StepProps } from '../../../../types/onboarding';
import { detectAndSaveTools } from '@/lib/tauri-commands';
import type { ToolInstance } from '@/types/tool-management';

// 工具信息定义
const TOOLS = [
  { id: 'claude-code', name: 'Claude Code', icon: '🤖' },
  { id: 'codex', name: 'CodeX', icon: '📦' },
  { id: 'gemini-cli', name: 'Gemini CLI', icon: '✨' },
];

type DetectionStatus = 'pending' | 'detecting' | 'done' | 'error';

interface ToolDetectionState {
  status: DetectionStatus;
  installed: boolean;
  version: string | null;
}

export default function ToolDetectionStep({ onNext }: StepProps) {
  const [detecting, setDetecting] = useState(false);
  const [toolStates, setToolStates] = useState<Record<string, ToolDetectionState>>(() => {
    const initial: Record<string, ToolDetectionState> = {};
    TOOLS.forEach((tool) => {
      initial[tool.id] = { status: 'pending', installed: false, version: null };
    });
    return initial;
  });
  const [error, setError] = useState<string | null>(null);
  const [completed, setCompleted] = useState(false);

  // 使用 ref 追踪是否已开始检测，防止重复执行
  const hasStartedRef = useRef(false);

  const runDetection = async () => {
    if (detecting) return;

    setDetecting(true);
    setError(null);

    // 设置所有工具为检测中状态
    setToolStates((prev) => {
      const updated = { ...prev };
      TOOLS.forEach((tool) => {
        updated[tool.id] = { ...updated[tool.id], status: 'detecting' };
      });
      return updated;
    });

    try {
      // 调用后端并行检测
      const results = await detectAndSaveTools();

      // 更新各工具状态
      setToolStates((prev) => {
        const updated = { ...prev };
        results.forEach((instance: ToolInstance) => {
          if (updated[instance.base_id]) {
            updated[instance.base_id] = {
              status: 'done',
              installed: instance.installed,
              version: instance.version ?? null,
            };
          }
        });
        // 确保没有结果的工具也标记为完成
        TOOLS.forEach((tool) => {
          if (updated[tool.id].status !== 'done') {
            updated[tool.id] = { status: 'done', installed: false, version: null };
          }
        });
        return updated;
      });

      setCompleted(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : '检测失败');
      setToolStates((prev) => {
        const updated = { ...prev };
        TOOLS.forEach((tool) => {
          if (updated[tool.id].status === 'detecting') {
            updated[tool.id] = { ...updated[tool.id], status: 'error' };
          }
        });
        return updated;
      });
    } finally {
      setDetecting(false);
    }
  };

  // 组件挂载时自动开始检测（仅执行一次）
  useEffect(() => {
    if (hasStartedRef.current) return;
    hasStartedRef.current = true;
    runDetection();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const installedCount = Object.values(toolStates).filter(
    (s) => s.status === 'done' && s.installed,
  ).length;

  return (
    <div className="onboarding-step tool-detection-step">
      <div className="step-content">
        <div className="step-icon">
          <span className="icon-large">🔍</span>
        </div>

        <h2 className="step-title">检测系统工具</h2>

        <p className="step-description">正在检测您系统中已安装的 AI 编程工具...</p>

        <div className="tool-detection-list">
          {TOOLS.map((tool) => {
            const state = toolStates[tool.id];
            return (
              <div key={tool.id} className={`tool-detection-item status-${state.status}`}>
                <div className="tool-icon">{tool.icon}</div>
                <div className="tool-info">
                  <div className="tool-name">{tool.name}</div>
                  <div className="tool-status">
                    {state.status === 'pending' && <span className="text-muted">等待检测</span>}
                    {state.status === 'detecting' && (
                      <span className="text-detecting">
                        <span className="spinner" /> 检测中...
                      </span>
                    )}
                    {state.status === 'done' && state.installed && (
                      <span className="text-installed">
                        已安装 {state.version && <span className="version">v{state.version}</span>}
                      </span>
                    )}
                    {state.status === 'done' && !state.installed && (
                      <span className="text-not-installed">未安装</span>
                    )}
                    {state.status === 'error' && <span className="text-error">检测失败</span>}
                  </div>
                </div>
                <div className="tool-check">
                  {state.status === 'done' && state.installed && (
                    <span className="check-icon">✓</span>
                  )}
                  {state.status === 'done' && !state.installed && (
                    <span className="cross-icon">✗</span>
                  )}
                </div>
              </div>
            );
          })}
        </div>

        {error && (
          <div className="error-box">
            <p>{error}</p>
            <button type="button" className="btn-secondary btn-small" onClick={runDetection}>
              重试
            </button>
          </div>
        )}

        {completed && (
          <div className="detection-summary">
            {installedCount > 0 ? (
              <p className="summary-text">
                检测到 <strong>{installedCount}</strong> 个已安装的工具
              </p>
            ) : (
              <p className="summary-text">未检测到已安装的工具，您可以稍后在工具管理页面安装</p>
            )}
          </div>
        )}

        <div className="action-buttons">
          <button
            type="button"
            className="btn-primary"
            onClick={() => onNext()}
            disabled={!completed}
          >
            {completed ? '继续' : '检测中...'}
          </button>
        </div>
      </div>
    </div>
  );
}
