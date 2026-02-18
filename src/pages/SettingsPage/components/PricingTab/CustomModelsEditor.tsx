// 自定义模型编辑器组件
// 展开式列表，每个模型可编辑价格和别名

import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Plus, Trash2, ChevronDown, ChevronRight } from 'lucide-react';
import type { ModelPrice } from '@/types/pricing';

/**
 * 常见的 AI 提供商列表
 */
const COMMON_PROVIDERS = [
  { value: 'anthropic', label: 'Anthropic (Claude)' },
  { value: 'openai', label: 'OpenAI (GPT)' },
  { value: 'google', label: 'Google (Gemini)' },
  { value: 'deepseek', label: 'DeepSeek' },
  { value: 'custom', label: '自定义提供商' },
];

interface CustomModelsEditorProps {
  /** 自定义模型数据 */
  data: Record<string, ModelPrice>;
  /** 数据变更回调 */
  onChange: (data: Record<string, ModelPrice>) => void;
  /** 只读模式（用于内置模板查看） */
  readOnly?: boolean;
}

export function CustomModelsEditor({ data, onChange, readOnly = false }: CustomModelsEditorProps) {
  const [expandedModels, setExpandedModels] = useState<Set<string>>(new Set());
  const [newModelName, setNewModelName] = useState('');
  const [newAliases, setNewAliases] = useState<Record<string, string>>({}); // 每个模型的新别名输入

  const modelNames = Object.keys(data);

  // 切换展开状态
  const toggleExpand = (modelName: string) => {
    const newSet = new Set(expandedModels);
    if (newSet.has(modelName)) {
      newSet.delete(modelName);
    } else {
      newSet.add(modelName);
    }
    setExpandedModels(newSet);
  };

  // 添加新模型
  const handleAddModel = () => {
    if (!newModelName.trim()) return;

    const newModel: ModelPrice = {
      provider: 'custom',
      input_price_per_1m: 0,
      output_price_per_1m: 0,
      cache_write_price_per_1m: 0,
      cache_read_price_per_1m: 0,
      currency: 'USD',
      aliases: [],
    };

    onChange({
      ...data,
      [newModelName.trim()]: newModel,
    });

    // 自动展开新添加的模型
    const newSet = new Set(expandedModels);
    newSet.add(newModelName.trim());
    setExpandedModels(newSet);

    setNewModelName('');
  };

  // 删除模型
  const handleDeleteModel = (modelName: string) => {
    const newData = { ...data };
    delete newData[modelName];
    onChange(newData);

    // 从展开列表中移除
    const newSet = new Set(expandedModels);
    newSet.delete(modelName);
    setExpandedModels(newSet);
  };

  // 更新模型字段
  const handleUpdateField = (
    modelName: string,
    field: keyof ModelPrice,
    value: string | number | string[],
  ) => {
    const newData = {
      ...data,
      [modelName]: {
        ...data[modelName],
        [field]: value,
      },
    };
    onChange(newData);
  };

  // 添加别名
  const handleAddAlias = (modelName: string) => {
    const newAlias = newAliases[modelName]?.trim();
    if (!newAlias) return;

    const currentAliases = data[modelName].aliases;
    if (currentAliases.includes(newAlias)) {
      // 别名已存在，不重复添加
      return;
    }

    handleUpdateField(modelName, 'aliases', [...currentAliases, newAlias]);
    // 清空输入框
    setNewAliases((prev) => ({ ...prev, [modelName]: '' }));
  };

  // 删除别名
  const handleDeleteAlias = (modelName: string, aliasIndex: number) => {
    const currentAliases = data[modelName].aliases;
    const newAliases = currentAliases.filter((_, i) => i !== aliasIndex);
    handleUpdateField(modelName, 'aliases', newAliases);
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h4 className="font-medium">自定义模型</h4>
          <p className="text-xs text-muted-foreground">直接定义模型价格，不依赖继承</p>
        </div>
      </div>

      {/* 添加新模型 */}
      {!readOnly && (
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-sm">添加新模型</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex gap-2">
              <Input
                placeholder="模型名称（如：custom-model-v1）"
                value={newModelName}
                onChange={(e) => setNewModelName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    handleAddModel();
                  }
                }}
              />
              <Button onClick={handleAddModel} disabled={!newModelName.trim()}>
                <Plus className="mr-1 h-3 w-3" />
                添加
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      {/* 模型列表 */}
      {modelNames.length === 0 ? (
        <div className="text-center py-8 text-sm text-muted-foreground border border-dashed rounded-lg">
          {readOnly ? '此模板无自定义模型' : '暂无自定义模型，在上方输入框添加新模型'}
        </div>
      ) : (
        <div className="space-y-2">
          {modelNames.map((modelName) => {
            const model = data[modelName];
            const isExpanded = expandedModels.has(modelName);

            return (
              <Collapsible
                key={modelName}
                open={isExpanded}
                onOpenChange={() => toggleExpand(modelName)}
              >
                <Card>
                  <CollapsibleTrigger className="w-full">
                    <CardHeader className="cursor-pointer hover:bg-accent/50 transition-colors">
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-2">
                          {isExpanded ? (
                            <ChevronDown className="h-4 w-4" />
                          ) : (
                            <ChevronRight className="h-4 w-4" />
                          )}
                          <span className="font-mono font-medium">{modelName}</span>
                        </div>
                        <div className="flex items-center gap-4 text-xs text-muted-foreground">
                          <span>输入: ${model.input_price_per_1m.toFixed(2)}/1M</span>
                          <span>输出: ${model.output_price_per_1m.toFixed(2)}/1M</span>
                          {!readOnly && (
                            <Button
                              size="sm"
                              variant="ghost"
                              onClick={(e) => {
                                e.stopPropagation();
                                handleDeleteModel(modelName);
                              }}
                              className="h-7 px-2 text-destructive hover:text-destructive"
                            >
                              <Trash2 className="h-3 w-3" />
                            </Button>
                          )}
                        </div>
                      </div>
                    </CardHeader>
                  </CollapsibleTrigger>

                  <CollapsibleContent>
                    <CardContent className="grid grid-cols-2 gap-4 pt-4">
                      {/* 提供商 */}
                      <div className="space-y-2">
                        <Label>提供商</Label>
                        <Select
                          value={model.provider}
                          onValueChange={(value) => handleUpdateField(modelName, 'provider', value)}
                          disabled={readOnly}
                        >
                          <SelectTrigger>
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            {COMMON_PROVIDERS.map((provider) => (
                              <SelectItem key={provider.value} value={provider.value}>
                                {provider.label}
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      </div>

                      {/* 货币 */}
                      <div className="space-y-2">
                        <Label>货币</Label>
                        <Input
                          value={model.currency}
                          onChange={(e) => handleUpdateField(modelName, 'currency', e.target.value)}
                          placeholder="USD"
                          disabled={readOnly}
                        />
                      </div>

                      {/* 输入价格 */}
                      <div className="space-y-2">
                        <Label>输入价格 (USD/百万 Token)</Label>
                        <Input
                          type="number"
                          step="0.01"
                          min="0"
                          value={model.input_price_per_1m}
                          onChange={(e) =>
                            handleUpdateField(
                              modelName,
                              'input_price_per_1m',
                              parseFloat(e.target.value) || 0,
                            )
                          }
                          disabled={readOnly}
                        />
                      </div>

                      {/* 输出价格 */}
                      <div className="space-y-2">
                        <Label>输出价格 (USD/百万 Token)</Label>
                        <Input
                          type="number"
                          step="0.01"
                          min="0"
                          value={model.output_price_per_1m}
                          onChange={(e) =>
                            handleUpdateField(
                              modelName,
                              'output_price_per_1m',
                              parseFloat(e.target.value) || 0,
                            )
                          }
                          disabled={readOnly}
                        />
                      </div>

                      {/* 缓存写入价格 - 5分钟 TTL */}
                      <div className="space-y-2">
                        <Label>缓存写入价格 - 5m (可选)</Label>
                        <Input
                          type="number"
                          step="0.01"
                          min="0"
                          value={model.cache_write_price_per_1m || ''}
                          onChange={(e) =>
                            handleUpdateField(
                              modelName,
                              'cache_write_price_per_1m',
                              e.target.value ? parseFloat(e.target.value) : 0,
                            )
                          }
                          placeholder="留空表示不支持"
                          disabled={readOnly}
                        />
                      </div>

                      {/* 缓存写入价格 - 1小时 TTL */}
                      <div className="space-y-2">
                        <Label>缓存写入价格 - 1h (可选)</Label>
                        <Input
                          type="number"
                          step="0.01"
                          min="0"
                          value={model.cache_write_1h_price_per_1m || ''}
                          onChange={(e) =>
                            handleUpdateField(
                              modelName,
                              'cache_write_1h_price_per_1m',
                              e.target.value ? parseFloat(e.target.value) : 0,
                            )
                          }
                          placeholder="留空表示不支持"
                          disabled={readOnly}
                        />
                      </div>

                      {/* 缓存读取价格 */}
                      <div className="space-y-2">
                        <Label>缓存读取价格 (可选)</Label>
                        <Input
                          type="number"
                          step="0.01"
                          min="0"
                          value={model.cache_read_price_per_1m || ''}
                          onChange={(e) =>
                            handleUpdateField(
                              modelName,
                              'cache_read_price_per_1m',
                              e.target.value ? parseFloat(e.target.value) : 0,
                            )
                          }
                          placeholder="留空表示不支持"
                          disabled={readOnly}
                        />
                      </div>

                      {/* 别名列表 */}
                      <div className="col-span-2 space-y-2">
                        <Label>别名列表</Label>
                        <div className="space-y-2">
                          {/* 添加别名输入框 */}
                          {!readOnly && (
                            <div className="flex gap-2">
                              <Input
                                value={newAliases[modelName] || ''}
                                onChange={(e) =>
                                  setNewAliases((prev) => ({
                                    ...prev,
                                    [modelName]: e.target.value,
                                  }))
                                }
                                onKeyDown={(e) => {
                                  if (e.key === 'Enter') {
                                    e.preventDefault();
                                    handleAddAlias(modelName);
                                  }
                                }}
                                placeholder="输入别名（如：claude-sonnet-4-5）"
                              />
                              <Button
                                size="sm"
                                onClick={() => handleAddAlias(modelName)}
                                disabled={!newAliases[modelName]?.trim()}
                              >
                                <Plus className="h-3 w-3" />
                              </Button>
                            </div>
                          )}

                          {/* 别名列表展示 */}
                          {model.aliases.length > 0 ? (
                            <div className="border rounded-md p-2 space-y-1">
                              {model.aliases.map((alias, index) => (
                                <div
                                  key={index}
                                  className="flex items-center justify-between bg-secondary/50 rounded px-2 py-1 text-sm"
                                >
                                  <code className="font-mono">{alias}</code>
                                  {!readOnly && (
                                    <Button
                                      size="sm"
                                      variant="ghost"
                                      onClick={() => handleDeleteAlias(modelName, index)}
                                      className="h-6 w-6 p-0 text-destructive hover:text-destructive"
                                    >
                                      <Trash2 className="h-3 w-3" />
                                    </Button>
                                  )}
                                </div>
                              ))}
                            </div>
                          ) : (
                            <div className="text-xs text-muted-foreground border border-dashed rounded-md p-2 text-center">
                              {readOnly ? '无别名' : '暂无别名，在上方输入框添加'}
                            </div>
                          )}
                          <p className="text-xs text-muted-foreground">
                            用于匹配不同格式的模型 ID（如 claude-sonnet-4-5-20250929）
                          </p>
                        </div>
                      </div>
                    </CardContent>
                  </CollapsibleContent>
                </Card>
              </Collapsible>
            );
          })}
        </div>
      )}
    </div>
  );
}
