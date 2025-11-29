import { BalanceTemplate } from '../types';

/**
 * 预设余额查询模板
 * 提供常见 API 服务商的配置模板
 */
export const BALANCE_TEMPLATES: BalanceTemplate[] = [
  {
    id: 'newapi',
    name: 'NewAPI',
    description: 'NewAPI 中转服务余额查询',
    endpoint: 'https://your-newapi-domain.com/api/user/self',
    method: 'GET',
    staticHeaders: {
      'Content-Type': 'application/json',
    },
    extractorScript: `const extractor = (response) => {
  // NewAPI 响应格式示例
  return {
    planName: response.data?.username || 'Unknown',
    remaining: (response.data?.quota - response.data?.used_quota) / 500000,
    used: response.data?.used_quota / 500000,
    total: response.data?.quota / 500000,
    unit: 'USD',
  };
};`,
    requiresApiKey: true,
  },
  {
    id: 'openai',
    name: 'OpenAI 官方',
    description: 'OpenAI 官方 API 余额查询',
    endpoint: 'https://api.openai.com/dashboard/billing/credit_grants',
    method: 'GET',
    staticHeaders: {
      'Content-Type': 'application/json',
    },
    extractorScript: `const extractor = (response) => {
  // OpenAI 响应格式
  const total = response.total_granted || 0;
  const used = response.total_used || 0;
  const available = response.total_available || 0;

  return {
    planName: 'OpenAI',
    remaining: available,
    used: used,
    total: total || (available + used),
    unit: 'USD',
  };
};`,
    requiresApiKey: true,
  },
  {
    id: 'custom',
    name: '自定义',
    description: '自定义 API 端点和提取器',
    endpoint: '',
    method: 'GET',
    staticHeaders: {},
    extractorScript: `const extractor = (response) => {
  // 根据您的 API 响应格式编写提取逻辑
  // 示例：
  return {
    planName: response.plan?.name || 'Unknown',
    remaining: response.balance?.remaining || 0,
    used: response.balance?.used || 0,
    total: response.balance?.total || 0,
    unit: response.balance?.currency || 'USD',
  };
};`,
    requiresApiKey: false,
  },
];

/**
 * 根据模板 ID 获取模板
 */
export function getTemplateById(id: string): BalanceTemplate | undefined {
  return BALANCE_TEMPLATES.find((t) => t.id === id);
}

/**
 * 获取默认模板（NewAPI）
 */
export function getDefaultTemplate(): BalanceTemplate {
  return BALANCE_TEMPLATES[0];
}
