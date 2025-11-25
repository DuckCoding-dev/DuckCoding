import { useState, useEffect, useCallback } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { FileText, Info, Loader2, Save, AlertCircle } from 'lucide-react';
import { useToast } from '@/hooks/use-toast';
import { getLogConfig, updateLogConfig, isReleaseBuild, type LogConfig } from '@/lib/tauri-commands';
import { Alert, AlertDescription } from '@/components/ui/alert';

export function LogSettingsTab() {
  const { toast } = useToast();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [isRelease, setIsRelease] = useState(false);
  const [config, setConfig] = useState<LogConfig>({
    level: 'info',
    format: 'text',
    output: 'both',
    file_path: null,
  });
  const [originalConfig, setOriginalConfig] = useState<LogConfig | null>(null);

  // 加载当前配置
  const loadConfig = useCallback(async () => {
    try {
      setLoading(true);
      const [currentConfig, releaseFlag] = await Promise.all([
        getLogConfig(),
        isReleaseBuild(),
      ]);

      setIsRelease(releaseFlag);

      // Release 版本强制使用 File 输出
      if (releaseFlag && currentConfig.output !== 'file') {
        currentConfig.output = 'file';
      }

      setConfig(currentConfig);
      setOriginalConfig(currentConfig);
    } catch (error) {
      console.error('Failed to load log config:', error);
      toast({
        title: '加载失败',
        description: String(error),
        variant: 'destructive',
      });
    } finally {
      setLoading(false);
    }
  }, [toast]);

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  // 判断配置是否需要重启
  const needsRestart = (): boolean => {
    if (!originalConfig) return false;
    return (
      config.format !== originalConfig.format ||
      config.output !== originalConfig.output ||
      config.file_path !== originalConfig.file_path
    );
  };

  // 保存配置
  const handleSave = async () => {
    try {
      setSaving(true);

      // Release 版本强制使用 File 输出
      const configToSave = isRelease
        ? { ...config, output: 'file' as const }
        : config;

      const message = await updateLogConfig(configToSave);
      setOriginalConfig(configToSave);
      toast({
        title: '保存成功',
        description: message,
      });
    } catch (error) {
      console.error('Failed to save log config:', error);
      toast({
        title: '保存失败',
        description: String(error),
        variant: 'destructive',
      });
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center p-12">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="space-y-4 rounded-lg border p-6">
      <div className="flex items-center gap-2">
        <FileText className="h-5 w-5" />
        <h3 className="text-lg font-semibold">日志系统配置</h3>
      </div>
      <Separator />

      <div className="space-y-6">
        {/* 日志级别 */}
        <div className="space-y-2">
          <Label htmlFor="log-level">日志级别</Label>
          <Select
            value={config.level}
            onValueChange={(value) =>
              setConfig({ ...config, level: value as LogConfig['level'] })
            }
          >
            <SelectTrigger id="log-level" className="shadow-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="trace">Trace（最详细）</SelectItem>
              <SelectItem value="debug">Debug（调试）</SelectItem>
              <SelectItem value="info">Info（信息）</SelectItem>
              <SelectItem value="warn">Warn（警告）</SelectItem>
              <SelectItem value="error">Error（错误）</SelectItem>
            </SelectContent>
          </Select>
          <p className="text-xs text-muted-foreground">
            日志级别变更可以实时生效（热重载），无需重启应用
          </p>
        </div>

        {/* 输出格式 */}
        <div className="space-y-2">
          <Label htmlFor="log-format">输出格式</Label>
          <Select
            value={config.format}
            onValueChange={(value) =>
              setConfig({ ...config, format: value as LogConfig['format'] })
            }
          >
            <SelectTrigger id="log-format" className="shadow-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="text">纯文本格式</SelectItem>
              <SelectItem value="json">JSON 格式</SelectItem>
            </SelectContent>
          </Select>
          <p className="text-xs text-muted-foreground">格式变更需要重启应用后生效</p>
        </div>

        {/* 输出目标（Release 版本隐藏，强制使用文件输出） */}
        {!isRelease && (
          <div className="space-y-2">
            <Label htmlFor="log-output">输出目标</Label>
            <Select
              value={config.output}
              onValueChange={(value) =>
                setConfig({ ...config, output: value as LogConfig['output'] })
              }
            >
              <SelectTrigger id="log-output" className="shadow-sm">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="console">仅控制台</SelectItem>
                <SelectItem value="file">仅文件</SelectItem>
                <SelectItem value="both">控制台和文件</SelectItem>
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">输出目标变更需要重启应用后生效</p>
          </div>
        )}

        {/* Release 版本提示 */}
        {isRelease && (
          <Alert className="border-blue-200 bg-blue-50 dark:bg-blue-950/30">
            <Info className="h-4 w-4 text-blue-600 dark:text-blue-400" />
            <AlertDescription className="text-blue-800 dark:text-blue-200">
              当前为 Release 版本，日志将自动输出到文件（无控制台输出）
            </AlertDescription>
          </Alert>
        )}

        {/* 文件路径（Release 版本总是显示） */}
        {(isRelease || config.output === 'file' || config.output === 'both') && (
          <div className="space-y-2">
            <Label htmlFor="log-file-path">日志文件目录</Label>
            <Input
              id="log-file-path"
              placeholder="留空使用默认路径（~/.duckcoding/logs）"
              value={config.file_path || ''}
              onChange={(e) =>
                setConfig({ ...config, file_path: e.target.value || null })
              }
              className="shadow-sm"
            />
            <p className="text-xs text-muted-foreground">
              指定日志文件的保存目录。日志文件会按日期自动滚动（每日一个文件）
            </p>
          </div>
        )}

        {/* 热重载提示 */}
        <div className="rounded-lg bg-blue-50 dark:bg-blue-950/30 border border-blue-200 dark:border-blue-800 p-4">
          <div className="flex items-start gap-2">
            <Info className="h-4 w-4 text-blue-600 dark:text-blue-400 flex-shrink-0 mt-0.5" />
            <div className="space-y-1 flex-1">
              <p className="text-sm font-semibold text-blue-800 dark:text-blue-200">
                关于热重载
              </p>
              <ul className="text-xs text-blue-700 dark:text-blue-300 space-y-1 list-disc list-inside">
                <li>
                  <strong>日志级别</strong>变更会立即生效，无需重启应用
                </li>
                <li>
                  <strong>输出格式、输出目标、文件路径</strong>变更需要重启应用后生效
                </li>
              </ul>
            </div>
          </div>
        </div>

        {/* 重启提示（需要重启时显示） */}
        {needsRestart() && (
          <Alert variant="default" className="border-orange-200 bg-orange-50 dark:bg-orange-950/30">
            <AlertCircle className="h-4 w-4 text-orange-600 dark:text-orange-400" />
            <AlertDescription className="text-orange-800 dark:text-orange-200">
              当前配置包含需要重启应用才能生效的项目，保存后请重启 DuckCoding
            </AlertDescription>
          </Alert>
        )}

        {/* 保存按钮 */}
        <div className="flex justify-end pt-2">
          <Button
            onClick={handleSave}
            disabled={saving}
            className="shadow-md hover:shadow-lg transition-all"
          >
            {saving ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                保存中...
              </>
            ) : (
              <>
                <Save className="mr-2 h-4 w-4" />
                保存日志配置
              </>
            )}
          </Button>
        </div>
      </div>
    </div>
  );
}
