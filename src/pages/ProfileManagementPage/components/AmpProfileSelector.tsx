/**
 * AMP Profile 选择器组件
 * 从 Claude Code、Codex、Gemini CLI 三个工具中选择 profile
 */

import { useState, useEffect, useCallback } from 'react';
import { Save, Loader2, AlertCircle, ExternalLink } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { useToast } from '@/hooks/use-toast';
import { pmGetAmpSelection, pmSaveAmpSelection } from '@/lib/tauri-commands';
import type {
  ProfileDescriptor,
  ProfileRef,
  ProfileToolId,
  AmpProfileSelection,
} from '@/types/profile';
import { logoMap } from '@/utils/constants';

interface AmpProfileSelectorProps {
  allProfiles: ProfileDescriptor[];
  onSwitchTab: (toolId: ProfileToolId) => void;
}

const TOOL_CONFIG: {
  key: keyof Omit<AmpProfileSelection, 'updated_at'>;
  toolId: ProfileToolId;
  label: string;
}[] = [
  { key: 'claude', toolId: 'claude-code', label: 'Claude API' },
  { key: 'codex', toolId: 'codex', label: 'OpenAI API' },
  { key: 'gemini', toolId: 'gemini-cli', label: 'Gemini API' },
];

export function AmpProfileSelector({ allProfiles, onSwitchTab }: AmpProfileSelectorProps) {
  const { toast } = useToast();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [selection, setSelection] = useState<{
    claude: ProfileRef | null;
    codex: ProfileRef | null;
    gemini: ProfileRef | null;
  }>({
    claude: null,
    codex: null,
    gemini: null,
  });

  // 按工具分组 profiles
  const profilesByTool: Record<ProfileToolId, ProfileDescriptor[]> = {
    'claude-code': allProfiles.filter((p) => p.tool_id === 'claude-code'),
    codex: allProfiles.filter((p) => p.tool_id === 'codex'),
    'gemini-cli': allProfiles.filter((p) => p.tool_id === 'gemini-cli'),
  };

  // 加载已保存的选择
  const loadSelection = useCallback(async () => {
    try {
      setLoading(true);
      const saved = await pmGetAmpSelection();
      setSelection({
        claude: saved.claude || null,
        codex: saved.codex || null,
        gemini: saved.gemini || null,
      });
    } catch (error) {
      console.error('Failed to load AMP selection:', error);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadSelection();
  }, [loadSelection]);

  // 保存选择
  const handleSave = async () => {
    try {
      setSaving(true);
      await pmSaveAmpSelection(selection);
      toast({
        title: '保存成功',
        description: 'AMP Profile 选择已更新',
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : '保存失败';
      toast({
        title: '保存失败',
        description: message,
        variant: 'destructive',
      });
    } finally {
      setSaving(false);
    }
  };

  // 更新单个工具的选择
  const handleSelectChange = (
    key: 'claude' | 'codex' | 'gemini',
    toolId: ProfileToolId,
    value: string,
  ) => {
    if (value === '__none__') {
      setSelection((prev) => ({ ...prev, [key]: null }));
    } else {
      setSelection((prev) => ({
        ...prev,
        [key]: { tool_id: toolId, profile_name: value },
      }));
    }
  };

  // 检查引用是否有效
  const isRefValid = (ref: ProfileRef | null, profiles: ProfileDescriptor[]): boolean => {
    if (!ref) return true;
    return profiles.some((p) => p.name === ref.profile_name);
  };

  if (loading) {
    return (
      <Card>
        <CardContent className="flex items-center justify-center py-12">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          <span className="ml-2 text-muted-foreground">加载中...</span>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-3">
          <img src={logoMap['amp-code']} alt="AMP Code" className="h-8 w-8" />
          <div>
            <CardTitle>AMP Code 配置</CardTitle>
            <CardDescription>AMP 使用其他工具的 Profile 配置，请从下方选择</CardDescription>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-6">
        {TOOL_CONFIG.map(({ key, toolId, label }) => {
          const profiles = profilesByTool[toolId];
          const currentRef = selection[key];
          const isValid = isRefValid(currentRef, profiles);

          return (
            <div key={key} className="space-y-2">
              <div className="flex items-center gap-2">
                <img src={logoMap[toolId]} alt={label} className="h-5 w-5" />
                <label className="text-sm font-medium">{label}</label>
              </div>

              {profiles.length === 0 ? (
                <Alert>
                  <AlertCircle className="h-4 w-4" />
                  <AlertDescription className="flex items-center justify-between">
                    <span>还没有 {label} 配置</span>
                    <Button
                      variant="link"
                      size="sm"
                      className="h-auto p-0"
                      onClick={() => onSwitchTab(toolId)}
                    >
                      去创建
                      <ExternalLink className="ml-1 h-3 w-3" />
                    </Button>
                  </AlertDescription>
                </Alert>
              ) : (
                <>
                  <Select
                    value={currentRef?.profile_name || '__none__'}
                    onValueChange={(value) => handleSelectChange(key, toolId, value)}
                  >
                    <SelectTrigger className={!isValid ? 'border-destructive' : ''}>
                      <SelectValue placeholder="选择 Profile..." />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="__none__">未选择</SelectItem>
                      {profiles.map((profile) => (
                        <SelectItem key={profile.name} value={profile.name}>
                          {profile.name}
                          <span className="ml-2 text-xs text-muted-foreground">
                            ({profile.api_key_preview})
                          </span>
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  {!isValid && currentRef && (
                    <p className="text-xs text-destructive">
                      引用的 Profile "{currentRef.profile_name}" 已失效，请重新选择
                    </p>
                  )}
                </>
              )}
            </div>
          );
        })}

        <div className="flex justify-end pt-4">
          <Button onClick={handleSave} disabled={saving}>
            {saving ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <Save className="mr-2 h-4 w-4" />
            )}
            保存选择
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
