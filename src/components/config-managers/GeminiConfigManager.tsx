import { useCallback, useState } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  getGeminiSchema,
  getGeminiSettings,
  saveGeminiSettings,
  type GeminiEnvConfig,
  type GeminiSettingsPayload,
  type JsonObject,
} from '@/lib/tauri-commands';
import { ToolConfigManager } from '@/components/ToolConfigManager';
import { SecretInput } from '@/components/SecretInput';
import type { DiffEntry } from '@/components/tool-config/types';
import { GEMINI_ENV_DEFAULT, cloneGeminiEnv } from '@/components/tool-config/types';

interface GeminiConfigManagerProps {
  refreshSignal?: number;
}

export function GeminiConfigManager({ refreshSignal }: GeminiConfigManagerProps) {
  const [envState, setEnvState] = useState<GeminiEnvConfig>(() =>
    cloneGeminiEnv(GEMINI_ENV_DEFAULT),
  );
  const [originalEnv, setOriginalEnv] = useState<GeminiEnvConfig>(() =>
    cloneGeminiEnv(GEMINI_ENV_DEFAULT),
  );
  const [envDirty, setEnvDirty] = useState(false);

  const loadSettings = useCallback(async () => {
    const payload: GeminiSettingsPayload = await getGeminiSettings();
    const nextEnv = cloneGeminiEnv(payload.env);
    setEnvState(nextEnv);
    setOriginalEnv(nextEnv);
    setEnvDirty(false);
    return payload.settings;
  }, []);

  const saveConfig = useCallback(
    async (settings: JsonObject) => {
      await saveGeminiSettings(settings, envState);
      setOriginalEnv(cloneGeminiEnv(envState));
      setEnvDirty(false);
    },
    [envState],
  );

  const handleResetEnv = useCallback(() => {
    setEnvState(cloneGeminiEnv(originalEnv));
    setEnvDirty(false);
  }, [originalEnv]);

  const updateEnvField = useCallback((field: keyof GeminiEnvConfig, value: string) => {
    setEnvState((prev) => ({ ...prev, [field]: value }));
    setEnvDirty(true);
  }, []);

  const computeEnvDiffs = useCallback((): DiffEntry[] => {
    const diffs: DiffEntry[] = [];
    (['apiKey', 'baseUrl', 'model'] as const).forEach((field) => {
      if (envState[field] === originalEnv[field]) {
        return;
      }

      const beforeValue = originalEnv[field];
      const afterValue = envState[field];
      let type: DiffEntry['type'] = 'changed';
      if (!beforeValue && afterValue) {
        type = 'added';
      } else if (beforeValue && !afterValue) {
        type = 'removed';
      }

      const path = `env.${
        field === 'apiKey'
          ? 'GEMINI_API_KEY'
          : field === 'baseUrl'
            ? 'GOOGLE_GEMINI_BASE_URL'
            : 'GEMINI_MODEL'
      }`;

      diffs.push({
        path,
        type,
        before: beforeValue || undefined,
        after: afterValue || undefined,
      });
    });
    return diffs;
  }, [envState, originalEnv]);

  return (
    <div className="space-y-4">
      <ToolConfigManager
        title="Gemini 配置管理"
        description="读取并编辑 settings.json"
        loadSchema={getGeminiSchema}
        loadSettings={loadSettings}
        saveSettings={saveConfig}
        emptyHint="当前 settings.json 为空，点击「新增配置选项」开始添加。"
        refreshSignal={refreshSignal}
        externalDirty={envDirty}
        onResetExternalChanges={handleResetEnv}
        computeExternalDiffs={computeEnvDiffs}
      />

      <Card className="border">
        <CardHeader>
          <CardTitle>Gemini .env</CardTitle>
          <CardDescription>读取并编辑 .env，管理 Base URL、API Key 与默认模型。</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-col gap-3 md:flex-row md:items-center">
            <div className="flex items-center gap-2 md:basis-1/3">
              <Label htmlFor="gemini-api-key" className="font-mono text-sm font-semibold">
                GEMINI_API_KEY
              </Label>
              <Badge variant="outline">string</Badge>
            </div>
            <SecretInput
              id="gemini-api-key"
              value={envState.apiKey}
              onValueChange={(val) => updateEnvField('apiKey', val)}
              placeholder="ya29...."
              className="w-full"
              wrapperClassName="w-full"
              toggleLabel="切换 Gemini API Key 可见性"
            />
          </div>
          <div className="flex flex-col gap-3 md:flex-row md:items-center">
            <div className="flex items-center gap-2 md:basis-1/3">
              <Label htmlFor="gemini-base-url" className="font-mono text-sm font-semibold">
                GOOGLE_GEMINI_BASE_URL
              </Label>
              <Badge variant="outline">string</Badge>
            </div>
            <Input
              id="gemini-base-url"
              value={envState.baseUrl}
              onChange={(event) => updateEnvField('baseUrl', event.target.value)}
              placeholder="https://generativelanguage.googleapis.com"
            />
          </div>
          <div className="flex flex-col gap-3 md:flex-row md:items-center">
            <div className="flex items-center gap-2 md:basis-1/3">
              <Label htmlFor="gemini-model" className="font-mono text-sm font-semibold">
                GEMINI_MODEL
              </Label>
              <Badge variant="outline">string</Badge>
            </div>
            <Input
              id="gemini-model"
              value={envState.model}
              onChange={(event) => updateEnvField('model', event.target.value)}
              placeholder="gemini-2.5-pro"
            />
          </div>
          <p className="text-xs text-muted-foreground">
            修改以上字段后请点击上方"保存"，系统会同步写入 settings.json 与 .env。
          </p>
          {envDirty && (
            <p className="text-xs text-amber-600">.env 内容已修改，记得通过保存按钮写回磁盘。</p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
