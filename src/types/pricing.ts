/**
 * 价格配置相关类型定义（Phase 6）
 *
 * 与 Rust 后端模型完全对应
 */

// ==================== 核心类型定义 ====================

/**
 * 单个模型的价格定义
 */
export interface ModelPrice {
  /** 提供商（如：anthropic、openai） */
  provider: string;
  /** 输入价格（USD/百万 Token） */
  input_price_per_1m: number;
  /** 输出价格（USD/百万 Token） */
  output_price_per_1m: number;
  /** 缓存写入价格 - 5分钟TTL（USD/百万 Token，可选） */
  cache_write_price_per_1m?: number;
  /** 缓存写入价格 - 1小时TTL（USD/百万 Token，可选） */
  cache_write_1h_price_per_1m?: number;
  /** 缓存读取价格（USD/百万 Token，可选） */
  cache_read_price_per_1m?: number;
  /** 货币类型（默认：USD） */
  currency: string;
  /** 模型别名列表（支持多种 ID 格式） */
  aliases: string[];
}

/**
 * 单个模型的继承配置
 */
export interface InheritedModel {
  /** 模型名称（如："claude-sonnet-4.5"） */
  model_name: string;
  /** 从哪个模板继承（如："claude_official_2025_01"） */
  source_template_id: string;
  /** 倍率（应用到继承的价格上） */
  multiplier: number;
}

/**
 * 价格模板（统一结构，支持三种模式）
 */
export interface PricingTemplate {
  // 基础信息
  /** 模板 ID（唯一标识） */
  id: string;
  /** 模板名称 */
  name: string;
  /** 模板描述 */
  description: string;
  /** 模板版本 */
  version: string;
  /** 创建时间（Unix 时间戳，毫秒） */
  created_at: number;
  /** 更新时间（Unix 时间戳，毫秒） */
  updated_at: number;

  // 继承配置
  /** 继承配置列表（每个模型独立配置，可从不同模板继承） */
  inherited_models: InheritedModel[];

  // 自定义模型
  /** 自定义模型（直接定义价格） */
  custom_models: Record<string, ModelPrice>;

  // 元数据
  /** 标签列表（用于分类和搜索） */
  tags: string[];
  /** 是否为内置预设模板 */
  is_default_preset: boolean;
}

// ==================== 工具 ID 类型 ====================

/**
 * 工具 ID 类型（用于默认模板管理）
 */
export type PricingToolId = 'claude-code' | 'codex' | 'gemini-cli';

/**
 * 工具显示名称映射
 */
export const TOOL_NAMES: Record<PricingToolId, string> = {
  'claude-code': 'Claude Code',
  codex: 'Codex',
  'gemini-cli': 'Gemini CLI',
};

// ==================== 表单数据类型 ====================

/**
 * 模板模式类型
 */
export type TemplateMode = 'custom' | 'inherited' | 'mixed';

/**
 * 价格模板表单数据（用于创建/编辑）
 */
export interface PricingTemplateFormData {
  /** 模板名称 */
  name: string;
  /** 模板描述 */
  description: string;
  /** 模式选择：custom（完全自定义） | inherited（纯继承） | mixed（混合） */
  mode: TemplateMode;
  /** 继承配置列表 */
  inherited_models: InheritedModel[];
  /** 自定义模型列表 */
  custom_models: Record<string, ModelPrice>;
  /** 标签列表 */
  tags: string[];
}

// ==================== 辅助函数 ====================

/**
 * 判断模板模式
 */
export function getTemplateMode(template: PricingTemplate): TemplateMode {
  const hasInherited = template.inherited_models.length > 0;
  const hasCustom = Object.keys(template.custom_models).length > 0;

  if (hasInherited && hasCustom) return 'mixed';
  if (hasInherited) return 'inherited';
  return 'custom';
}

/**
 * 获取模板模式显示名称
 */
export function getTemplateModeName(mode: TemplateMode): string {
  switch (mode) {
    case 'custom':
      return '完全自定义';
    case 'inherited':
      return '继承模式';
    case 'mixed':
      return '混合模式';
    default:
      return '未知模式';
  }
}

/**
 * 获取模板总模型数
 */
export function getTotalModelCount(template: PricingTemplate): number {
  return template.inherited_models.length + Object.keys(template.custom_models).length;
}

/**
 * 格式化时间戳为本地时间字符串
 */
export function formatTemplateTimestamp(timestamp: number): string {
  return new Date(timestamp).toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  });
}

/**
 * 创建空的价格模板表单数据
 */
export function createEmptyTemplateFormData(): PricingTemplateFormData {
  return {
    name: '',
    description: '',
    mode: 'custom',
    inherited_models: [],
    custom_models: {},
    tags: [],
  };
}

/**
 * 将模板转换为表单数据
 */
export function templateToFormData(template: PricingTemplate): PricingTemplateFormData {
  return {
    name: template.name,
    description: template.description,
    mode: getTemplateMode(template),
    inherited_models: template.inherited_models,
    custom_models: template.custom_models,
    tags: template.tags,
  };
}

/**
 * 将表单数据转换为模板（用于保存）
 */
export function formDataToTemplate(
  formData: PricingTemplateFormData,
  existingTemplate?: PricingTemplate,
): PricingTemplate {
  const now = Date.now();
  return {
    id: existingTemplate?.id || generateTemplateId(formData.name),
    name: formData.name,
    description: formData.description,
    version: existingTemplate?.version || '1.0.0',
    created_at: existingTemplate?.created_at || now,
    updated_at: now,
    inherited_models: formData.inherited_models,
    custom_models: formData.custom_models,
    tags: formData.tags,
    is_default_preset: existingTemplate?.is_default_preset || false,
  };
}

/**
 * 生成模板 ID（基于名称和时间戳）
 */
function generateTemplateId(name: string): string {
  const sanitizedName = name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '_')
    .replace(/^_|_$/g, '');
  const timestamp = Date.now().toString(36);
  return `${sanitizedName}_${timestamp}`;
}
