// Profile 管理命令模块
// 负责 Profile 的 CRUD、激活、导入导出、原生配置同步

import { invoke } from '@tauri-apps/api/core';
import type { ProfileData, ProfileDescriptor, ProfilePayload, ToolId } from './types';

// ==================== 旧版 Profile 管理 ====================

/**
 * 列出所有 Profile 描述符（旧版）
 * @deprecated 请使用 pmListAllProfiles
 */
export async function listProfileDescriptors(tool?: string): Promise<ProfileDescriptor[]> {
  return await invoke<ProfileDescriptor[]>('list_profile_descriptors', { tool });
}

// ==================== Profile 管理 v2.0 ====================

/**
 * 列出所有 Profile 描述符
 */
export async function pmListAllProfiles(): Promise<ProfileDescriptor[]> {
  return invoke<ProfileDescriptor[]>('pm_list_all_profiles');
}

/**
 * 列出指定工具的 Profile 名称
 */
export async function pmListToolProfiles(toolId: ToolId): Promise<string[]> {
  return invoke<string[]>('pm_list_tool_profiles', { toolId });
}

/**
 * 获取指定 Profile 的完整数据
 */
export async function pmGetProfile(toolId: ToolId, name: string): Promise<ProfileData> {
  return invoke<ProfileData>('pm_get_profile', { toolId, name });
}

/**
 * 保存 Profile（创建或更新）
 */
export async function pmSaveProfile(
  toolId: ToolId,
  name: string,
  payload: ProfilePayload,
): Promise<void> {
  return invoke<void>('pm_save_profile', { toolId, name, input: payload });
}

/**
 * 删除 Profile
 */
export async function pmDeleteProfile(toolId: ToolId, name: string): Promise<void> {
  return invoke<void>('pm_delete_profile', { toolId, name });
}

/**
 * 激活 Profile（切换）
 */
export async function pmActivateProfile(toolId: ToolId, name: string): Promise<void> {
  return invoke<void>('pm_activate_profile', { toolId, name });
}

/**
 * 获取当前激活的 Profile 名称
 */
export async function pmGetActiveProfileName(toolId: ToolId): Promise<string | null> {
  return invoke<string | null>('pm_get_active_profile_name', { toolId });
}

/**
 * 获取当前激活的 Profile 完整数据
 */
export async function pmGetActiveProfile(toolId: ToolId): Promise<ProfileData | null> {
  return invoke<ProfileData | null>('pm_get_active_profile', { toolId });
}

/**
 * 从原生配置文件捕获并保存为 Profile
 */
export async function pmCaptureFromNative(toolId: ToolId, name: string): Promise<void> {
  return invoke<void>('pm_capture_from_native', { toolId, name });
}

/**
 * 从 Profile 更新代理配置（不激活 Profile）
 */
export async function updateProxyFromProfile(toolId: ToolId, profileName: string): Promise<void> {
  return invoke<void>('update_proxy_from_profile', { toolId, profileName });
}

// ==================== AMP Profile Selection ====================

import type { AmpProfileSelection, ProfileRef } from '@/types/profile';

/**
 * 获取 AMP Profile 选择
 */
export async function pmGetAmpSelection(): Promise<AmpProfileSelection> {
  return invoke<AmpProfileSelection>('pm_get_amp_selection');
}

/**
 * 保存 AMP Profile 选择
 */
export async function pmSaveAmpSelection(input: {
  claude: ProfileRef | null;
  codex: ProfileRef | null;
  gemini: ProfileRef | null;
}): Promise<void> {
  return invoke<void>('pm_save_amp_selection', { input });
}
