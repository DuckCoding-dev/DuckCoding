// 类型定义模块
// 集中管理所有 Tauri 命令相关的类型定义，避免循环依赖

import type { SSHConfig } from '@/types/tool-management';
import type { ProfileData, ProfileDescriptor, ProfilePayload, ToolId } from '@/types/profile';
import type {
  Provider,
  ProviderStore,
  _ProviderFormData,
  ProviderValidationResult,
  ApiInfo,
} from '@/types/provider';

// 重新导出 Profile 相关类型供其他模块使用
export type { ProfileData, ProfileDescriptor, ProfilePayload, ToolId };

// 重新导出工具管理类型
export type { SSHConfig };

// 重新导出供应商管理类型
export type { Provider, ProviderStore, _ProviderFormData, ProviderValidationResult, ApiInfo };

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
  user_id?: string; // 已废弃，由供应商系统管理
  system_token?: string; // 已废弃，由供应商系统管理
  proxy_enabled?: boolean;
  proxy_type?: 'http' | 'https' | 'socks5';
  proxy_host?: string;
  proxy_port?: string;
  proxy_username?: string;
  proxy_password?: string;
  proxy_bypass_urls?: string[]; // 代理过滤URL列表
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
  // 配置监听
  external_watch_enabled?: boolean;
  external_poll_interval_ms?: number;
  // 单实例模式开关（默认 true，仅生产环境生效）
  single_instance_enabled?: boolean;
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

export interface ClaudeSettingsPayload {
  settings: JsonObject;
  extraConfig?: JsonObject | null;
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
  tavily_api_key?: string | null; // Tavily API Key（用于本地搜索，可选）
}

export interface TransparentProxyStatus {
  running: boolean;
  port: number;
}

// 多工具代理状态映射
export type AllProxyStatus = Record<string, TransparentProxyStatus>;

// 会话记录（后端数据模型）
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

// 会话列表响应
export interface SessionListResponse {
  sessions: SessionRecord[];
  total: number;
  page: number;
  page_size: number;
}

// 工具候选结果
export interface ToolCandidate {
  tool_path: string;
  installer_path: string | null;
  install_method: string; // "Npm" | "Brew" | "Official" | "Other"
  version: string;
}

// 安装器候选结果
export interface InstallerCandidate {
  path: string;
  installer_type: string; // "Npm" | "Brew" | "Official" | "Other"
  level: number; // 1=同级目录, 2=上级目录
}

// 余额监控存储结构（后端返回）
export interface BalanceStore {
  version: number;
  configs: BalanceConfigBackend[];
}

// 后端 BalanceConfig 格式（snake_case）
export interface BalanceConfigBackend {
  id: string;
  name: string;
  endpoint: string;
  method: 'GET' | 'POST';
  static_headers?: Record<string, string>;
  extractor_script: string;
  interval_sec?: number;
  timeout_ms?: number;
  save_api_key: boolean;
  api_key?: string;
  created_at: number;
  updated_at: number;
}

// 前端 BalanceConfig 格式（camelCase）- 从 BalancePage 导入
export type { BalanceConfig } from '@/pages/BalancePage/types';

// AMP 用户信息
export interface AmpUserInfo {
  id: string;
  email: string | null;
  name: string | null;
  username: string | null;
}
