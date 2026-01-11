/**
 * 时间范围控制 Hook
 * 统一管理预设时间范围和自定义时间范围的状态和逻辑
 */

import { useState, useMemo, useCallback } from 'react';
import type { TimeRange, TimeGranularity } from '@/types/analytics';
import {
  PRESET_ALLOWED_GRANULARITIES,
  calculateAllowedGranularitiesFromTimeSpan,
  selectDefaultGranularity,
  calculatePresetStartTime,
} from '@/utils/time-range';

export interface UseTimeRangeControlReturn {
  // 模式控制
  mode: 'preset' | 'custom';
  setMode: (mode: 'preset' | 'custom') => void;

  // 预设时间范围
  presetRange: Exclude<TimeRange, 'custom'>;
  setPresetRange: (range: Exclude<TimeRange, 'custom'>) => void;

  // 自定义时间范围
  customStartTime: Date | null;
  customEndTime: Date | null;
  setCustomStartTime: (date: Date | null) => void;
  setCustomEndTime: (date: Date | null) => void;

  // 粒度控制
  granularity: TimeGranularity;
  setGranularity: (g: TimeGranularity) => void;
  allowedGranularities: TimeGranularity[];

  // 计算后的时间戳
  startTimeMs: number;
  endTimeMs: number;

  // 自定义时间对话框控制
  showCustomDialog: boolean;
  openCustomDialog: () => void;
  closeCustomDialog: () => void;
  confirmCustomTime: () => void;

  // 验证状态
  isCustomTimeValid: boolean;
}

/**
 * 时间范围控制 Hook
 */
export function useTimeRangeControl(): UseTimeRangeControlReturn {
  // 模式：预设或自定义
  const [mode, setMode] = useState<'preset' | 'custom'>('preset');

  // 预设时间范围
  const [presetRange, setPresetRange] = useState<Exclude<TimeRange, 'custom'>>('day');

  // 自定义时间范围（临时状态，确认后才应用）
  const [customStartTime, setCustomStartTime] = useState<Date | null>(null);
  const [customEndTime, setCustomEndTime] = useState<Date | null>(null);

  // 已确认的自定义时间范围
  const [confirmedCustomStart, setConfirmedCustomStart] = useState<Date | null>(null);
  const [confirmedCustomEnd, setConfirmedCustomEnd] = useState<Date | null>(null);

  // 粒度
  const [granularity, setGranularity] = useState<TimeGranularity>('hour');

  // 自定义时间对话框状态
  const [showCustomDialog, setShowCustomDialog] = useState(false);

  // 计算允许的粒度选项
  const allowedGranularities = useMemo(() => {
    if (mode === 'preset') {
      return PRESET_ALLOWED_GRANULARITIES[presetRange];
    } else {
      // 自定义模式：基于已确认的时间范围计算
      if (confirmedCustomStart && confirmedCustomEnd) {
        return calculateAllowedGranularitiesFromTimeSpan(
          confirmedCustomStart.getTime(),
          confirmedCustomEnd.getTime(),
        );
      }
      return ['day']; // 兜底
    }
  }, [mode, presetRange, confirmedCustomStart, confirmedCustomEnd]);

  // 计算实际使用的时间戳
  const { startTimeMs, endTimeMs } = useMemo(() => {
    const now = Date.now();

    if (mode === 'preset') {
      return {
        startTimeMs: calculatePresetStartTime(presetRange, now),
        endTimeMs: now,
      };
    } else {
      // 自定义模式：使用已确认的时间
      if (confirmedCustomStart && confirmedCustomEnd) {
        return {
          startTimeMs: confirmedCustomStart.getTime(),
          endTimeMs: confirmedCustomEnd.getTime(),
        };
      }
      // 兜底：返回最近1天
      return {
        startTimeMs: calculatePresetStartTime('day', now),
        endTimeMs: now,
      };
    }
  }, [mode, presetRange, confirmedCustomStart, confirmedCustomEnd]);

  // 验证自定义时间是否有效
  const isCustomTimeValid = useMemo(() => {
    if (!customStartTime || !customEndTime) return false;

    const startMs = customStartTime.getTime();
    const endMs = customEndTime.getTime();
    const now = Date.now();

    if (startMs >= endMs) return false;
    if (endMs > now) return false;

    const spanMs = endMs - startMs;
    const DAY_90 = 90 * 24 * 60 * 60 * 1000;
    if (spanMs > DAY_90) return false;

    return true;
  }, [customStartTime, customEndTime]);

  // 打开自定义时间对话框
  const openCustomDialog = useCallback(() => {
    // 初始化为已确认的时间，如果没有则使用最近1小时
    if (confirmedCustomStart && confirmedCustomEnd) {
      setCustomStartTime(confirmedCustomStart);
      setCustomEndTime(confirmedCustomEnd);
    } else {
      const now = new Date();
      const oneHourAgo = new Date(now.getTime() - 60 * 60 * 1000);
      setCustomStartTime(oneHourAgo);
      setCustomEndTime(now);
    }
    setShowCustomDialog(true);
  }, [confirmedCustomStart, confirmedCustomEnd]);

  // 关闭对话框
  const closeCustomDialog = useCallback(() => {
    setShowCustomDialog(false);
  }, []);

  // 确认自定义时间
  const confirmCustomTime = useCallback(() => {
    if (!isCustomTimeValid || !customStartTime || !customEndTime) return;

    // 保存已确认的时间
    setConfirmedCustomStart(customStartTime);
    setConfirmedCustomEnd(customEndTime);

    // 切换到自定义模式
    setMode('custom');

    // 计算允许的粒度并选择默认值
    const allowed = calculateAllowedGranularitiesFromTimeSpan(
      customStartTime.getTime(),
      customEndTime.getTime(),
    );
    const defaultGranularity = selectDefaultGranularity(allowed);
    setGranularity(defaultGranularity);

    // 关闭对话框
    setShowCustomDialog(false);
  }, [isCustomTimeValid, customStartTime, customEndTime]);

  // 切换预设范围时自动调整粒度
  const handleSetPresetRange = useCallback((range: Exclude<TimeRange, 'custom'>) => {
    setPresetRange(range);
    setMode('preset');

    // 自动选择默认粒度
    const allowed = PRESET_ALLOWED_GRANULARITIES[range];
    const defaultGranularity = selectDefaultGranularity(allowed);
    setGranularity(defaultGranularity);
  }, []);

  return {
    mode,
    setMode,
    presetRange,
    setPresetRange: handleSetPresetRange,
    customStartTime,
    customEndTime,
    setCustomStartTime,
    setCustomEndTime,
    granularity,
    setGranularity,
    allowedGranularities,
    startTimeMs,
    endTimeMs,
    showCustomDialog,
    openCustomDialog,
    closeCustomDialog,
    confirmCustomTime,
    isCustomTimeValid,
  };
}
