import { invoke } from '@tauri-apps/api/core';
import type { ToolInstance, SSHConfig } from '@/types/tool-management';

export interface ToolStatus {
  mirrorIsStale: boolean;
  mirrorVersion: string | null;
  latestVersion: string | null;
  hasUpdate: boolean;
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
  proxy_bypass_urls?: string[]; // 代理过滤URL列表
  // 透明代理功能 (实验性)
  transparent_proxy_enabled?: boolean;
  transparent_proxy_port?: number;
  transparent_proxy_api_key?: string;
  transparent_proxy_allow_public?: boolean;
  // 保存真实的 API 配置
  transparent_proxy_real_api_key?: string;
  transparent_proxy_real_base_url?: string;
  // 多工具透明代理配置（新架构）
  proxy_configs?: Record<string, ToolProxyConfig>;
  // 会话级端点配置开关（默认关闭）
  session_endpoint_config_enabled?: boolean;
  // 是否隐藏透明代理推荐提示（默认显示）
  hide_transparent_proxy_tip?: boolean;
  // 是否隐藏会话级端点配置提示（默认显示）
  hide_session_config_hint?: boolean;
  // 日志系统配置
  log_config?: LogConfig;
}

export type LogLevel = 'trace' | 'debug' | 'info' | 'warn' | 'error';
export type LogFormat = 'json' | 'text';
export type LogOutput = 'console' | 'file' | 'both';

