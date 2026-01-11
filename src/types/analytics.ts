/**
 * Token 统计分析相关类型定义
 */

/**
 * 时间范围粒度
 */
export type TimeRange =
  | 'fifteen_minutes'
  | 'thirty_minutes'
  | 'hour'
  | 'twelve_hours'
  | 'day'
  | 'week'
  | 'month'
  | 'custom'; // 自定义时间范围

/**
 * 时间粒度（与后端 TimeGranularity 对应）
 */
export type TimeGranularity =
  | 'fifteen_minutes'
  | 'thirty_minutes'
  | 'hour'
  | 'twelve_hours'
  | 'day';

/**
 * 趋势查询参数
 */
export interface TrendQuery {
  /** 开始时间戳（毫秒） */
  start_time?: number;
  /** 结束时间戳（毫秒） */
  end_time?: number;
  /** 工具类型过滤（可选） */
  tool_type?: string;
  /** 模型过滤（可选） */
  model?: string;
  /** 配置名称过滤（可选） */
  config_name?: string;
  /** 时间粒度（必需） */
  granularity: TimeGranularity;
}

/**
 * 趋势数据点
 */
export interface TrendDataPoint {
  /** 时间戳（毫秒） */
  timestamp: number;
  /** 输入 Token 总数 */
  input_tokens: number;
  /** 输出 Token 总数 */
  output_tokens: number;
  /** 缓存写入 Token 总数 */
  cache_creation_tokens: number;
  /** 缓存读取 Token 总数 */
  cache_read_tokens: number;
  /** 总成本（USD） */
  total_cost: number;
  /** 输入部分成本（USD） */
  input_price: number;
  /** 输出部分成本（USD） */
  output_price: number;
  /** 缓存写入部分成本（USD） */
  cache_write_price: number;
  /** 缓存读取部分成本（USD） */
  cache_read_price: number;
  /** 请求总数 */
  request_count: number;
  /** 错误请求数 */
  error_count: number;
  /** 平均响应时间（毫秒） */
  avg_response_time: number | null;
}

/**
 * 按模型分组的成本统计
 */
export interface ModelCostStat {
  /** 模型名称 */
  model: string;
  /** 总成本（USD） */
  total_cost: number;
  /** 请求数 */
  request_count: number;
}

/**
 * 按配置分组的成本统计
 */
export interface ConfigCostStat {
  /** 配置名称 */
  config_name: string;
  /** 总成本（USD） */
  total_cost: number;
  /** 请求数 */
  request_count: number;
}

/**
 * 成本汇总数据
 */
export interface CostSummary {
  /** 总成本（USD） */
  total_cost: number;
  /** 总请求数 */
  total_requests: number;
  /** 成功请求数 */
  successful_requests: number;
  /** 失败请求数 */
  failed_requests: number;
  /** 平均响应时间（毫秒） */
  avg_response_time: number | null;
  /** 按模型分组的成本 */
  cost_by_model: ModelCostStat[];
  /** 按配置分组的成本 */
  cost_by_config: ConfigCostStat[];
  /** 按天的成本趋势 */
  daily_costs: Array<{
    /** 日期（时间戳毫秒） */
    date: number;
    /** 总成本（USD） */
    cost: number;
  }>;
}
