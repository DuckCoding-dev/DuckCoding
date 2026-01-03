// 供应商管理命令模块
// 负责供应商的 CRUD、验证

import { invoke } from '@tauri-apps/api/core';
import type { Provider, _ProviderFormData, ProviderValidationResult } from './types';

/**
 * 列出所有供应商
 */
export async function listProviders(): Promise<Provider[]> {
  return invoke<Provider[]>('list_providers');
}

/**
 * 创建新供应商
 */
export async function createProvider(provider: Provider): Promise<Provider> {
  return invoke<Provider>('create_provider', { provider });
}

/**
 * 更新供应商
 */
export async function updateProvider(id: string, provider: Provider): Promise<Provider> {
  return invoke<Provider>('update_provider', { id, provider });
}

/**
 * 删除供应商
 */
export async function deleteProvider(id: string): Promise<void> {
  return invoke<void>('delete_provider', { id });
}

/**
 * 验证供应商配置（检查 API 连通性，获取用户名）
 */
export async function validateProviderConfig(
  provider: Provider,
): Promise<ProviderValidationResult> {
  try {
    return await invoke<ProviderValidationResult>('validate_provider_config', { provider });
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}
