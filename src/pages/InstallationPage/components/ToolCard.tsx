import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { CheckCircle2, Loader2, Package } from 'lucide-react';
import { logoMap, descriptionMap } from '@/utils/constants';
import { formatVersionLabel } from '@/utils/formatting';
import type { ToolStatus } from '@/lib/tauri-commands';

interface ToolCardProps {
  tool: ToolStatus;
  installMethod: string;
  installing: boolean;
  availableMethods: Array<{ value: string; label: string; disabled?: boolean }>;
  onInstall: () => void;
  onMethodChange: (method: string) => void;
}

export function ToolCard({
  tool,
  installMethod,
  installing,
  availableMethods,
  onInstall,
  onMethodChange,
}: ToolCardProps) {
  return (
    <Card className="shadow-sm border">
      <CardContent className="p-5">
        <div className="flex items-start justify-between gap-6">
          <div className="flex items-center gap-4 flex-1">
            <div className="bg-secondary p-3 rounded-lg flex-shrink-0">
              <img src={logoMap[tool.id]} alt={tool.name} className="w-12 h-12" />
            </div>
            <div className="flex-1 space-y-1.5">
              <div className="flex items-center gap-3">
                <h4 className="font-semibold text-lg">{tool.name}</h4>
                {tool.installed && (
                  <Badge variant="default" className="gap-1">
                    <CheckCircle2 className="h-3 w-3" />
                    已安装
                  </Badge>
                )}
              </div>
              <p className="text-sm text-muted-foreground leading-relaxed">
                {descriptionMap[tool.id]}
              </p>
              {tool.installed && tool.version && (
                <div className="flex items-center gap-2 mt-3">
                  <span className="text-xs font-semibold text-slate-600 dark:text-slate-400">
                    当前版本:
                  </span>
                  <span className="font-mono text-xs font-semibold text-blue-600 dark:text-blue-400 bg-blue-50 dark:bg-blue-950 px-2.5 py-1 rounded-lg shadow-sm">
                    {formatVersionLabel(tool.version)}
                  </span>
                </div>
              )}
            </div>
          </div>

          <div className="flex flex-col gap-3 items-end">
            {!tool.installed && (
              <div className="w-48">
                <Label htmlFor={`method-${tool.id}`} className="text-xs mb-1.5 block">
                  安装方式
                </Label>
                <Select value={installMethod} onValueChange={onMethodChange}>
                  <SelectTrigger id={`method-${tool.id}`} className="shadow-sm h-9 text-sm">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {availableMethods.map((method) => (
                      <SelectItem
                        key={method.value}
                        value={method.value}
                        disabled={method.disabled}
                      >
                        {method.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            )}

            <Button
              disabled={tool.installed || installing}
              onClick={onInstall}
              className="shadow-md hover:shadow-lg transition-all bg-gradient-to-r from-blue-500 to-cyan-500 hover:from-blue-600 hover:to-cyan-600 disabled:from-slate-400 disabled:to-slate-400 h-11 px-6 font-medium w-48"
              size="lg"
            >
              {installing ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  安装中...
                </>
              ) : tool.installed ? (
                <>
                  <CheckCircle2 className="mr-2 h-4 w-4" />
                  已安装
                </>
              ) : (
                <>
                  <Package className="mr-2 h-4 w-4" />
                  安装工具
                </>
              )}
            </Button>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
