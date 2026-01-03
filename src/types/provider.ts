/**
 * 供应商管理系统类型定义
 */

/**
 * 供应商信息
 */
export interface Provider {
  /** 供应商唯一标识（如 "duckcoding"） */
  id: string;
  /** 供应商名称（用于显示） */
  name: string;
  /** 供应商官网地址 */
  website_url: string;
  /** 用户ID */
  user_id: string;
  /** 访问令牌 */
  access_token: string;
  /** 用户名（可选） */
  username?: string;
  /** 是否为默认供应商 */
  is_default: boolean;
  /** 创建时间（Unix timestamp） */
  created_at: number;
  /** 更新时间（Unix timestamp） */
  updated_at: number;
}

/**
 * 供应商存储结构
 */
export interface ProviderStore {
  /** 数据版本 */
  version: number;
  /** 供应商列表 */
  providers: Provider[];
  /** 当前激活的供应商ID */
  active_provider_id?: string;
  /** 最后更新时间（Unix timestamp） */
  updated_at: number;
}

/**
 * 供应商配置表单数据（暂未使用，保留给 UI 组件）
 */
export interface _ProviderFormData {
  /** 供应商名称 */
  name: string;
  /** 官网地址 */
  website_url: string;
  /** 用户ID */
  user_id: string;
  /** 访问令牌 */
  access_token: string;
  /** 用户名（可选） */
  username?: string;
}

/**
 * 供应商验证结果
 */
export interface ProviderValidationResult {
  /** 是否验证成功 */
  success: boolean;
  /** 从 API 获取的用户名（用于确认身份） */
  username?: string;
  /** 错误消息（验证失败时） */
  error?: string;
}
