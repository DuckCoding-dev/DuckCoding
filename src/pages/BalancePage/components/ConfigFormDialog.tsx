import { useEffect, useMemo, useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Checkbox } from '@/components/ui/checkbox';
import { Eye, EyeOff, RefreshCw } from 'lucide-react';
import { BalanceConfig, BalanceFormValues } from '../types';
import { BALANCE_TEMPLATES } from '../templates';
import { Textarea } from '@/components/ui/textarea';

const METHOD_OPTIONS = [
  { value: 'GET', label: 'GET' },
  { value: 'POST', label: 'POST' },
];

interface ConfigFormDialogProps {
  open: boolean;
  initial?: BalanceConfig;
  onClose: () => void;
  onSubmit: (config: BalanceFormValues) => void;
}

export function ConfigFormDialog({ open, initial, onClose, onSubmit }: ConfigFormDialogProps) {
  const [values, setValues] = useState<BalanceFormValues>({
    name: '',
    endpoint: '',
    method: 'GET',
    staticHeaders: '',
    extractorScript: '',
    intervalSec: 0,
    timeoutMs: 30000,
    apiKey: '',
    saveApiKey: true, // 默认勾选
  });
  const [showKey, setShowKey] = useState(false);
  const [selectedTemplate, setSelectedTemplate] = useState<string>('');

  useEffect(() => {
    if (initial) {
      setValues({
        name: initial.name,
        endpoint: initial.endpoint,
        method: initial.method,
        staticHeaders: initial.staticHeaders ? JSON.stringify(initial.staticHeaders, null, 2) : '',
        extractorScript: initial.extractorScript,
        intervalSec: initial.intervalSec ?? 0,
        timeoutMs: initial.timeoutMs ?? 30000,
        apiKey: initial.apiKey ?? '', // 编辑时加载已保存的 API Key
        saveApiKey: initial.saveApiKey ?? true, // 编辑时加载保存状态，默认 true
      });
      setSelectedTemplate('');
    } else {
      setValues({
        name: '',
        endpoint: '',
        method: 'GET',
        staticHeaders: '',
        extractorScript: '',
        intervalSec: 0,
        timeoutMs: 30000,
        apiKey: '',
        saveApiKey: true, // 新增时默认勾选
      });
      setSelectedTemplate('');
    }
    setShowKey(false);
  }, [initial, open]);

  const isEdit = useMemo(() => Boolean(initial), [initial]);

  const handleTemplateChange = (templateId: string) => {
    setSelectedTemplate(templateId);
    const template = BALANCE_TEMPLATES.find((t) => t.id === templateId);
    if (template) {
      setValues((v) => ({
        ...v,
        endpoint: template.endpoint,
        method: template.method,
        staticHeaders: template.staticHeaders
          ? JSON.stringify(template.staticHeaders, null, 2)
          : '',
        extractorScript: template.extractorScript,
      }));
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!values.name.trim() || !values.endpoint.trim() || !values.extractorScript.trim()) {
      return;
    }
    onSubmit({ ...values, name: values.name.trim() });
  };

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{isEdit ? '编辑配置' : '新增配置'}</DialogTitle>
          <DialogDescription>配置 API 端点和提取器脚本查询余额信息。</DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4">
          {!isEdit && (
            <div className="space-y-2">
              <Label>选择模板（可选）</Label>
              <Select value={selectedTemplate} onValueChange={handleTemplateChange}>
                <SelectTrigger>
                  <SelectValue placeholder="选择预设模板或自定义" />
                </SelectTrigger>
                <SelectContent>
                  {BALANCE_TEMPLATES.map((template) => (
                    <SelectItem key={template.id} value={template.id}>
                      {template.name} - {template.description}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}

          <div className="space-y-2">
            <Label htmlFor="name">配置名称</Label>
            <Input
              id="name"
              value={values.name}
              onChange={(e) => setValues((v) => ({ ...v, name: e.target.value }))}
              placeholder="例如：NewAPI 主账号"
              required
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="endpoint">API 端点 URL</Label>
            <Input
              id="endpoint"
              value={values.endpoint}
              onChange={(e) => setValues((v) => ({ ...v, endpoint: e.target.value }))}
              placeholder="https://api.example.com/balance"
              required
            />
          </div>

          <div className="space-y-2">
            <Label>HTTP 方法</Label>
            <Select
              value={values.method}
              onValueChange={(method) =>
                setValues((v) => ({ ...v, method: method as 'GET' | 'POST' }))
              }
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {METHOD_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label htmlFor="staticHeaders">静态请求头（JSON 格式，可选）</Label>
            <Textarea
              id="staticHeaders"
              value={values.staticHeaders ?? ''}
              onChange={(e) => setValues((v) => ({ ...v, staticHeaders: e.target.value }))}
              placeholder='{"Content-Type": "application/json"}'
              rows={3}
            />
            <p className="text-xs text-muted-foreground">
              静态请求头将被持久化。如需使用 API Key，请在下方单独输入。
            </p>
          </div>

          <div className="space-y-2">
            <Label htmlFor="extractorScript">提取器脚本（JavaScript）</Label>
            <Textarea
              id="extractorScript"
              value={values.extractorScript ?? ''}
              onChange={(e) => setValues((v) => ({ ...v, extractorScript: e.target.value }))}
              placeholder="const extractor = (response) => { ... }"
              rows={12}
              className="font-mono text-sm"
              required
            />
            <p className="text-xs text-muted-foreground">
              脚本需定义 extractor 函数，返回 {'{'}planName, remaining, used, total, unit{'}'}
            </p>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="intervalSec">自动刷新间隔（秒）</Label>
              <Input
                id="intervalSec"
                type="number"
                min={0}
                value={values.intervalSec ?? 0}
                onChange={(e) =>
                  setValues((v) => ({ ...v, intervalSec: Number(e.target.value) || 0 }))
                }
                placeholder="0 表示不自动刷新"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="timeoutMs">请求超时（毫秒）</Label>
              <Input
                id="timeoutMs"
                type="number"
                min={1000}
                value={values.timeoutMs ?? 30000}
                onChange={(e) =>
                  setValues((v) => ({ ...v, timeoutMs: Number(e.target.value) || 30000 }))
                }
              />
            </div>
          </div>

          <div className="space-y-3">
            <Label htmlFor="apiKey">API Key</Label>
            <div className="flex items-center gap-2">
              <Input
                id="apiKey"
                type={showKey ? 'text' : 'password'}
                value={values.apiKey ?? ''}
                onChange={(e) => setValues((v) => ({ ...v, apiKey: e.target.value }))}
                placeholder="sk-..."
              />
              <Button
                type="button"
                variant="outline"
                size="icon"
                onClick={() => setShowKey((s) => !s)}
                aria-label={showKey ? '隐藏密钥' : '显示密钥'}
              >
                {showKey ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
              </Button>
            </div>

            <div className="flex items-start space-x-2 pt-1">
              <Checkbox
                id="saveApiKey"
                checked={values.saveApiKey}
                onCheckedChange={(checked: boolean) =>
                  setValues((v) => ({ ...v, saveApiKey: checked === true }))
                }
              />
              <div className="grid gap-1.5 leading-none">
                <label
                  htmlFor="saveApiKey"
                  className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70 cursor-pointer"
                >
                  保存 API Key 到文件
                </label>
                <p className="text-xs text-muted-foreground">
                  勾选后密钥将保存到 balance.json，应用重启后自动加载。不勾选则仅保存在内存中。
                </p>
              </div>
            </div>
          </div>

          <DialogFooter className="gap-2">
            <Button type="button" variant="outline" onClick={onClose}>
              取消
            </Button>
            <Button type="submit">
              <RefreshCw className="mr-2 h-4 w-4" />
              {isEdit ? '保存' : '添加'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
