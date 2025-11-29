import { BalanceResult } from '../types';

/**
 * 执行 extractor 脚本提取余额信息
 * @param response API 原始响应
 * @param extractorScript 提取器 JavaScript 代码
 * @returns 提取的余额结果
 */
export function executeExtractor(response: unknown, extractorScript: string): BalanceResult {
  try {
    // 创建执行环境，只暴露 response
    const func = new Function(
      'response',
      `
      'use strict';
      ${extractorScript}
      return extractor(response);
    `,
    );

    const result = func(response);

    // 验证返回值
    if (typeof result !== 'object' || result === null) {
      throw new Error('提取器必须返回一个对象');
    }

    // 标准化结果
    return {
      planName: result.planName,
      remaining: typeof result.remaining === 'number' ? result.remaining : undefined,
      used: typeof result.used === 'number' ? result.used : undefined,
      total: typeof result.total === 'number' ? result.total : undefined,
      unit: result.unit || 'USD',
      expiresAt: result.expiresAt,
    };
  } catch (error) {
    throw new Error(`提取器执行失败: ${error instanceof Error ? error.message : '未知错误'}`);
  }
}
