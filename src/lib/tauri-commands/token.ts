// Token Management Tauri Commands
//
// NEW API 令牌管理 Tauri 命令包装器

import { invoke } from '@tauri-apps/api/core';
import type { Provider } from '@/types/provider';
import type { CreateRemoteTokenRequest, RemoteToken, RemoteTokenGroup } from '@/types/remote-token';

/**
 * 获取指定供应商的远程令牌列表
 */
export async function fetchProviderTokens(provider: Provider): Promise<RemoteToken[]> {
  return invoke<RemoteToken[]>('fetch_provider_tokens', { provider });
}

/**
 * 获取指定供应商的令牌分组列表
 */
export async function fetchProviderGroups(provider: Provider): Promise<RemoteTokenGroup[]> {
  return invoke<RemoteTokenGroup[]>('fetch_provider_groups', { provider });
}

/**
 * 在供应商创建新的远程令牌（仅返回成功状态）
 */
export async function createProviderToken(
  provider: Provider,
  request: CreateRemoteTokenRequest,
): Promise<void> {
  return invoke<void>('create_provider_token', { provider, request });
}

/**
 * 删除供应商的远程令牌
 */
export async function deleteProviderToken(provider: Provider, tokenId: number): Promise<void> {
  return invoke<void>('delete_provider_token', { provider, tokenId });
}

/**
 * 更新供应商的远程令牌名称
 */
export async function updateProviderToken(
  provider: Provider,
  tokenId: number,
  name: string,
): Promise<RemoteToken> {
  return invoke<RemoteToken>('update_provider_token', { provider, tokenId, name });
}

/**
 * 导入远程令牌为本地 Profile
 */
export async function importTokenAsProfile(
  provider: Provider,
  remoteToken: RemoteToken,
  toolId: string,
  profileName: string,
): Promise<void> {
  return invoke<void>('import_token_as_profile', {
    profileManager: null, // Managed by Tauri State
    provider,
    remoteToken,
    toolId,
    profileName,
  });
}

/**
 * 创建自定义 Profile（非导入令牌）
 */
export async function createCustomProfile(
  toolId: string,
  profileName: string,
  apiKey: string,
  baseUrl: string,
  extraConfig?: { wire_api?: string; model?: string },
): Promise<void> {
  return invoke<void>('create_custom_profile', {
    profileManager: null, // Managed by Tauri State
    toolId,
    profileName,
    apiKey,
    baseUrl,
    extraConfig: extraConfig || null,
  });
}
