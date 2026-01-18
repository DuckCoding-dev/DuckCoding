// Token Management Tauri Commands
//
// NEW API 令牌管理 Tauri 命令包装器

import { invoke } from '@tauri-apps/api/core';
import type { Provider } from '@/types/provider';
import type {
  CreateRemoteTokenRequest,
  RemoteToken,
  RemoteTokenGroup,
  TokenImportStatus,
  TokenListResponse,
  UpdateRemoteTokenRequest,
} from '@/types/remote-token';

/**
 * 获取指定供应商的远程令牌列表（支持分页）
 */
export async function fetchProviderTokens(
  provider: Provider,
  page = 1,
  pageSize = 10,
): Promise<TokenListResponse> {
  return invoke<TokenListResponse>('fetch_provider_tokens', { provider, page, pageSize });
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
 * 更新供应商的远程令牌（完整版本，支持所有字段）
 */
export async function updateProviderTokenFull(
  provider: Provider,
  tokenId: number,
  request: UpdateRemoteTokenRequest,
): Promise<RemoteToken> {
  return invoke<RemoteToken>('update_provider_token_full', { provider, tokenId, request });
}

/**
 * 导入远程令牌为本地 Profile
 */
export async function importTokenAsProfile(
  provider: Provider,
  remoteToken: RemoteToken,
  toolId: string,
  profileName: string,
  pricingTemplateId?: string, // 🆕 Phase 6: 可选的价格模板 ID
): Promise<void> {
  return invoke<void>('import_token_as_profile', {
    profileManager: null, // Managed by Tauri State
    provider,
    remoteToken,
    toolId,
    profileName,
    pricingTemplateId: pricingTemplateId || null,
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
  extraConfig?: { wire_api?: string; model?: string; pricing_template_id?: string },
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

/**
 * 检测令牌是否已导入到任何工具
 */
export async function checkTokenImportStatus(
  providerId: string,
  remoteTokenId: number,
): Promise<TokenImportStatus[]> {
  return invoke<TokenImportStatus[]>('check_token_import_status', {
    profileManager: null, // Managed by Tauri State
    providerId,
    remoteTokenId,
  });
}
