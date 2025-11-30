// filepath: e:\DuckCoding\src\types\onboarding.ts

import { ComponentType } from 'react';

/**
 * 引导状态（与 Rust 后端对应）
 */
export interface OnboardingStatus {
  /** 已完成的引导版本（例如："v1", "v2"） */
  completed_version: string;
  /** 跳过的步骤 ID 列表 */
  skipped_steps: string[];
  /** 完成时间戳（ISO 8601 格式） */
  completed_at?: string;
}

/**
 * 引导步骤定义
 */
export interface OnboardingStep {
  /** 步骤唯一标识符 */
  id: string;
  /** 步骤标题 */
  title: string;
  /** 步骤描述（可选） */
  description?: string;
  /** 步骤对应的 React 组件 */
  component: ComponentType<StepProps>;
  /** 是否允许跳过此步骤（默认 false） */
  skippable?: boolean;
}

/**
 * 步骤组件接收的 Props
 */
export interface StepProps {
  /** 进入下一步的回调函数 */
  onNext: (skipped?: boolean) => void;
  /** 返回上一步的回调函数 */
  onPrevious: () => void;
  /** 是否为第一步 */
  isFirst: boolean;
  /** 是否为最后一步 */
  isLast: boolean;
}

/**
 * 引导版本配置映射类型
 */
export type VersionStepsMap = Record<string, OnboardingStep[]>;
