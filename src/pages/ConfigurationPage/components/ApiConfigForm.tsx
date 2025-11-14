import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Info, Loader2, Save, Sparkles } from 'lucide-react';
import { logoMap } from '@/utils/constants';
import { openExternalLink } from '@/utils/formatting';
import type { ToolStatus } from '@/lib/tauri-commands';

interface ApiConfigFormProps {
  selectedTool: string;
  setSelectedTool: (tool: string) => void;
  provider: string;
  setProvider: (provider: string) => void;
  apiKey: string;
  setApiKey: (key: string) => void;
  baseUrl: string;
  setBaseUrl: (url: string) => void;
  profileName: string;
  setProfileName: (name: string) => void;
  installedTools: ToolStatus[];
  configuring: boolean;
  generatingKey: boolean;
  onGenerateKey: () => void;
  onSaveConfig: () => void;
  onClearForm: () => void;
}

export function ApiConfigForm({
  selectedTool,
  setSelectedTool,
  provider,
  setProvider,
  apiKey,
  setApiKey,
  baseUrl,
  setBaseUrl,
  profileName,
  setProfileName,
  installedTools,
  configuring,
  generatingKey,
  onGenerateKey,
  onSaveConfig,
  onClearForm,
}: ApiConfigFormProps) {
  return (
    <Card className="shadow-sm border">
      <CardHeader>
        <CardTitle>API 配置</CardTitle>
        <CardDescription>为已安装的工具配置 API 密钥</CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        <div className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="tool-select">选择工具 *</Label>
            <Select value={selectedTool} onValueChange={setSelectedTool}>
              <SelectTrigger id="tool-select" className="shadow-sm">
                <SelectValue placeholder="选择要配置的工具" />
              </SelectTrigger>
              <SelectContent>
                {installedTools.map((tool) => (
                  <SelectItem key={tool.id} value={tool.id}>
                    <div className="flex items-center gap-2">
                      <img src={logoMap[tool.id]} className="w-4 h-4" />
                      {tool.name}
                    </div>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label htmlFor="provider-select">API 提供商 *</Label>
            <Select value={provider} onValueChange={setProvider}>
              <SelectTrigger id="provider-select" className="shadow-sm">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="duckcoding">DuckCoding (推荐)</SelectItem>
                <SelectItem value="custom">自定义端点</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label htmlFor="api-key">API Key *</Label>
            <div className="flex gap-2">
              <Input
                id="api-key"
                type="password"
                placeholder="输入 API Key"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                className="shadow-sm flex-1"
              />
              <Button
                onClick={onGenerateKey}
                disabled={generatingKey || !selectedTool}
                variant="outline"
                className="shadow-sm hover:shadow-md transition-all"
                title="一键生成 DuckCoding API Key"
              >
                {generatingKey ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    生成中...
                  </>
                ) : (
                  <>
                    <Sparkles className="mr-2 h-4 w-4" />
                    一键生成
                  </>
                )}
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">
              点击"一键生成"可自动创建 DuckCoding API Key（需先配置全局设置）
            </p>
          </div>

          {provider === 'duckcoding' && (
            <div className="rounded-lg bg-blue-50 dark:bg-blue-950/30 border border-blue-200 dark:border-blue-800 p-4">
              <div className="flex items-start gap-2 mb-3">
                <Info className="h-4 w-4 text-blue-600 dark:text-blue-400 flex-shrink-0 mt-0.5" />
                <div className="space-y-2 flex-1">
                  <p className="text-xs font-semibold text-blue-800 dark:text-blue-200">
                    点击"改用 npm 安装"将自动切换为 npm 方式并重新安装
                  </p>
                  <p className="text-xs text-blue-700 dark:text-blue-300">
                    npm 安装会直接从 npm 仓库获取最新版本
                  </p>
                </div>
              </div>
            </div>
          )}

          {provider === 'custom' && (
            <div className="space-y-2">
              <Label htmlFor="base-url">Base URL *</Label>
              <Input
                id="base-url"
                type="url"
                placeholder="https://api.example.com"
                value={baseUrl}
                onChange={(e) => setBaseUrl(e.target.value)}
                className="shadow-sm"
              />
            </div>
          )}

          <div className="space-y-2">
            <Label htmlFor="profile-name">配置文件名称 (可选)</Label>
            <Input
              id="profile-name"
              type="text"
              placeholder="例如: work, personal"
              value={profileName}
              onChange={(e) => setProfileName(e.target.value)}
              className="shadow-sm"
            />
            <p className="text-xs text-muted-foreground">
              留空将直接保存到主配置。填写名称可保存多个配置方便切换
            </p>
          </div>
        </div>
      </CardContent>
      <CardFooter className="flex justify-between">
        <Button variant="outline" onClick={onClearForm} className="shadow-sm">
          清空
        </Button>
        <Button
          onClick={onSaveConfig}
          disabled={configuring || !selectedTool || !apiKey}
          className="shadow-sm hover:shadow-md transition-all"
        >
          {configuring ? (
            <>
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              保存中...
            </>
          ) : (
            <>
              <Save className="mr-2 h-4 w-4" />
              保存配置
            </>
          )}
        </Button>
      </CardFooter>
    </Card>
  );
}
