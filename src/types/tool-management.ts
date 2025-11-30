/**
 * 工具管理系统类型定义
 */

/**
 * 工具环境类型
 */
export enum ToolType {
  /** 本地环境 */
  Local = 'Local',
  /** WSL 环境 */
  WSL = 'WSL',
  /** SSH 远程环境 */
  SSH = 'SSH',
}

/**
 * 工具安装来源
 */
export enum ToolSource {
  /** DuckCoding 管理（安装在 ~/.duckcoding/tool/bin/） */
  DuckCodingManaged = 'DuckCodingManaged',
  /** 外部安装（npm、官方脚本等） */
  External = 'External',
}

/**
 * SSH 连接配置
 */
export interface SSHConfig {
  /** 显示名称（如"开发服务器"、"生产环境"） */
  display_name: string;
  /** 主机地址 */
  host: string;
  /** 端口 */
  port: number;
  /** 用户名 */
  user: string;
  /** SSH 密钥路径（可选） */
  key_path?: string;
}

/**
 * 工具实例（具体环境中的安装）
 */
export interface ToolInstance {
  /** 实例唯一标识（如"claude-code-local", "codex-wsl-Ubuntu"） */
  instance_id: string;
  /** 基础工具ID（claude-code, codex, gemini-cli） */
  base_id: string;
  /** 工具名称（用于显示） */
  tool_name: string;
  /** 环境类型 */
  tool_type: ToolType;
  /** 安装来源 */
  tool_source: ToolSource;
  /** 是否已安装 */
  installed: boolean;
  /** 版本号 */
  version?: string;
  /** 实际安装路径 */
  install_path?: string;
  /** WSL发行版名称（仅WSL类型使用） */
  wsl_distro?: string;
  /** SSH配置（仅SSH类型使用） */
  ssh_config?: SSHConfig;
  /** 是否为内置实例 */
  is_builtin: boolean;
  /** 创建时间（Unix timestamp） */
  created_at: number;
  /** 更新时间（Unix timestamp） */
  updated_at: number;
}

/**
 * 按工具ID分组的实例集合
 */
export interface GroupedToolInstances {
  'claude-code': ToolInstance[];
  codex: ToolInstance[];
  'gemini-cli': ToolInstance[];
}

/**
 * 按环境类型分组的实例集合
 */
export interface GroupedByType {
  local: ToolInstance[];
  wsl: ToolInstance[];
  ssh: ToolInstance[];
}

/**
 * 工具配置（用于UI显示）
 */
export interface ToolConfig {
  id: string;
  name: string;
  icon: string;
}

/**
 * 添加实例请求参数
 */
export interface AddInstanceRequest {
  base_id: string;
  type: 'wsl' | 'ssh';
  distro_name?: string; // WSL发行版名称（仅WSL类型）
  ssh_config?: SSHConfig;
}
