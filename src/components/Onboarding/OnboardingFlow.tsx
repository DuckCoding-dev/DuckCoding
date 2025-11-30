// filepath: e:\DuckCoding\src\components\Onboarding\OnboardingFlow.tsx

import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { OnboardingStep } from '../../types/onboarding';
import { CURRENT_ONBOARDING_VERSION } from './config/versions';

interface OnboardingFlowProps {
  /** 需要展示的引导步骤列表 */
  steps: OnboardingStep[];
  /** 完成引导的回调函数 */
  onComplete: () => void;
}

export default function OnboardingFlow({ steps, onComplete }: OnboardingFlowProps) {
  const [currentStepIndex, setCurrentStepIndex] = useState(0);
  const [skippedSteps, setSkippedSteps] = useState<string[]>([]);

  const currentStep = steps[currentStepIndex];
  const CurrentStepComponent = currentStep.component;

  const isFirst = currentStepIndex === 0;
  const isLast = currentStepIndex === steps.length - 1;

  const handleNext = async (skipped = false) => {
    // 记录跳过的步骤
    if (skipped && currentStep.skippable) {
      const newSkippedSteps = [...skippedSteps, currentStep.id];
      setSkippedSteps(newSkippedSteps);

      // 保存进度到后端
      try {
        await invoke('save_onboarding_progress', {
          version: CURRENT_ONBOARDING_VERSION,
          skippedSteps: newSkippedSteps,
        });
      } catch (error) {
        console.error('保存引导进度失败:', error);
      }
    }

    // 如果是最后一步，完成引导
    if (isLast) {
      try {
        await invoke('complete_onboarding', {
          version: CURRENT_ONBOARDING_VERSION,
        });
        onComplete();
      } catch (error) {
        console.error('完成引导失败:', error);
      }
    } else {
      // 否则进入下一步
      setCurrentStepIndex((prev) => prev + 1);
    }
  };

  const handlePrevious = () => {
    if (!isFirst) {
      setCurrentStepIndex((prev) => prev - 1);
    }
  };

  return (
    <div className="onboarding-flow">
      {/* 进度指示器 */}
      <div className="progress-indicator">
        <div className="progress-bar">
          <div
            className="progress-fill"
            style={{
              width: `${((currentStepIndex + 1) / steps.length) * 100}%`,
            }}
          />
        </div>
        <div className="progress-text">
          {currentStepIndex + 1} / {steps.length}
        </div>
      </div>

      {/* 当前步骤内容 */}
      <div className="step-container">
        <CurrentStepComponent
          onNext={handleNext}
          onPrevious={handlePrevious}
          isFirst={isFirst}
          isLast={isLast}
        />
      </div>

      {/* 步骤指示点 */}
      <div className="step-dots">
        {steps.map((step, index) => (
          <div
            key={step.id}
            className={`step-dot ${index === currentStepIndex ? 'active' : ''} ${
              index < currentStepIndex ? 'completed' : ''
            }`}
            title={step.title}
          />
        ))}
      </div>
    </div>
  );
}
