/**
 * 签到服务
 * 使用 Tauri 后端 fetchApi 发起 HTTP 请求，绕过浏览器 CORS 限制
 */

import type { Provider, CheckinResponse } from '@/types/provider';
import { fetchApi } from '@/lib/tauri-commands';

/**
 * 检查供应商是否有基本认证信息（可以发起 API 请求）
 */
export function hasProviderAuth(provider: Provider): boolean {
  if (!provider.api_address && !provider.website_url) return false;
  if (!provider.user_id || !provider.access_token) return false;
  return true;
}

/**
 * 检查供应商签到功能是否已启用
 */
export function isCheckinEnabled(provider: Provider): boolean {
  return hasProviderAuth(provider) && !!provider.checkin_config?.enabled;
}

/**
 * 构建签到 API 的完整 URL
 */
function buildCheckinUrl(provider: Provider): string {
  const endpoint = provider.checkin_config?.endpoint || '/api/user/checkin';
  const baseUrl = provider.api_address || provider.website_url;
  return `${baseUrl.replace(/\/$/, '')}${endpoint}`;
}

/**
 * 构建签到请求的认证 headers
 */
function buildCheckinHeaders(provider: Provider): Record<string, string> {
  return {
    Authorization: `Bearer ${provider.access_token}`,
    'New-Api-User': provider.user_id,
    'Content-Type': 'application/json',
  };
}

/**
 * 执行签到
 */
export async function performCheckin(provider: Provider): Promise<CheckinResponse> {
  if (!hasProviderAuth(provider)) {
    return {
      success: false,
      message: '供应商缺少必要的签到配置信息',
    };
  }

  const url = buildCheckinUrl(provider);

  try {
    const data = (await fetchApi(url, 'POST', buildCheckinHeaders(provider), 10000)) as Record<
      string,
      unknown
    >;

    if (typeof data.success !== 'boolean') {
      return {
        success: false,
        message: '供应商返回的数据格式不正确',
      };
    }

    return data as unknown as CheckinResponse;
  } catch (error) {
    const msg = String(error);
    if (msg.includes('404')) {
      return {
        success: false,
        message: '该供应商不支持签到功能 (404)',
      };
    }

    return {
      success: false,
      message: msg || '签到请求失败',
    };
  }
}

/**
 * 获取签到状态
 */
export async function getCheckinStatus(provider: Provider): Promise<CheckinResponse> {
  if (!hasProviderAuth(provider)) {
    return {
      success: false,
      message: '供应商缺少必要的签到配置信息',
    };
  }

  const url = buildCheckinUrl(provider);
  const { 'Content-Type': _, ...getHeaders } = buildCheckinHeaders(provider);

  try {
    const data = (await fetchApi(url, 'GET', getHeaders, 10000)) as Record<string, unknown>;

    if (typeof data.success !== 'boolean') {
      return {
        success: false,
        message: '供应商返回的数据格式不正确',
      };
    }

    return data as unknown as CheckinResponse;
  } catch (error) {
    const msg = String(error);
    if (msg.includes('404')) {
      return {
        success: false,
        message: '该供应商不支持签到功能 (404)',
      };
    }

    return {
      success: false,
      message: msg || '获取签到状态失败',
    };
  }
}
