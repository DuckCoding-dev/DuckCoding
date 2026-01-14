// 会话设置 Tab 组件
// 包含基本信息、备注编辑、配置管理、删除操作

import { useState, useEffect, useCallback } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Textarea } from '@/components/ui/textarea';
import { Badge } from '@/components/ui/badge';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from '@/components/ui/alert-dialog';
import { useToast } from '@/hooks/use-toast';
import { isActiveSession } from '@/utils/sessionHelpers';
import {
  updateSessionNote,
  updateSessionConfig,
  deleteSession,
  type SessionRecord,
} from '@/lib/tauri-commands';
import { pmListToolProfiles } from '@/lib/tauri-commands/profile';
import type { ToolId } from '../../types/proxy-history';

interface SessionSettingsTabProps {
  session: SessionRecord;
  toolId: ToolId;
  onBack: () => void;
}

/**
 * 信息行组件
 */
function InfoRow({ label, value }: { label: string; value: string | React.ReactNode }) {
  return (
    <div className="flex items-center justify-between py-2 border-b last:border-b-0">
      <span className="text-sm text-muted-foreground">{label}</span>
      <div className="text-sm font-medium">{value}</div>
    </div>
  );
}

/**
 * 会话设置 Tab 组件
 */
export function SessionSettingsTab({ session, toolId, onBack }: SessionSettingsTabProps) {
  const { toast } = useToast();

  // 备注状态
  const [note, setNote] = useState(session.note || '');
  const [savingNote, setSavingNote] = useState(false);

  // 配置状态
  const [configMode, setConfigMode] = useState<'global' | 'custom'>(
    session.config_name === 'global' ? 'global' : 'custom',
  );
  const [profileName, setProfileName] = useState(session.custom_profile_name || '');
  const [savingConfig, setSavingConfig] = useState(false);
  const [profiles, setProfiles] = useState<string[]>([]);

  // 删除状态
  const [deleting, setDeleting] = useState(false);

  // 加载 Profile 列表
  const loadProfiles = useCallback(async () => {
    try {
      const result = await pmListToolProfiles(toolId);
      setProfiles(result);
    } catch (error) {
      console.error('Failed to load profiles:', error);
    }
  }, [toolId]);

  // 初始加载
  useEffect(() => {
    loadProfiles();
  }, [loadProfiles]);

  // 保存备注
  const handleSaveNote = async () => {
    try {
      setSavingNote(true);
      await updateSessionNote(session.session_id, note);
      toast({
        title: '保存成功',
        description: '会话备注已更新',
      });
    } catch (error: any) {
      toast({
        title: '保存失败',
        description: error?.message || String(error),
        variant: 'destructive',
      });
    } finally {
      setSavingNote(false);
    }
  };

  // 保存配置
  const handleSaveConfig = async () => {
    try {
      setSavingConfig(true);

      if (configMode === 'global') {
        await updateSessionConfig(session.session_id, 'global', null, session.url, session.api_key);
      } else {
        if (!profileName) {
          toast({
            title: '请选择 Profile',
            description: '自定义配置模式需要选择一个 Profile',
            variant: 'destructive',
          });
          return;
        }
        await updateSessionConfig(
          session.session_id,
          'custom',
          profileName,
          session.url,
          session.api_key,
        );
      }

      toast({
        title: '保存成功',
        description: '会话配置已更新',
      });
    } catch (error: any) {
      toast({
        title: '保存失败',
        description: error?.message || String(error),
        variant: 'destructive',
      });
    } finally {
      setSavingConfig(false);
    }
  };

  // 删除会话
  const handleDelete = async () => {
    try {
      setDeleting(true);
      await deleteSession(session.session_id);
      toast({
        title: '删除成功',
        description: '会话记录已删除',
      });
      // 返回会话列表
      onBack();
    } catch (error: any) {
      toast({
        title: '删除失败',
        description: error?.message || String(error),
        variant: 'destructive',
      });
      setDeleting(false);
    }
  };

  // 格式化时间
  const formatTime = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString('zh-CN');
  };

  return (
    <div className="space-y-6">
      {/* 基本信息 */}
      <Card>
        <CardHeader>
          <CardTitle>基本信息</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-1">
            <InfoRow label="会话 ID" value={session.session_id} />
            <InfoRow label="显示 ID" value={session.display_id} />
            <InfoRow label="首次活动" value={formatTime(session.first_seen_at)} />
            <InfoRow label="最近活动" value={formatTime(session.last_seen_at)} />
            <InfoRow
              label="状态"
              value={
                isActiveSession(session.last_seen_at) ? (
                  <Badge variant="default" className="bg-green-500">
                    活跃
                  </Badge>
                ) : (
                  <Badge variant="secondary">空闲</Badge>
                )
              }
            />
            <InfoRow label="请求次数" value={`${session.request_count} 次`} />
          </div>
        </CardContent>
      </Card>

      {/* 备注编辑 */}
      <Card>
        <CardHeader>
          <CardTitle>备注编辑</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <Textarea
            value={note}
            onChange={(e) => setNote(e.target.value)}
            placeholder="例如：开发环境测试会话，用于验证新功能"
            rows={3}
          />
          <Button onClick={handleSaveNote} disabled={savingNote}>
            {savingNote ? '保存中...' : '保存备注'}
          </Button>
        </CardContent>
      </Card>

      {/* 配置管理 */}
      <Card>
        <CardHeader>
          <CardTitle>配置管理</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <RadioGroup value={configMode} onValueChange={(v) => setConfigMode(v as any)}>
            <div className="flex items-center space-x-2">
              <RadioGroupItem value="global" id="global" />
              <Label htmlFor="global">跟随主配置（使用透明代理的全局配置）</Label>
            </div>
            <div className="flex items-center space-x-2">
              <RadioGroupItem value="custom" id="custom" />
              <Label htmlFor="custom">自定义配置</Label>
            </div>
          </RadioGroup>

          {configMode === 'custom' && (
            <div className="ml-6 space-y-2">
              <Label>选择 Profile</Label>
              <Select value={profileName} onValueChange={setProfileName}>
                <SelectTrigger>
                  <SelectValue placeholder="选择一个 Profile" />
                </SelectTrigger>
                <SelectContent>
                  {profiles.length === 0 ? (
                    <SelectItem value="none" disabled>
                      暂无 Profile
                    </SelectItem>
                  ) : (
                    profiles.map((name) => (
                      <SelectItem key={name} value={name}>
                        {name}
                      </SelectItem>
                    ))
                  )}
                </SelectContent>
              </Select>
            </div>
          )}

          <Button onClick={handleSaveConfig} disabled={savingConfig}>
            {savingConfig ? '保存中...' : '保存配置'}
          </Button>
        </CardContent>
      </Card>

      {/* 危险操作 */}
      <Card className="border-destructive">
        <CardHeader>
          <CardTitle className="text-destructive">危险操作</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <AlertDialog>
            <AlertDialogTrigger asChild>
              <Button variant="destructive" disabled={deleting}>
                {deleting ? '删除中...' : '删除会话'}
              </Button>
            </AlertDialogTrigger>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>确认删除会话？</AlertDialogTitle>
                <AlertDialogDescription>
                  此操作将删除会话「{session.note || session.display_id}
                  」的所有数据，包括统计信息和配置记录。此操作不可恢复！
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>取消</AlertDialogCancel>
                <AlertDialogAction onClick={handleDelete} className="bg-destructive">
                  确认删除
                </AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
          <p className="text-sm text-muted-foreground">删除该会话的所有数据（不可恢复）</p>
        </CardContent>
      </Card>
    </div>
  );
}
