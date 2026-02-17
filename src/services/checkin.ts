/**
 * 签到服务
 */

import type { Provider, CheckinResponse } from '@/types/provider';

/**
 * 检查供应商是否支持签到
 */
export function isCheckinSupported(provider: Provider): boolean {
  // 必须有 API 地址和认证信息
  if (!provider.api_address && !provider.website_url) return false;
  if (!provider.user_id || !provider.access_token) return false;
  return true;
}

/**
 * 执行签到
 */
export async function performCheckin(provider: Provider): Promise<CheckinResponse> {
  if (!isCheckinSupported(provider)) {
    return {
      success: false,
      message: '供应商缺少必要的签到配置信息',
    };
  }

  const endpoint = provider.checkin_config?.endpoint || '/api/user/checkin';
  const baseUrl = provider.api_address || provider.website_url;
  const url = `${baseUrl.replace(/\/$/, '')}${endpoint}`;

  try {
    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${provider.access_token}`,
        'New-Api-User': provider.user_id,
        'Content-Type': 'application/json',
      },
    });

    // 检查 HTTP 状态码
    if (!response.ok) {
      // 404 可能表示不支持签到功能
      if (response.status === 404) {
        return {
          success: false,
          message: '该供应商不支持签到功能 (404)',
        };
      }
      
      // 尝试解析错误响应
      try {
        const errorData = await response.json();
        return {
          success: false,
          message: errorData.message || `请求失败 (${response.status})`,
        };
      } catch {
        return {
          success: false,
          message: `请求失败 (${response.status})`,
        };
      }
    }

    const data = await response.json();
    
    // 检查响应格式
    if (typeof data.success !== 'boolean') {
      return {
        success: false,
        message: '供应商返回的数据格式不正确',
      };
    }

    return data as CheckinResponse;
  } catch (error) {
    // 网络错误或其他异常
    if (error instanceof TypeError && error.message.includes('fetch')) {
      return {
        success: false,
        message: '网络连接失败，请检查供应商地址是否正确',
      };
    }
    
    return {
      success: false,
      message: error instanceof Error ? error.message : '签到请求失败',
    };
  }
}

/**
 * 获取签到状态
 */
export async function getCheckinStatus(provider: Provider): Promise<CheckinResponse> {
  if (!isCheckinSupported(provider)) {
    return {
      success: false,
      message: '供应商缺少必要的签到配置信息',
    };
  }

  const endpoint = provider.checkin_config?.endpoint || '/api/user/checkin';
  const baseUrl = provider.api_address || provider.website_url;
  const url = `${baseUrl.replace(/\/$/, '')}${endpoint}`;

  try {
    const response = await fetch(url, {
      method: 'GET',
      headers: {
        'Authorization': `Bearer ${provider.access_token}`,
        'New-Api-User': provider.user_id,
      },
    });

    // 检查 HTTP 状态码
    if (!response.ok) {
      // 404 可能表示不支持签到功能
      if (response.status === 404) {
        return {
          success: false,
          message: '该供应商不支持签到功能',
        };
      }
      
      // 尝试解析错误响应
      try {
        const errorData = await response.json();
        return {
          success: false,
          message: errorData.message || `请求失败 (${response.status})`,
        };
      } catch {
        return {
          success: false,
          message: `请求失败 (${response.status})`,
        };
      }
    }

    const data = await response.json();
    
    // 检查响应格式
    if (typeof data.success !== 'boolean') {
      return {
        success: false,
        message: '供应商返回的数据格式不正确',
      };
    }

    return data as CheckinResponse;
  } catch (error) {
    // 网络错误或其他异常
    if (error instanceof TypeError && error.message.includes('fetch')) {
      return {
        success: false,
        message: '网络连接失败，请检查供应商地址是否正确',
      };
    }
    
    return {
      success: false,
      message: error instanceof Error ? error.message : '获取签到状态失败',
    };
  }
}
