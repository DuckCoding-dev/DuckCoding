/**
 * Token 统计分析相关 Tauri 命令
 */
import { invoke } from '@tauri-apps/api/core';
import type { TrendQuery, TrendDataPoint, CostSummary } from '@/types/analytics';

/**
 * 查询 Token 使用趋势数据
 * @param query 查询参数
 * @returns 趋势数据点数组
 */
export async function queryTokenTrends(query: TrendQuery): Promise<TrendDataPoint[]> {
  return await invoke<TrendDataPoint[]>('query_token_trends', { query });
}

/**
 * 查询成本汇总数据
 * @param startTime 开始时间戳（毫秒）
 * @param endTime 结束时间戳（毫秒）
 * @param toolType 工具类型过滤（可选）
 * @param sessionId 会话 ID 过滤（可选）
 * @returns 成本汇总数据
 */
export async function queryCostSummary(
  startTime: number,
  endTime: number,
  toolType?: string,
  sessionId?: string,
): Promise<CostSummary> {
  return await invoke<CostSummary>('query_cost_summary', {
    startTime,
    endTime,
    toolType,
    sessionId,
  });
}
