import { useCallback, useState } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Label } from '@/components/ui/label';
import {
  getCodexSchema,
  getCodexSettings,
  saveCodexSettings,
  type CodexSettingsPayload,
  type JsonObject,
} from '@/lib/tauri-commands';
import { ToolConfigManager } from '@/components/ToolConfigManager';
import { SecretInput } from '@/components/SecretInput';
import type { DiffEntry } from '@/components/tool-config/types';

interface CodexConfigManagerProps {
  refreshSignal?: number;
}

export function CodexConfigManager({ refreshSignal }: CodexConfigManagerProps) {
  const [authToken, setAuthToken] = useState('');
  const [originalAuthToken, setOriginalAuthToken] = useState('');
  const [authDirty, setAuthDirty] = useState(false);

  const loadSettings = useCallback(async () => {
    const payload: CodexSettingsPayload = await getCodexSettings();
    const token = payload.authToken ?? '';
    setAuthToken(token);
    setOriginalAuthToken(token);
    setAuthDirty(false);
    return payload.config;
  }, []);

  const saveConfig = useCallback(
    async (settings: JsonObject) => {
      await saveCodexSettings(settings, authToken);
      setOriginalAuthToken(authToken);
      setAuthDirty(false);
    },
    [authToken],
  );

  const computeAuthDiffs = useCallback((): DiffEntry[] => {
    if (authToken === originalAuthToken) {
      return [];
    }
    const beforeValue = originalAuthToken ?? '';
    const afterValue = authToken ?? '';

    let type: DiffEntry['type'] = 'changed';
    if (!beforeValue && afterValue) {
      type = 'added';
    } else if (beforeValue && !afterValue) {
      type = 'removed';
    }

    return [
      {
        path: 'auth.OPENAI_API_KEY',
        type,
        before: beforeValue || undefined,
        after: afterValue || undefined,
      },
    ];
  }, [authToken, originalAuthToken]);

  const handleResetAuthToken = useCallback(() => {
    setAuthToken(originalAuthToken);
    setAuthDirty(false);
  }, [originalAuthToken]);

  return (
    <div className="space-y-4">
      <ToolConfigManager
        title="Codex 配置管理"
        description="读取并编辑 config.toml"
        loadSchema={getCodexSchema}
        loadSettings={loadSettings}
        saveSettings={saveConfig}
        emptyHint="当前 config.toml 为空，点击「新增配置选项」开始添加。"
        refreshSignal={refreshSignal}
        externalDirty={authDirty}
        onResetExternalChanges={handleResetAuthToken}
        computeExternalDiffs={computeAuthDiffs}
      />

      <Card className="border">
        <CardHeader>
          <CardTitle>Codex API Key</CardTitle>
          <CardDescription>读取并编辑 auth.json，用于 Codex CLI 请求。</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="flex flex-col gap-3 md:flex-row md:items-center">
            <div className="flex items-center gap-2 md:basis-1/2">
              <Label htmlFor="codex-api-key" className="font-mono text-sm font-semibold">
                OPENAI_API_KEY
              </Label>
              <Badge variant="outline">string</Badge>
            </div>
            <div className="flex-1 min-w-0">
              <SecretInput
                id="codex-api-key"
                value={authToken}
                onValueChange={(val) => {
                  setAuthToken(val);
                  setAuthDirty(true);
                }}
                placeholder="sk-..."
                toggleLabel="切换 Codex API Key 可见性"
                className="w-full"
                wrapperClassName="w-full"
              />
            </div>
          </div>
          <p className="text-xs text-muted-foreground">
            修改后点击上方"保存"将同时写入 config.toml 与 auth.json。
          </p>
          {authDirty && <p className="text-xs text-amber-600">API Key 已更新，记得保存以生效。</p>}
        </CardContent>
      </Card>
    </div>
  );
}
