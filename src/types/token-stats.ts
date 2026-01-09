/**
 * Token 统计系统类型定义
 */

// ==================== 核心数据模型 ====================

/**
 * Token 日志记录
 */
export interface TokenLog {
  id?: number;
  tool_type: string;
  timestamp: number; // Unix 时间戳（毫秒）
  client_ip: string;
  session_id: string;
  config_name: string;
  model: string;
  message_id?: string;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;
  cache_read_tokens: number;
  request_status: 'success' | 'failed'; // 请求状态
  response_type: 'sse' | 'json' | 'unknown'; // 响应类型
  error_type?: 'parse_error' | 'request_interrupted' | 'upstream_error'; // 错误类型
  error_detail?: string; // 错误详情
}

/**
 * 会话统计数据
 */
export interface SessionStats {
  total_input: number;
  total_output: number;
  total_cache_creation: number;
  total_cache_read: number;
  request_count: number;
}

/**
 * Token 日志查询参数
 */
export interface TokenStatsQuery {
  tool_type?: string;
  session_id?: string;
  config_name?: string;
  start_time?: number; // Unix 时间戳（毫秒）
  end_time?: number; // Unix 时间戳（毫秒）
  page: number;
  page_size: number;
}

/**
 * 分页查询结果
 */
export interface TokenLogsPage {
  logs: TokenLog[];
  total: number;
  page: number;
  page_size: number;
}

/**
 * Token 统计配置
 */
export interface TokenStatsConfig {
  retention_days?: number; // 保留天数（可选）
  max_log_count?: number; // 最大日志条数（可选）
  auto_cleanup_enabled: boolean; // 是否启用自动清理
}

// ==================== 前端辅助类型 ====================

/**
 * 工具类型 ID
 */
export type ToolType = 'claude-code' | 'codex' | 'gemini-cli';

/**
 * 工具类型显示名称映射
 */
export const TOOL_TYPE_NAMES: Record<ToolType, string> = {
  'claude-code': 'Claude Code',
  codex: 'CodeX',
  'gemini-cli': 'Gemini CLI',
};

/**
 * 工具类型颜色映射
 */
export const TOOL_TYPE_COLORS: Record<ToolType, string> = {
  'claude-code': 'text-orange-600 bg-orange-50 border-orange-200',
  codex: 'text-green-600 bg-green-50 border-green-200',
  'gemini-cli': 'text-blue-600 bg-blue-50 border-blue-200',
};

/**
 * 请求状态显示名称映射
 */
export const REQUEST_STATUS_NAMES: Record<'success' | 'failed', string> = {
  success: '成功',
  failed: '失败',
};

/**
 * 请求状态颜色映射
 */
export const REQUEST_STATUS_COLORS: Record<'success' | 'failed', string> = {
  success: 'text-green-700 bg-green-50 border-green-200',
  failed: 'text-red-700 bg-red-50 border-red-200',
};

/**
 * 响应类型显示名称映射
 */
export const RESPONSE_TYPE_NAMES: Record<'sse' | 'json' | 'unknown', string> = {
  sse: '流式',
  json: '非流',
  unknown: '未知',
};

/**
 * 错误类型显示名称映射
 */
export const ERROR_TYPE_NAMES: Record<
  'parse_error' | 'request_interrupted' | 'upstream_error',
  string
> = {
  parse_error: '解析失败',
  request_interrupted: '请求中断',
  upstream_error: '上游错误',
};

/**
 * 时间范围快捷选项
 */
export interface TimeRangeOption {
  label: string;
  value: 'today' | 'week' | 'month' | 'all';
  getRange: () => { start_time?: number; end_time?: number };
}

/**
 * Token 使用情况摘要（用于实时展示）
 */
export interface TokenUsageSummary {
  session_id: string;
  tool_type: string;
  stats: SessionStats;
  last_updated: number; // 最后更新时间戳
}

/**
 * 数据库统计摘要
 */
export interface DatabaseSummary {
  total_logs: number;
  oldest_timestamp?: number;
  newest_timestamp?: number;
}

// ==================== 查询过滤器默认值 ====================

/**
 * 默认查询参数
 */
export const DEFAULT_QUERY: Omit<TokenStatsQuery, 'page' | 'page_size'> = {
  tool_type: undefined,
  session_id: undefined,
  config_name: undefined,
  start_time: undefined,
  end_time: undefined,
};

/**
 * 默认分页参数
 */
export const DEFAULT_PAGE_SIZE = 20;

/**
 * 默认 Token 统计配置
 */
export const DEFAULT_TOKEN_STATS_CONFIG: TokenStatsConfig = {
  retention_days: 30,
  max_log_count: 10000,
  auto_cleanup_enabled: true,
};

// ==================== 时间范围快捷选项 ====================

/**
 * 预定义时间范围选项
 */
export const TIME_RANGE_OPTIONS: TimeRangeOption[] = [
  {
    label: '今天',
    value: 'today',
    getRange: () => {
      const now = Date.now();
      const todayStart = new Date(now).setHours(0, 0, 0, 0);
      return { start_time: todayStart, end_time: now };
    },
  },
  {
    label: '最近 7 天',
    value: 'week',
    getRange: () => {
      const now = Date.now();
      const weekAgo = now - 7 * 24 * 60 * 60 * 1000;
      return { start_time: weekAgo, end_time: now };
    },
  },
  {
    label: '最近 30 天',
    value: 'month',
    getRange: () => {
      const now = Date.now();
      const monthAgo = now - 30 * 24 * 60 * 60 * 1000;
      return { start_time: monthAgo, end_time: now };
    },
  },
  {
    label: '全部',
    value: 'all',
    getRange: () => ({
      start_time: undefined,
      end_time: undefined,
    }),
  },
];
