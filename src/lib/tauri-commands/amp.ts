// AMP Code 用户认证命令模块
// 负责 AMP Code Access Token 验证和用户信息获取

import { invoke } from '@tauri-apps/api/core';
import type { AmpUserInfo } from './types';

/**
 * 通过 AMP Code Access Token 获取用户信息
 * @param accessToken - AMP Code Access Token
 * @returns 用户信息
 */
export async function getAmpUserInfo(accessToken: string): Promise<AmpUserInfo> {
  return await invoke<AmpUserInfo>('get_amp_user_info', { accessToken });
}

/**
 * 验证 AMP Code Access Token 并保存到代理配置
 * @param accessToken - AMP Code Access Token
 * @returns 验证成功后的用户信息
 */
export async function validateAndSaveAmpToken(accessToken: string): Promise<AmpUserInfo> {
  return await invoke<AmpUserInfo>('validate_and_save_amp_token', { accessToken });
}

/**
 * 获取已保存的 AMP Code 用户信息
 * @returns 用户信息（如果已保存且有效）
 */
export async function getSavedAmpUserInfo(): Promise<AmpUserInfo | null> {
  return await invoke<AmpUserInfo | null>('get_saved_amp_user_info');
}
