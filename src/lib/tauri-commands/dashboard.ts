// Dashboard 管理命令模块
// 负责仪表板状态管理：工具实例选择、选中供应商 Tab

import { invoke } from '@tauri-apps/api/core';

/**
 * 获取工具实例选择
 * @param toolId 工具 ID（"claude-code" | "codex" | "gemini-cli"）
 * @returns 实例 ID（如 "claude-code-local"）或 null
 */
export async function getToolInstanceSelection(toolId: string): Promise<string | null> {
  return invoke<string | null>('get_tool_instance_selection', { toolId });
}

/**
 * 设置工具实例选择
 * @param toolId 工具 ID
 * @param instanceId 实例 ID
 */
export async function setToolInstanceSelection(toolId: string, instanceId: string): Promise<void> {
  return invoke<void>('set_tool_instance_selection', { toolId, instanceId });
}

/**
 * 获取最后选中的供应商 ID
 * @returns 供应商 ID 或 null
 */
export async function getSelectedProviderId(): Promise<string | null> {
  return invoke<string | null>('get_selected_provider_id');
}

/**
 * 设置最后选中的供应商 ID
 * @param providerId 供应商 ID（传 null 表示清除）
 */
export async function setSelectedProviderId(providerId: string | null): Promise<void> {
  return invoke<void>('set_selected_provider_id', { providerId });
}
