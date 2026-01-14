// 价格模板选择器通用组件
// 用于 Profile 表单中选择价格模板

import { useEffect, useState } from 'react';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { listPricingTemplates } from '@/lib/tauri-commands/pricing';
import type { PricingTemplate } from '@/types/pricing';
import { Loader2 } from 'lucide-react';

interface PricingTemplateSelectorProps {
  /** 工具 ID（可选，用于工具特定逻辑） */
  toolId?: string;
  /** 当前选中的模板 ID */
  value?: string;
  /** 值变更回调 */
  onChange: (value?: string) => void;
  /** 是否禁用 */
  disabled?: boolean;
}

/**
 * 价格模板选择器
 *
 * 自动加载所有可用模板，支持"使用默认模板"选项
 */
export function PricingTemplateSelector({
  toolId: _toolId, // 预留用于工具特定逻辑
  value,
  onChange,
  disabled,
}: PricingTemplateSelectorProps) {
  const [loading, setLoading] = useState(true);
  const [templates, setTemplates] = useState<PricingTemplate[]>([]);

  // TODO: 使用 _toolId 实现工具特定的模板过滤逻辑

  useEffect(() => {
    listPricingTemplates()
      .then(setTemplates)
      .catch((error) => {
        console.error('Failed to load pricing templates:', error);
      })
      .finally(() => setLoading(false));
  }, []);

  if (loading) {
    return (
      <div className="space-y-2">
        <Label>价格模板</Label>
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin" />
          加载中...
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-2">
      <Label>价格模板</Label>
      <Select
        value={value || 'default'}
        onValueChange={(val) => onChange(val === 'default' ? undefined : val)}
        disabled={disabled}
      >
        <SelectTrigger>
          <SelectValue placeholder="选择价格模板" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="default">使用默认模板 (工具默认价格)</SelectItem>
          {templates.map((t) => (
            <SelectItem key={t.id} value={t.id}>
              {t.name}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <p className="text-xs text-muted-foreground">
        用于计算该 Profile 的 Token 成本（留空使用工具默认模板）
      </p>
    </div>
  );
}
