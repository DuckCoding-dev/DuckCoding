// filepath: e:\DuckCoding\src\components\Onboarding\OnboardingOverlay.tsx

import { useState, useEffect } from 'react';
import { listen, emit } from '@tauri-apps/api/event';
import { X } from 'lucide-react';
import OnboardingFlow from './OnboardingFlow';
import type { OnboardingStep } from '../../types/onboarding';
import './onboarding.css';

interface OnboardingOverlayProps {
  /** 需要展示的引导步骤列表 */
  steps: OnboardingStep[];
  /** 完成引导的回调函数 */
  onComplete: () => void;
  /** 是否允许退出（主动查看时为 true，强制引导时为 false） */
  canExit?: boolean;
  /** 退出引导的回调函数 */
  onExit?: () => void;
}

export default function OnboardingOverlay({
  steps,
  onComplete,
  canExit = false,
  onExit,
}: OnboardingOverlayProps) {
  const [isHidden, setIsHidden] = useState(false);

  useEffect(() => {
    // 监听隐藏引导事件
    const unlisten1 = listen('hide-onboarding', () => {
      setIsHidden(true);
    });

    // 监听显示引导事件
    const unlisten2 = listen('show-onboarding', () => {
      setIsHidden(false);
    });

    return () => {
      unlisten1.then((fn) => fn());
      unlisten2.then((fn) => fn());
    };
  }, []);

  const handleContinueOnboarding = async () => {
    // 触发继续引导事件，让当前步骤决定如何处理
    await emit('continue-onboarding');
  };

  const handleExit = () => {
    if (onExit) {
      onExit();
    }
  };

  return (
    <>
      {/* 悬浮继续按钮（在引导隐藏时显示） */}
      {isHidden && (
        <div className="onboarding-continue-fab">
          <button
            type="button"
            onClick={handleContinueOnboarding}
            className="fab-button"
            title="继续引导"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <path d="M5 12h14" />
              <path d="m12 5 7 7-7 7" />
            </svg>
            <span>继续引导</span>
          </button>
        </div>
      )}

      {/* 引导主界面（用 CSS 控制显隐，保持组件挂载） */}
      <div className="onboarding-overlay" style={{ display: isHidden ? 'none' : 'flex' }}>
        {/* 全屏遮罩背景 */}
        <div className="overlay-backdrop" />

        {/* 引导窗口容器 */}
        <div className="overlay-content">
          <div className="onboarding-window">
            {/* 退出按钮（仅在允许退出时显示） */}
            {canExit && (
              <button
                type="button"
                onClick={handleExit}
                className="onboarding-exit-button"
                title="退出引导"
              >
                <X size={20} />
              </button>
            )}

            <OnboardingFlow steps={steps} onComplete={onComplete} />
          </div>
        </div>
      </div>
    </>
  );
}
