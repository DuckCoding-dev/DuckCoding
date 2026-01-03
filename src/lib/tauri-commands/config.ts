// 配置管理命令模块
// 负责全局配置、工具配置、代理配置、外部变更监听等功能

import { invoke } from '@tauri-apps/api/core';
import type {
  GlobalConfig,
  ClaudeSettingsPayload,
  CodexSettingsPayload,
  GeminiSettingsPayload,
  GeminiEnvConfig,
  JsonObject,
  JsonSchema,
  JsonValue,
  TestProxyResult,
  ProxyTestConfig,
  ExternalConfigChange,
  ImportExternalChangeResult,
} from './types';

// ==================== 全局配置 ====================

/**
 * 保存全局配置
 */
export async function saveGlobalConfig(config: GlobalConfig): Promise<void> {
  return await invoke<void>('save_global_config', { config });
}

/**
 * 获取全局配置
 */
export async function getGlobalConfig(): Promise<GlobalConfig | null> {
  return await invoke<GlobalConfig | null>('get_global_config');
}

/**
 * 获取当前代理配置字符串
 */
export async function getCurrentProxy(): Promise<string | null> {
  return await invoke<string | null>('get_current_proxy');
}

/**
 * 立即应用代理配置
 */
export async function applyProxyNow(): Promise<string | null> {
  return await invoke<string | null>('apply_proxy_now');
}

/**
 * 测试代理连接
 */
export async function testProxyRequest(
  testUrl: string,
  proxyConfig: ProxyTestConfig,
): Promise<TestProxyResult> {
  return await invoke<TestProxyResult>('test_proxy_request', { testUrl, proxyConfig });
}

// ==================== Claude Code 配置 ====================

/**
 * 获取 Claude Code 配置
 */
export async function getClaudeSettings(): Promise<ClaudeSettingsPayload> {
  const data = await invoke<JsonValue>('get_claude_settings');

  if (data && typeof data === 'object' && !Array.isArray(data)) {
    const payload = data as Record<string, unknown>;
    const settings =
      payload.settings && typeof payload.settings === 'object' && !Array.isArray(payload.settings)
        ? (payload.settings as JsonObject)
        : {};
    const extraConfig =
      payload.extraConfig &&
      typeof payload.extraConfig === 'object' &&
      !Array.isArray(payload.extraConfig)
        ? (payload.extraConfig as JsonObject)
        : null;
    return { settings, extraConfig };
  }

  return { settings: {}, extraConfig: null };
}

/**
 * 保存 Claude Code 配置
 */
export async function saveClaudeSettings(
  settings: JsonObject,
  extraConfig?: JsonObject | null,
): Promise<void> {
  const payload: Record<string, unknown> = { settings };
  if (extraConfig !== undefined) {
    payload.extraConfig = extraConfig;
  }
  return await invoke<void>('save_claude_settings', payload);
}

/**
 * 获取 Claude Code 配置 Schema
 */
export async function getClaudeSchema(): Promise<JsonSchema> {
  return await invoke<JsonSchema>('get_claude_schema');
}

// ==================== Codex 配置 ====================

/**
 * 获取 Codex 配置
 */
export async function getCodexSettings(): Promise<CodexSettingsPayload> {
  return await invoke<CodexSettingsPayload>('get_codex_settings');
}

/**
 * 保存 Codex 配置
 */
export async function saveCodexSettings(
  settings: JsonObject,
  authToken?: string | null,
): Promise<void> {
  return await invoke<void>('save_codex_settings', { settings, authToken });
}

/**
 * 获取 Codex 配置 Schema
 */
export async function getCodexSchema(): Promise<JsonSchema> {
  return await invoke<JsonSchema>('get_codex_schema');
}

// ==================== Gemini CLI 配置 ====================

/**
 * 获取 Gemini CLI 配置
 */
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

/**
 * 保存 Gemini CLI 配置
 */
export async function saveGeminiSettings(
  settings: JsonObject,
  env: GeminiEnvConfig,
): Promise<void> {
  return await invoke<void>('save_gemini_settings', { settings, env });
}

/**
 * 获取 Gemini CLI 配置 Schema
 */
export async function getGeminiSchema(): Promise<JsonSchema> {
  return await invoke<JsonSchema>('get_gemini_schema');
}

// ==================== 配置监听 ====================

/**
 * 获取配置监听器状态
 */
export async function getWatcherStatus(): Promise<boolean> {
  return await invoke<boolean>('get_watcher_status');
}

/**
 * 启动配置监听器（如果需要）
 */
export async function startWatcherIfNeeded(): Promise<boolean> {
  return await invoke<boolean>('start_watcher_if_needed');
}

/**
 * 停止配置监听器
 */
export async function stopWatcher(): Promise<boolean> {
  return await invoke<boolean>('stop_watcher');
}

/**
 * 保存配置监听器设置
 */
export async function saveWatcherSettings(
  enabled: boolean,
  pollIntervalMs?: number,
): Promise<void> {
  await invoke<void>('save_watcher_settings', {
    enabled,
    pollIntervalMs,
  });
}

/**
 * 获取外部配置变更列表
 */
export async function getExternalChanges(): Promise<ExternalConfigChange[]> {
  return await invoke<ExternalConfigChange[]>('get_external_changes');
}

/**
 * 确认外部配置变更
 */
export async function ackExternalChange(tool: string): Promise<void> {
  return await invoke<void>('ack_external_change', { tool });
}

/**
 * 导入原生配置变更为 Profile
 */
export async function importNativeChange(
  tool: string,
  profile: string,
  asNew: boolean,
): Promise<ImportExternalChangeResult> {
  return await invoke<ImportExternalChangeResult>('import_native_change', {
    tool,
    profile,
    asNew,
  });
}

// ==================== 单实例模式配置 ====================

/**
 * 获取单实例模式配置状态
 * @returns 单实例模式是否启用
 */
export async function getSingleInstanceConfig(): Promise<boolean> {
  return await invoke<boolean>('get_single_instance_config');
}

/**
 * 更新单实例模式配置（需要重启应用生效）
 * @param enabled - 是否启用单实例模式
 */
export async function updateSingleInstanceConfig(enabled: boolean): Promise<void> {
  return await invoke<void>('update_single_instance_config', { enabled });
}

// ==================== 开机自启动配置 ====================

/**
 * 获取开机自启动配置状态
 * @returns 开机自启动是否启用
 */
export async function getStartupConfig(): Promise<boolean> {
  return await invoke<boolean>('get_startup_config');
}

/**
 * 更新开机自启动配置
 * @param enabled - 是否启用开机自启动
 */
export async function updateStartupConfig(enabled: boolean): Promise<void> {
  return await invoke<void>('update_startup_config', { enabled });
}
