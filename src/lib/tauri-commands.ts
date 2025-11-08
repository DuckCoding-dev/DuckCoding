import { invoke } from '@tauri-apps/api/core';

export interface ToolStatus {
  id: string;
  name: string;
  installed: boolean;
  version: string | null;
}

export interface InstallResult {
  success: boolean;
  message: string;
  output: string;
}

export interface UpdateResult {
  success: boolean;
  message: string;
  has_update: boolean;
  current_version: string | null;
  latest_version: string | null;
  mirror_version?: string | null; // 镜像实际可安装的版本
  mirror_is_stale?: boolean | null; // 镜像是否滞后
  tool_id?: string;
}

export interface ActiveConfig {
  api_key: string;
  base_url: string;
  profile_name?: string;
}

export interface GlobalConfig {
  user_id: string;
  system_token: string;
}

export interface GenerateApiKeyResult {
  success: boolean;
  message: string;
  api_key: string | null;
}

export interface UsageData {
  id: number;
  user_id: number;
  username: string;
  model_name: string;
  created_at: number;
  token_used: number;
  count: number;
  quota: number;
}

export interface UsageStatsResult {
  success: boolean;
  message: string;
  data: UsageData[];
}

export interface UserQuotaResult {
  success: boolean;
  message: string;
  total_quota: number;
  used_quota: number;
  remaining_quota: number;
  request_count: number;
}

export interface NodeEnvironment {
  node_available: boolean;
  node_version: string | null;
  npm_available: boolean;
  npm_version: string | null;
}

export type CloseAction = 'minimize' | 'quit';

export async function checkInstallations(): Promise<ToolStatus[]> {
  return await invoke<ToolStatus[]>('check_installations');
}

export async function checkNodeEnvironment(): Promise<NodeEnvironment> {
  return await invoke<NodeEnvironment>('check_node_environment');
}

export async function installTool(
  tool: string,
  method: string,
  force?: boolean,
): Promise<InstallResult> {
  return await invoke<InstallResult>('install_tool', { tool, method, force });
}

export async function checkUpdate(tool: string): Promise<UpdateResult> {
  return await invoke<UpdateResult>('check_update', { tool });
}

export async function checkAllUpdates(): Promise<UpdateResult[]> {
  return await invoke<UpdateResult[]>('check_all_updates');
}

export async function updateTool(tool: string, force?: boolean): Promise<UpdateResult> {
  return await invoke<UpdateResult>('update_tool', { tool, force });
}

export async function configureApi(
  tool: string,
  provider: string,
  apiKey: string,
  baseUrl?: string,
  profileName?: string,
): Promise<void> {
  return await invoke<void>('configure_api', {
    tool,
    provider,
    apiKey,
    baseUrl,
    profileName,
  });
}

export async function listProfiles(tool: string): Promise<string[]> {
  return await invoke<string[]>('list_profiles', { tool });
}

export async function switchProfile(tool: string, profile: string): Promise<void> {
  return await invoke<void>('switch_profile', { tool, profile });
}

export async function deleteProfile(tool: string, profile: string): Promise<void> {
  return await invoke<void>('delete_profile', { tool, profile });
}

export async function getActiveConfig(tool: string): Promise<ActiveConfig> {
  return await invoke<ActiveConfig>('get_active_config', { tool });
}

export async function saveGlobalConfig(userId: string, systemToken: string): Promise<void> {
  return await invoke<void>('save_global_config', {
    userId: userId,
    systemToken: systemToken,
  });
}

export async function getGlobalConfig(): Promise<GlobalConfig | null> {
  return await invoke<GlobalConfig | null>('get_global_config');
}

export async function generateApiKeyForTool(tool: string): Promise<GenerateApiKeyResult> {
  return await invoke<GenerateApiKeyResult>('generate_api_key_for_tool', { tool });
}

export async function getUsageStats(): Promise<UsageStatsResult> {
  return await invoke<UsageStatsResult>('get_usage_stats');
}

export async function getUserQuota(): Promise<UserQuotaResult> {
  return await invoke<UserQuotaResult>('get_user_quota');
}

export async function applyCloseAction(action: CloseAction): Promise<void> {
  return await invoke<void>('handle_close_action', { action });
}
