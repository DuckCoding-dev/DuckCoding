import { useCallback, useEffect, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  getClaudeSchema,
  getClaudeSettings,
  saveClaudeSettings,
  type ClaudeSettingsPayload,
  type JsonObject,
} from '@/lib/tauri-commands';
import { useToast } from '@/hooks/use-toast';
import { Plus, Trash2 } from 'lucide-react';
import { ToolConfigManager } from '@/components/ToolConfigManager';
import type { DiffEntry } from '@/components/tool-config/types';
import { formatJson } from '@/components/tool-config/utils';

interface ClaudeConfigManagerProps {
  refreshSignal?: number;
}

export function ClaudeConfigManager({ refreshSignal }: ClaudeConfigManagerProps) {
  const { toast } = useToast();
  const [extraEntries, setExtraEntries] = useState<{ key: string; value: string }[]>([]);
  const [originalExtraEntries, setOriginalExtraEntries] = useState<
    { key: string; value: string }[]
  >([]);
  const [extraDirty, setExtraDirty] = useState(false);
  const [extraError, setExtraError] = useState<string | null>(null);

  const toEntries = useCallback((obj?: JsonObject | null): { key: string; value: string }[] => {
    if (!obj) return [];
    return Object.entries(obj).map(([key, value]) => ({
      key,
      value: typeof value === 'string' ? value : formatJson(value ?? null),
    }));
  }, []);

  const normalizeEntries = useCallback(
    (entries: { key: string; value: string }[]) => entries.filter((e) => e.key.trim()),
    [],
  );

  const buildExtraObject = useCallback((): JsonObject | null => {
    const obj: JsonObject = {};
    const seen = new Set<string>();
    const isLikelyJson = (text: string) => {
      const trimmed = text.trim();
      if (!trimmed) return false;
      const first = trimmed[0];
      return (
        first === '{' ||
        first === '[' ||
        first === '"' ||
        /^-?\d/.test(trimmed) ||
        trimmed === 'true' ||
        trimmed === 'false' ||
        trimmed === 'null'
      );
    };

    for (const { key, value } of normalizeEntries(extraEntries)) {
      const normalizedKey = key.trim();
      if (!normalizedKey) continue;
      if (seen.has(normalizedKey)) {
        throw new Error(`config.json 出现重复键：${normalizedKey}`);
      }
      seen.add(normalizedKey);
      const trimmed = value.trim();
      if (!trimmed) {
        obj[normalizedKey] = '';
        continue;
      }
      try {
        obj[normalizedKey] = JSON.parse(trimmed);
      } catch {
        if (isLikelyJson(trimmed)) {
          throw new Error(`config.json 中 ${normalizedKey} 的值 JSON 解析失败，请检查格式`);
        }
        obj[normalizedKey] = value;
      }
    }
    return Object.keys(obj).length ? obj : null;
  }, [extraEntries, normalizeEntries]);

  const validateExtraEntries = useCallback((): JsonObject | null => {
    try {
      const result = buildExtraObject();
      setExtraError(null);
      return result;
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setExtraError(message);
      throw err;
    }
  }, [buildExtraObject]);

  useEffect(() => {
    try {
      validateExtraEntries();
    } catch {
      // ignore，错误信息已写入 extraError
    }
  }, [validateExtraEntries]);

  const loadSettings = useCallback(async () => {
    const payload: ClaudeSettingsPayload = await getClaudeSettings();
    const entries = toEntries(payload.extraConfig);
    setExtraEntries(entries);
    setOriginalExtraEntries(entries);
    setExtraDirty(false);
    setExtraError(null);
    return payload.settings;
  }, [toEntries]);

  const saveConfig = useCallback(
    async (settings: JsonObject) => {
      let parsedExtra: JsonObject | null = null;
      try {
        parsedExtra = validateExtraEntries();
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        toast({ title: '保存失败', description: message, variant: 'destructive' });
        throw err;
      }

      if (parsedExtra) {
        await saveClaudeSettings(settings, parsedExtra);
      } else {
        await saveClaudeSettings(settings);
      }

      const nextEntries = toEntries(parsedExtra);
      setOriginalExtraEntries(nextEntries);
      setExtraEntries(nextEntries);
      setExtraDirty(false);
      setExtraError(null);
    },
    [toEntries, toast, validateExtraEntries],
  );

  const handleResetExtra = useCallback(() => {
    setExtraEntries(originalExtraEntries);
    setExtraDirty(false);
    setExtraError(null);
  }, [originalExtraEntries]);

  const computeExtraDiffs = useCallback((): DiffEntry[] => {
    try {
      const current = buildExtraObject();
      const original = (() => {
        if (!originalExtraEntries.length) return null;
        const obj: JsonObject = {};
        for (const { key, value } of normalizeEntries(originalExtraEntries)) {
          if (!key.trim()) continue;
          try {
            obj[key] = JSON.parse(value);
          } catch {
            obj[key] = value;
          }
        }
        return Object.keys(obj).length ? obj : null;
      })();

      if (JSON.stringify(current) === JSON.stringify(original)) {
        return [];
      }

      let type: DiffEntry['type'] = 'changed';
      if (!original && current) type = 'added';
      if (original && !current) type = 'removed';

      return [
        {
          path: 'config.json',
          type,
          before: original ?? undefined,
          after: current ?? undefined,
        },
      ];
    } catch {
      return [];
    }
  }, [buildExtraObject, normalizeEntries, originalExtraEntries]);

  return (
    <div className="space-y-4">
      <ToolConfigManager
        title="Claude Code 配置管理"
        description="读取并编辑 settings.json"
        loadSchema={getClaudeSchema}
        loadSettings={loadSettings}
        saveSettings={saveConfig}
        refreshSignal={refreshSignal}
        externalDirty={extraDirty}
        onResetExternalChanges={() => {
          handleResetExtra();
        }}
        computeExternalDiffs={computeExtraDiffs}
      />

      <Card className="border">
        <CardHeader>
          <CardTitle>附属配置：config.json</CardTitle>
          <CardDescription>可选文件，存在时将与 settings.json 一同保存。</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="flex items-center justify-between">
            <div>
              <Label className="font-mono text-sm font-semibold">~/.claude/config.json</Label>
              <p className="text-xs text-muted-foreground">
                以键值对形式编辑 config.json，值可写 JSON（自动解析）；留空则不写入。
              </p>
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                setExtraEntries((prev) => [...prev, { key: '', value: '' }]);
                setExtraDirty(true);
              }}
            >
              <Plus className="mr-1 h-4 w-4" />
              新增键值
            </Button>
          </div>

          <div className="space-y-3">
            {extraEntries.length === 0 && (
              <p className="text-xs text-muted-foreground">
                当前为空，保存时不会写入 config.json。
              </p>
            )}
            {extraEntries.map((entry, idx) => (
              <div
                key={`${idx}-${entry.key}`}
                className="flex flex-col gap-2 rounded-md border p-3 md:flex-row md:items-center"
              >
                <div className="flex flex-1 items-center gap-2">
                  <div className="w-40 min-w-[140px]">
                    <Label className="font-mono text-sm font-semibold">key</Label>
                    <Input
                      value={entry.key}
                      onChange={(e) => {
                        const next = [...extraEntries];
                        next[idx] = { ...entry, key: e.target.value };
                        setExtraEntries(next);
                        setExtraDirty(true);
                      }}
                      placeholder="primaryApiKey"
                    />
                  </div>
                  <div className="flex-1">
                    <Label className="font-mono text-sm font-semibold">value</Label>
                    <Input
                      value={entry.value}
                      onChange={(e) => {
                        const next = [...extraEntries];
                        next[idx] = { ...entry, value: e.target.value };
                        setExtraEntries(next);
                        setExtraDirty(true);
                      }}
                      placeholder='如 "sk-..." 或 {"enabled":true}'
                    />
                  </div>
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => {
                    const next = [...extraEntries];
                    next.splice(idx, 1);
                    setExtraEntries(next);
                    setExtraDirty(true);
                  }}
                  aria-label="删除键值"
                >
                  <Trash2 className="h-4 w-4 text-slate-500" />
                </Button>
              </div>
            ))}
          </div>

          {extraError ? (
            <p className="text-xs text-red-600">格式错误：{extraError}</p>
          ) : extraDirty ? (
            <p className="text-xs text-amber-600">config.json 已修改，保存后生效。</p>
          ) : (
            <p className="text-xs text-muted-foreground">同步保存时将一并写入 config.json。</p>
          )}
          <div className="flex gap-2">
            <Button variant="outline" size="sm" onClick={handleResetExtra} disabled={!extraDirty}>
              还原 config.json
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
