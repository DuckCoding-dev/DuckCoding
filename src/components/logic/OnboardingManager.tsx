import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, emit } from '@tauri-apps/api/event';
import { useAppContext } from '@/hooks/useAppContext';
import { useToast } from '@/hooks/use-toast';
import OnboardingOverlay from '@/components/Onboarding/OnboardingOverlay';
import {
  getRequiredSteps,
  getAllSteps,
  CURRENT_ONBOARDING_VERSION,
} from '@/components/Onboarding/config/versions';
import type { OnboardingStatus, OnboardingStep } from '@/types/onboarding';
import type { TabType } from '@/contexts/AppContext.types';

export function OnboardingManager() {
  const { setActiveTab, setSettingsInitialTab, setSettingsRestrictToTab, setRestrictedPage } =
    useAppContext();

  const { toast } = useToast();

  const [showOnboarding, setShowOnboarding] = useState(false);
  const [onboardingSteps, setOnboardingSteps] = useState<OnboardingStep[]>([]);
  const [onboardingChecked, setOnboardingChecked] = useState(false);
  const [canExitOnboarding, setCanExitOnboarding] = useState(false);

  // Check Onboarding Status
  useEffect(() => {
    const checkOnboardingStatus = async () => {
      try {
        const status = await invoke<OnboardingStatus | null>('get_onboarding_status');
        const currentVersion = CURRENT_ONBOARDING_VERSION;

        if (!status || !status.completed_version) {
          const steps = getRequiredSteps(null);
          setOnboardingSteps(steps);
          setShowOnboarding(steps.length > 0);
        } else if (status.completed_version < currentVersion) {
          const steps = getRequiredSteps(status.completed_version);
          setOnboardingSteps(steps);
          setShowOnboarding(steps.length > 0);
        }
      } catch (error) {
        console.error('Check onboarding status failed:', error);
      } finally {
        setOnboardingChecked(true);
      }
    };

    checkOnboardingStatus();
  }, []);

  const handleOnboardingComplete = useCallback(() => {
    setShowOnboarding(false);
    toast({
      title: '欢迎使用 DuckCoding',
      description: '您已完成初始配置，现在可以开始使用了',
    });
  }, [toast]);

  const handleShowOnboarding = useCallback(() => {
    const steps = getAllSteps();
    setOnboardingSteps(steps);
    setCanExitOnboarding(true);
    setShowOnboarding(true);
  }, []);

  const handleExitOnboarding = useCallback(() => {
    setShowOnboarding(false);
    setCanExitOnboarding(false);
    toast({
      title: '已退出引导',
      description: '您可以随时从帮助页面重新查看引导',
    });
  }, [toast]);

  // Listen for onboarding events
  useEffect(() => {
    const unlistenRequest = listen('request-show-onboarding', () => {
      handleShowOnboarding();
    });

    const unlistenNavigate = listen<{
      targetPage: string;
      restrictToTab?: string;
      autoAction?: string;
    }>('onboarding-navigate', (event) => {
      const { targetPage, restrictToTab, autoAction } = event.payload || {};

      setRestrictedPage(targetPage);

      if (targetPage === 'settings' && restrictToTab) {
        setSettingsInitialTab(restrictToTab);
        setSettingsRestrictToTab(restrictToTab);
      }

      setActiveTab(targetPage as TabType);

      if (autoAction) {
        setTimeout(() => {
          emit(autoAction);
        }, 500);
      }
    });

    const unlistenClear = listen('clear-onboarding-restriction', () => {
      setRestrictedPage(undefined);
      setSettingsRestrictToTab(undefined);
    });

    return () => {
      unlistenRequest.then((fn) => fn());
      unlistenNavigate.then((fn) => fn());
      unlistenClear.then((fn) => fn());
    };
  }, [
    handleShowOnboarding,
    setActiveTab,
    setRestrictedPage,
    setSettingsInitialTab,
    setSettingsRestrictToTab,
  ]);

  return (
    <>
      {showOnboarding && onboardingChecked && onboardingSteps.length > 0 && (
        <OnboardingOverlay
          steps={onboardingSteps}
          onComplete={handleOnboardingComplete}
          canExit={canExitOnboarding}
          onExit={handleExitOnboarding}
        />
      )}
    </>
  );
}
