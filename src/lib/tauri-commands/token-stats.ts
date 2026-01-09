// Token 统计命令模块
// 负责透明代理的 Token 使用统计和请求日志管理

import { invoke } from '@tauri-apps/api/core';
import type {
  SessionStats,
  TokenStatsQuery,
  TokenLogsPage,
  TokenStatsConfig,
  DatabaseSummary,
} from '@/types/token-stats';

/**
 * 查询会话实时统计
 * @param toolType - 工具类型 ("claude-code", "codex", "gemini-cli")
 * @param sessionId - 会话 ID（UUID 格式）
 * @returns 会话统计数据（输入/输出/缓存 Token 数量、请求次数）
 */
export async function getSessionStats(toolType: string, sessionId: string): Promise<SessionStats> {
  return await invoke<SessionStats>('get_session_stats', {
    toolType,
    sessionId,
  });
}

/**
 * 分页查询 Token 日志
 * @param queryParams - 查询参数（工具类型、会话ID、时间范围、分页参数）
 * @returns 分页查询结果（日志列表、总数、分页信息）
 */
export async function queryTokenLogs(queryParams: TokenStatsQuery): Promise<TokenLogsPage> {
  return await invoke<TokenLogsPage>('query_token_logs', {
    queryParams,
  });
}

/**
 * 手动清理旧日志
 * @param retentionDays - 保留天数（可选，未提供则使用配置）
 * @param maxCount - 最大日志条数（可选，未提供则使用配置）
 * @returns 删除的日志条数
 */
export async function cleanupTokenLogs(retentionDays?: number, maxCount?: number): Promise<number> {
  return await invoke<number>('cleanup_token_logs', {
    retentionDays: retentionDays ?? null,
    maxCount: maxCount ?? null,
  });
}

/**
 * 获取数据库统计摘要
 * @returns 数据库摘要信息（总日志数、最早/最新时间戳）
 */
export async function getTokenStatsSummary(): Promise<DatabaseSummary> {
  const result = await invoke<[number, number | null, number | null]>('get_token_stats_summary');
  return {
    total_logs: result[0],
    oldest_timestamp: result[1] ?? undefined,
    newest_timestamp: result[2] ?? undefined,
  };
}

/**
 * 强制执行 WAL checkpoint
 *
 * 将 WAL 文件中的所有数据回写到主数据库文件，
 * 用于手动清理过大的 WAL 文件
 */
export async function forceTokenStatsCheckpoint(): Promise<void> {
  return await invoke<void>('force_token_stats_checkpoint');
}

/**
 * 获取 Token 统计配置
 * @returns Token 统计配置（保留天数、最大条数、自动清理开关）
 */
export async function getTokenStatsConfig(): Promise<TokenStatsConfig> {
  const config = await invoke<{ token_stats_config: TokenStatsConfig }>('get_global_config');
  return config.token_stats_config;
}

/**
 * 更新 Token 统计配置
 * @param config - 新配置（部分字段更新）
 */
export async function updateTokenStatsConfig(config: Partial<TokenStatsConfig>): Promise<void> {
  // 先获取完整配置
  const globalConfig = await invoke<{ token_stats_config: TokenStatsConfig }>('get_global_config');
  const updatedTokenConfig = { ...globalConfig.token_stats_config, ...config };

  // 更新整个全局配置（传递 token_stats_config 字段）
  return await invoke<void>('set_global_config', {
    config: {
      ...globalConfig,
      token_stats_config: updatedTokenConfig,
    },
  });
}
