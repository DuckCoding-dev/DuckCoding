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
  /** API 地址（可选，优先于 website_url 用于 API 调用） */
  api_address?: string;
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
  /** 签到配置（可选） */
  checkin_config?: CheckinConfig;
}

/**
 * 签到配置
 */
export interface CheckinConfig {
  /** 是否启用自动签到 */
  enabled: boolean;
  /** 签到 API 端点（相对路径，如 "/api/user/checkin"） */
  endpoint: string;
  /** 签到时间范围 - 开始小时（0-23，默认 0） */
  checkin_hour_start?: number;
  /** 签到时间范围 - 结束小时（0-23，默认 0）；start==end 或 start>end 时为全天 */
  checkin_hour_end?: number;
  /** 下次计划签到时间（Unix timestamp），由后端调度器生成 */
  next_checkin_at?: number;
  /** 最后签到时间（Unix timestamp） */
  last_checkin_at?: number;
  /** 最后签到状态 */
  last_checkin_status?: 'success' | 'failed';
  /** 最后签到消息 */
  last_checkin_message?: string;
  /** 累计签到次数 */
  total_checkins?: number;
  /** 累计获得额度 */
  total_quota?: number;
}

/**
 * 签到响应
 */
export interface CheckinResponse {
  success: boolean;
  message?: string;
  data?: {
    quota_awarded?: number;
    checkin_date?: string;
    stats?: {
      checked_in_today: boolean;
      checkin_count: number;
      total_checkins: number;
      total_quota: number;
      records?: Array<{
        checkin_date: string;
        quota_awarded: number;
      }>;
    };
  };
}

/**
 * API 地址信息
 */
export interface ApiInfo {
  /** API 地址 URL */
  url: string;
  /** 地址描述 */
  description: string;
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