export interface LogConfig {
  level: LogLevel;
  format: LogFormat;
  output: LogOutput;
  file_path: string | null;
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

export interface UpdateInfo {
  current_version: string;
  latest_version: string;
  has_update: boolean;
  update_url?: string;
  update?: any;
  release_notes?: string;
  file_size?: number;
  required: boolean;
}

export interface DownloadProgress {
  downloaded_bytes: number;
  total_bytes: number;
  percentage: number;
  speed?: number;
  eta?: number;
}

export interface PlatformInfo {
  os: string;
  arch: string;
  is_windows: boolean;
  is_macos: boolean;
  is_linux: boolean;
}

export interface PackageFormatInfo {
  platform: string;
  preferred_formats: string[];
  fallback_format: string;
}

export type CloseAction = 'minimize' | 'quit';

export interface JsonObject {
  [key: string]: JsonValue;
}

export type JsonValue = string | number | boolean | null | JsonObject | JsonValue[];

export type JsonSchema = Record<string, unknown>;

export interface CodexSettingsPayload {
  config: JsonObject;
  authToken: string | null;
}

export interface GeminiEnvConfig {
  apiKey: string;
  baseUrl: string;
  model: string;
}

export interface GeminiSettingsPayload {
  settings: JsonObject;
  env: GeminiEnvConfig;
}

export async function checkInstallations(): Promise<ToolStatus[]> {
  return await invoke<ToolStatus[]>('check_installations');
}

/**
 * 刷新工具状态（清除缓存并重新检测）
 * 用于用户手动刷新或外部安装/卸载工具后更新状态
 */
export async function refreshToolStatus(): Promise<ToolStatus[]> {
  return await invoke<ToolStatus[]>('refresh_tool_status');
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

/**
 * 获取指定配置文件的详情（不激活）
 * @param tool - 工具 ID
 * @param profile - 配置名称
 */
export async function getProfileConfig(tool: string, profile: string): Promise<ActiveConfig> {
  return await invoke<ActiveConfig>('get_profile_config', { tool, profile });
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

export async function getClaudeSettings(): Promise<JsonObject> {
  const data = await invoke<JsonValue>('get_claude_settings');

  if (data && typeof data === 'object' && !Array.isArray(data)) {
    return data as JsonObject;
  }

  return {};
}

export async function saveClaudeSettings(settings: JsonObject): Promise<void> {
  return await invoke<void>('save_claude_settings', { settings });
}

export async function getClaudeSchema(): Promise<JsonSchema> {
  return await invoke<JsonSchema>('get_claude_schema');
}

export async function getCodexSettings(): Promise<CodexSettingsPayload> {
  return await invoke<CodexSettingsPayload>('get_codex_settings');
}

export async function saveCodexSettings(
  settings: JsonObject,
  authToken?: string | null,
): Promise<void> {
  return await invoke<void>('save_codex_settings', { settings, authToken });
}

export async function getCodexSchema(): Promise<JsonSchema> {
  return await invoke<JsonSchema>('get_codex_schema');
}

export async function getGeminiSettings(): Promise<GeminiSettingsPayload> {
  const payload = await invoke<GeminiSettingsPayload>('get_gemini_settings');
  const settings =
    payload.settings && typeof payload.settings === 'object' && !Array.isArray(payload.settings)
      ? (payload.settings as JsonObject)
      : {};
  const env: GeminiEnvConfig = {
    apiKey: payload.env?.apiKey ?? '',
    baseUrl: payload.env?.baseUrl ?? '',
    model: payload.env?.model ?? 'gemini-2.5-pro',
  };

  return {
    settings,
    env,
  };
}

export async function saveGeminiSettings(
  settings: JsonObject,
  env: GeminiEnvConfig,
): Promise<void> {
  return await invoke<void>('save_gemini_settings', { settings, env });
}

export async function getGeminiSchema(): Promise<JsonSchema> {
  return await invoke<JsonSchema>('get_gemini_schema');
}

// 透明代理相关接口和函数

// 单个工具的代理配置
export interface ToolProxyConfig {
  enabled: boolean;
  port: number;
  local_api_key: string | null;
  real_api_key: string | null;
  real_base_url: string | null;
  real_model_provider: string | null; // Codex 专用：备份的 model_provider
  real_profile_name: string | null; // 备份的配置名称
  allow_public: boolean;
  session_endpoint_config_enabled: boolean; // 工具级：是否允许会话自定义端点
  auto_start: boolean; // 应用启动时自动运行代理（默认关闭）
}

export interface TransparentProxyStatus {
  running: boolean;
  port: number;
}

// 多工具代理状态映射
export type AllProxyStatus = Record<string, TransparentProxyStatus>;

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

// ==================== 多工具透明代理 API（新架构） ====================

/**
 * 启动指定工具的透明代理
 * @param toolId - 工具 ID ("claude-code", "codex", "gemini-cli")
 */
export async function startToolProxy(toolId: string): Promise<string> {
  return await invoke<string>('start_tool_proxy', { toolId });
}

/**
 * 停止指定工具的透明代理
 * @param toolId - 工具 ID ("claude-code", "codex", "gemini-cli")
 */
export async function stopToolProxy(toolId: string): Promise<string> {
  return await invoke<string>('stop_tool_proxy', { toolId });
}

/**
 * 获取所有工具的透明代理状态
 * @returns 工具 ID 到状态的映射
 */
export async function getAllProxyStatus(): Promise<AllProxyStatus> {
  return await invoke<AllProxyStatus>('get_all_proxy_status');
}

// 更新管理相关函数
export async function checkForAppUpdates(): Promise<UpdateInfo> {
  return await invoke<UpdateInfo>('check_for_app_updates');
}

export async function downloadAppUpdate(url: string): Promise<string> {
  return await invoke<string>('download_app_update', { url });
}

export async function installAppUpdate(updatePath: string): Promise<void> {
  return await invoke<void>('install_app_update', { updatePath });
}

export async function getAppUpdateStatus(): Promise<string> {
  return await invoke<string>('get_app_update_status');
}

export async function rollbackAppUpdate(): Promise<void> {
  return await invoke<void>('rollback_app_update');
}

export async function getCurrentAppVersion(): Promise<string> {
  return await invoke<string>('get_current_app_version');
}

export async function restartAppForUpdate(): Promise<void> {
  return await invoke<void>('restart_app_for_update');
}

export async function getPlatformInfo(): Promise<PlatformInfo> {
  return await invoke<PlatformInfo>('get_platform_info');
}

export async function getRecommendedPackageFormat(): Promise<PackageFormatInfo> {
  return await invoke<PackageFormatInfo>('get_recommended_package_format');
}

// ==================== 会话管理 API ====================

/**
 * 会话记录（后端数据模型）
 */
export interface SessionRecord {
  session_id: string;
  display_id: string;
  tool_id: string;
  config_name: string;
  /** 自定义配置名称（config_name 为 "custom" 时记录） */
  custom_profile_name: string | null;
  url: string;
  api_key: string;
  /** 会话备注 */
  note: string | null;
  first_seen_at: number;
  last_seen_at: number;
  request_count: number;
  created_at: number;
  updated_at: number;
}

/**
 * 会话列表响应
 */
export interface SessionListResponse {
  sessions: SessionRecord[];
  total: number;
  page: number;
  page_size: number;
}

/**
 * 获取会话列表
 * @param toolId - 工具 ID ("claude-code", "codex", "gemini-cli")
 * @param page - 页码（从 1 开始）
 * @param pageSize - 每页数量
 */
export async function getSessionList(
  toolId: string,
  page: number,
  pageSize: number,
): Promise<SessionListResponse> {
  return await invoke<SessionListResponse>('get_session_list', {
    toolId,
    page,
    pageSize,
  });
}

/**
 * 删除单个会话
 * @param sessionId - 完整的会话 ID
 */
export async function deleteSession(sessionId: string): Promise<void> {
  return await invoke<void>('delete_session', { sessionId });
}

/**
 * 清空指定工具的所有会话
 * @param toolId - 工具 ID
 */
export async function clearAllSessions(toolId: string): Promise<void> {
  return await invoke<void>('clear_all_sessions', { toolId });
}

/**
 * 更新会话配置
 * @param sessionId - 会话 ID
 * @param configName - 配置名称 ("global" 或 "custom")
 * @param customProfileName - 自定义配置名称 (global 时为 null)
 * @param url - API Base URL (global 时为空字符串)
 * @param apiKey - API Key (global 时为空字符串)
 */
export async function updateSessionConfig(
  sessionId: string,
  configName: string,
  customProfileName: string | null,
  url: string,
  apiKey: string,
): Promise<void> {
  return await invoke<void>('update_session_config', {
    sessionId,
    configName,
    customProfileName,
    url,
    apiKey,
  });
}

/**
 * 更新会话备注
 * @param sessionId - 会话 ID
 * @param note - 备注内容 (null 表示清空)
 */
export async function updateSessionNote(sessionId: string, note: string | null): Promise<void> {
  return await invoke<void>('update_session_note', {
    sessionId,
    note,
  });
}

// ==================== 日志配置管理 ====================

/**
 * 检测当前是否为 Release 构建
 */
export async function isReleaseBuild(): Promise<boolean> {
  return await invoke<boolean>('is_release_build');
}

/**
 * 获取当前日志配置
 */
export async function getLogConfig(): Promise<LogConfig> {
  return await invoke<LogConfig>('get_log_config');
}

/**
 * 更新日志配置
 * @param newConfig - 新的日志配置
 * @returns 提示消息，包含是否需要重启的信息
 */
export async function updateLogConfig(newConfig: LogConfig): Promise<string> {
  return await invoke<string>('update_log_config', { newConfig });
}

// ==================== 工具管理系统 ====================

/**
 * 获取所有工具实例（按工具ID分组）
 * @returns 按工具ID分组的实例集合
 */
export async function getToolInstances(): Promise<Record<string, ToolInstance[]>> {
  return await invoke<Record<string, ToolInstance[]>>('get_tool_instances');
}

/**
 * 刷新工具实例状态
 * @returns 刷新后的实例集合
 */
export async function refreshToolInstances(): Promise<Record<string, ToolInstance[]>> {
  return await invoke<Record<string, ToolInstance[]>>('refresh_tool_instances');
}

/**
 * 列出所有可用的WSL发行版
 * @returns WSL发行版名称列表
 */
export async function listWslDistributions(): Promise<string[]> {
  return await invoke<string[]>('list_wsl_distributions');
}

/**
 * 添加WSL工具实例
 * @param baseId - 工具ID（claude-code, codex, gemini-cli）
 * @param distroName - WSL发行版名称
 * @returns 创建的实例
 */
export async function addWslToolInstance(
  baseId: string,
  distroName: string,
): Promise<ToolInstance> {
  return await invoke<ToolInstance>('add_wsl_tool_instance', { baseId, distroName });
}

/**
 * 添加SSH工具实例
 * @param baseId - 工具ID
 * @param sshConfig - SSH连接配置
 * @returns 创建的实例
 */
export async function addSshToolInstance(
  baseId: string,
  sshConfig: SSHConfig,
): Promise<ToolInstance> {
  return await invoke<ToolInstance>('add_ssh_tool_instance', {
    baseId,
    sshConfig,
  });
}

/**
 * 删除工具实例（仅SSH类型）
 * @param instanceId - 实例ID
 */
export async function deleteToolInstance(instanceId: string): Promise<void> {
  return await invoke<void>('delete_tool_instance', { instanceId });
}

/**
 * 检查数据库中是否已有本地工具数据
 * @returns 是否已有本地工具数据
 */
export async function hasToolsInDatabase(): Promise<boolean> {
  return await invoke<boolean>('has_tools_in_database');
}

/**
 * 检测本地工具并保存到数据库（用于新手引导）
 * @returns 检测到的工具实例列表
 */
export async function detectAndSaveTools(): Promise<ToolInstance[]> {
  return await invoke<ToolInstance[]>('detect_and_save_tools');
}
