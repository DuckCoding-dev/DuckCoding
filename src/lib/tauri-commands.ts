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
  proxy_enabled?: boolean;
  proxy_type?: 'http' | 'https' | 'socks5';
  proxy_host?: string;
  proxy_port?: string;
  proxy_username?: string;
  proxy_password?: string;
  // 透明代理功能 (实验性)
  transparent_proxy_enabled?: boolean;
  transparent_proxy_port?: number;
  transparent_proxy_api_key?: string;
  transparent_proxy_allow_public?: boolean;
  // 保存真实的 API 配置
  transparent_proxy_real_api_key?: string;
  transparent_proxy_real_base_url?: string;
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

export async function saveGlobalConfig(config: GlobalConfig): Promise<void> {
  return await invoke<void>('save_global_config', { config });
}

export async function getGlobalConfig(): Promise<GlobalConfig | null> {
  return await invoke<GlobalConfig | null>('get_global_config');
}

export async function getCurrentProxy(): Promise<string | null> {
  return await invoke<string | null>('get_current_proxy');
}

export async function applyProxyNow(): Promise<string | null> {
  return await invoke<string | null>('apply_proxy_now');
}

export interface TestProxyResult {
  success: boolean;
  status: number;
  url?: string | null;
  error?: string | null;
}

export interface ProxyTestConfig {
  enabled: boolean;
  proxy_type: string;
  host: string;
  port: string;
  username?: string;
  password?: string;
}

export async function testProxyRequest(
  testUrl: string,
  proxyConfig: ProxyTestConfig,
): Promise<TestProxyResult> {
  return await invoke<TestProxyResult>('test_proxy_request', { testUrl, proxyConfig });
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

// 透明代理相关接口和函数
export interface TransparentProxyStatus {
  running: boolean;
  port: number;
}

export async function startTransparentProxy(): Promise<string> {
  return await invoke<string>('start_transparent_proxy');
}

export async function stopTransparentProxy(): Promise<string> {
  return await invoke<string>('stop_transparent_proxy');
}

export async function getTransparentProxyStatus(): Promise<TransparentProxyStatus> {
  return await invoke<TransparentProxyStatus>('get_transparent_proxy_status');
}

export async function updateTransparentProxyConfig(
  newApiKey: string,
  newBaseUrl: string,
): Promise<string> {
  return await invoke<string>('update_transparent_proxy_config', {
    newApiKey,
    newBaseUrl,
  });
}
