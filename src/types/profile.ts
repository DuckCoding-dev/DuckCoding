/**
 * Profile 管理相关类型定义（v2.1 - 简化版）
 */

// ==================== Profile Payload（前端构建用）====================

/**
 * Claude Profile Payload（前端构建 Profile 时使用）
 */
export interface ClaudeProfilePayload {
  api_key: string;
  base_url: string;
}

/**
 * Codex Profile Payload（前端构建 Profile 时使用）
 */
export interface CodexProfilePayload {
  api_key: string;
  base_url: string;
  wire_api: string; // "responses" 或 "chat"
}

/**
 * Gemini Profile Payload（前端构建 Profile 时使用）
 */
export interface GeminiProfilePayload {
  api_key: string;
  base_url: string;
  model?: string; // 可选，不填则不修改原生配置
}

/**
 * Profile Payload 联合类型
 */
export type ProfilePayload = ClaudeProfilePayload | CodexProfilePayload | GeminiProfilePayload;

/**
 * Profile 完整数据（包含时间戳）
 */
export interface ProfileData {
  api_key: string;
  base_url: string;
  created_at: string; // ISO 8601 时间字符串
  updated_at: string; // ISO 8601 时间字符串
  // 工具特定字段
  provider?: string; // Codex
  model?: string; // Gemini
  raw_settings?: Record<string, unknown>;
  raw_config_json?: Record<string, unknown>;
  raw_config_toml?: string;
  raw_auth_json?: Record<string, unknown>;
  raw_env?: string;
}

/**
 * Profile 描述符（前端展示用）
 */
export interface ProfileDescriptor {
  tool_id: string;
  name: string;
  api_key_preview: string; // 脱敏显示（如 "sk-ant-***xxx"）
  base_url: string;
  created_at: string; // ISO 8601 时间字符串
  updated_at: string; // ISO 8601 时间字符串
  is_active: boolean;
  switched_at?: string; // 激活时间（ISO 8601 时间字符串）
  // Codex 特定字段（注意：后端是 wire_api，前端展示用 provider 兼容）
  wire_api?: string;
  provider?: string; // 向后兼容
  // Gemini 特定字段
  model?: string;
}

/**
 * 工具 ID 类型
 */
export type ToolId = 'claude-code' | 'codex' | 'gemini-cli';

/**
 * 工具显示名称映射
 */
export const TOOL_NAMES: Record<ToolId, string> = {
  'claude-code': 'Claude Code',
  codex: 'CodeX',
  'gemini-cli': 'Gemini CLI',
};

/**
 * 工具颜色映射（用于 UI 区分）
 */
export const TOOL_COLORS: Record<ToolId, string> = {
  'claude-code': 'bg-orange-500',
  codex: 'bg-green-500',
  'gemini-cli': 'bg-blue-500',
};

/**
 * Profile 表单数据
 */
export interface ProfileFormData {
  name: string;
  api_key: string;
  base_url: string;
  // Codex 特定
  wire_api?: string;
  // Gemini 特定
  model?: string;
}

/**
 * Profile 操作类型
 */
export type ProfileOperation = 'create' | 'edit' | 'delete' | 'activate';

/**
 * Profile 分组（按工具）
 */
export interface ProfileGroup {
  tool_id: ToolId;
  tool_name: string;
  profiles: ProfileDescriptor[];
  active_profile?: ProfileDescriptor;
}
