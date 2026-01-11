/**
 * 时间范围和粒度计算工具函数
 */

import type { TimeRange, TimeGranularity } from '@/types/analytics';

// 时间常量（毫秒）
export const TIME_CONSTANTS = {
  MINUTE_15: 15 * 60 * 1000,
  MINUTE_30: 30 * 60 * 1000,
  HOUR_1: 60 * 60 * 1000,
  HOUR_6: 6 * 60 * 60 * 1000, // 新增：6小时阈值
  HOUR_12: 12 * 60 * 60 * 1000,
  DAY_1: 24 * 60 * 60 * 1000,
  DAY_2: 2 * 24 * 60 * 60 * 1000,
  DAY_7: 7 * 24 * 60 * 60 * 1000,
  DAY_30: 30 * 24 * 60 * 60 * 1000,
  DAY_90: 90 * 24 * 60 * 60 * 1000, // 最大允许跨度
} as const;

// 粒度对应的毫秒值
export const GRANULARITY_MS: Record<TimeGranularity, number> = {
  fifteen_minutes: TIME_CONSTANTS.MINUTE_15,
  thirty_minutes: TIME_CONSTANTS.MINUTE_30,
  hour: TIME_CONSTANTS.HOUR_1,
  twelve_hours: TIME_CONSTANTS.HOUR_12,
  day: TIME_CONSTANTS.DAY_1,
};

// 粒度显示标签
export const GRANULARITY_LABELS: Record<TimeGranularity, string> = {
  fifteen_minutes: '15分钟',
  thirty_minutes: '30分钟',
  hour: '1小时',
  twelve_hours: '12小时',
  day: '1天',
};

// 时间范围显示标签
export const TIME_RANGE_LABELS: Record<TimeRange, string> = {
  fifteen_minutes: '最近15分钟',
  thirty_minutes: '最近30分钟',
  hour: '最近1小时',
  twelve_hours: '最近12小时',
  day: '最近1天',
  week: '最近7天',
  month: '最近30天',
  custom: '自定义',
};

// 预设时间范围的粒度映射
export const PRESET_ALLOWED_GRANULARITIES: Record<
  Exclude<TimeRange, 'custom'>,
  TimeGranularity[]
> = {
  fifteen_minutes: ['fifteen_minutes'],
  thirty_minutes: ['fifteen_minutes', 'thirty_minutes'],
  hour: ['fifteen_minutes', 'thirty_minutes', 'hour'],
  twelve_hours: ['fifteen_minutes', 'thirty_minutes', 'hour', 'twelve_hours'],
  day: ['thirty_minutes', 'hour', 'twelve_hours', 'day'],
  week: ['day'],
  month: ['day'],
};

/**
 * 根据自定义时间跨度计算允许的粒度选项
 * @param startTime - 开始时间戳（毫秒）
 * @param endTime - 结束时间戳（毫秒）
 * @returns 允许的粒度数组
 */
export function calculateAllowedGranularitiesFromTimeSpan(
  startTime: number,
  endTime: number,
): TimeGranularity[] {
  const spanMs = endTime - startTime;

  // 边界检查：无效范围
  if (spanMs <= 0) {
    return [];
  }

  // 应用分段规则
  const { MINUTE_15, MINUTE_30, HOUR_1, HOUR_6, HOUR_12, DAY_2 } = TIME_CONSTANTS;

  if (spanMs <= MINUTE_15) {
    return ['fifteen_minutes'];
  } else if (spanMs <= MINUTE_30) {
    return ['fifteen_minutes', 'thirty_minutes'];
  } else if (spanMs <= HOUR_1) {
    return ['fifteen_minutes', 'thirty_minutes', 'hour'];
  } else if (spanMs <= HOUR_6) {
    // 1小时 < 跨度 <= 6小时：不显示12小时粒度
    return ['fifteen_minutes', 'thirty_minutes', 'hour'];
  } else if (spanMs <= HOUR_12) {
    // 6小时 < 跨度 <= 12小时：显示12小时粒度
    return ['fifteen_minutes', 'thirty_minutes', 'hour', 'twelve_hours'];
  } else if (spanMs <= DAY_2) {
    return ['thirty_minutes', 'hour', 'twelve_hours', 'day'];
  } else {
    return ['day'];
  }
}

/**
 * 选择默认粒度
 * @param allowed - 允许的粒度数组
 * @returns 推荐的默认粒度
 */
export function selectDefaultGranularity(allowed: TimeGranularity[]): TimeGranularity {
  if (allowed.includes('hour')) return 'hour';
  if (allowed.includes('day')) return 'day';
  if (allowed.includes('thirty_minutes')) return 'thirty_minutes';
  if (allowed.includes('twelve_hours')) return 'twelve_hours';
  return allowed[0];
}

/**
 * 根据预设时间范围计算起始时间
 * @param range - 时间范围
 * @param endTime - 结束时间戳（毫秒），默认为当前时间
 * @returns 起始时间戳（毫秒）
 */
export function calculatePresetStartTime(
  range: Exclude<TimeRange, 'custom'>,
  endTime: number = Date.now(),
): number {
  switch (range) {
    case 'fifteen_minutes':
      return endTime - TIME_CONSTANTS.MINUTE_15;
    case 'thirty_minutes':
      return endTime - TIME_CONSTANTS.MINUTE_30;
    case 'hour':
      return endTime - TIME_CONSTANTS.HOUR_1;
    case 'twelve_hours':
      return endTime - TIME_CONSTANTS.HOUR_12;
    case 'day':
      return endTime - TIME_CONSTANTS.DAY_1;
    case 'week':
      return endTime - TIME_CONSTANTS.DAY_7;
    case 'month':
      return endTime - TIME_CONSTANTS.DAY_30;
    default:
      return endTime - TIME_CONSTANTS.DAY_1;
  }
}

/**
 * 验证自定义时间范围是否有效
 * @param startTime - 开始时间
 * @param endTime - 结束时间
 * @returns 验证结果
 */
export function validateCustomTimeRange(
  startTime: Date | null,
  endTime: Date | null,
): { valid: boolean; error?: string } {
  if (!startTime || !endTime) {
    return { valid: false, error: '请选择开始和结束时间' };
  }

  const startMs = startTime.getTime();
  const endMs = endTime.getTime();
  const now = Date.now();

  if (startMs >= endMs) {
    return { valid: false, error: '开始时间必须早于结束时间' };
  }

  if (endMs > now) {
    return { valid: false, error: '结束时间不能晚于当前时刻' };
  }

  const spanMs = endMs - startMs;
  if (spanMs > TIME_CONSTANTS.DAY_90) {
    return { valid: false, error: '时间跨度不能超过90天' };
  }

  return { valid: true };
}

/**
 * 格式化时间范围显示
 * @param startTime - 开始时间戳（毫秒）
 * @param endTime - 结束时间戳（毫秒）
 * @returns 格式化的时间范围字符串
 */
export function formatTimeRangeDisplay(startTime: number, endTime: number): string {
  const start = new Date(startTime);
  const end = new Date(endTime);

  const formatDate = (date: Date) => {
    return date.toLocaleString('zh-CN', {
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  return `${formatDate(start)} ~ ${formatDate(end)}`;
}
