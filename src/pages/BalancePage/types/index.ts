export interface BalanceConfig {
  id: string;
  name: string;
  endpoint: string; // API 端点 URL
  method: 'GET' | 'POST'; // HTTP 方法
  staticHeaders?: Record<string, string>; // 静态请求头（持久化）
  extractorScript: string; // 提取器 JavaScript 代码
  intervalSec?: number; // 0 或 undefined 表示不自动刷新
  timeoutMs?: number; // 请求超时（毫秒）
  updatedAt: number;
  createdAt: number;
}

export interface BalanceRuntimeState {
  loading: boolean;
  error?: string | null;
  lastResult?: BalanceResult | null;
  lastFetchedAt?: number;
}

export type BalanceStateMap = Record<string, BalanceRuntimeState>;

export type ApiKeyMap = Record<string, string>; // configId -> apiKey (用于动态 headers)

export interface StoragePayload {
  version: number;
  configs: BalanceConfig[];
}

export interface BalanceResult {
  planName?: string; // 套餐/计划名称
  remaining?: number; // 剩余额度
  used?: number; // 已用额度
  total?: number; // 总额度
  unit?: string; // 单位 (USD, CNY, etc.)
  expiresAt?: string; // 到期时间
}

export interface BalanceFormValues {
  name: string;
  endpoint: string;
  method: 'GET' | 'POST';
  staticHeaders?: string; // JSON 字符串
  extractorScript: string;
  intervalSec?: number;
  timeoutMs?: number;
  apiKey?: string; // 用于 Authorization header，不持久化
}

// 预设模板类型
export interface BalanceTemplate {
  id: string;
  name: string;
  description: string;
  endpoint: string;
  method: 'GET' | 'POST';
  staticHeaders?: Record<string, string>;
  extractorScript: string;
  requiresApiKey: boolean; // 是否需要 API Key
}
