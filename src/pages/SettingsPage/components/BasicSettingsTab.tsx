import { useEffect, useState } from 'react';
import { Switch } from '@/components/ui/switch';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { RefreshCw, Power, MonitorPlay } from 'lucide-react';
import { useToast } from '@/hooks/use-toast';
import {
  getSingleInstanceConfig,
  updateSingleInstanceConfig,
  getStartupConfig,
  updateStartupConfig,
} from '@/lib/tauri-commands';

export function BasicSettingsTab() {
  const [singleInstanceEnabled, setSingleInstanceEnabled] = useState(true);
  const [startupEnabled, setStartupEnabled] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const { toast } = useToast();

  // 加载配置
  useEffect(() => {
    const loadConfig = async () => {
      setLoading(true);
      try {
        const [singleInstance, startup] = await Promise.all([
          getSingleInstanceConfig(),
          getStartupConfig(),
        ]);
        setSingleInstanceEnabled(singleInstance);
        setStartupEnabled(startup);
      } catch (error) {
        console.error('加载配置失败:', error);
        toast({
          title: '加载失败',
          description: String(error),
          variant: 'destructive',
        });
      } finally {
        setLoading(false);
      }
    };

    loadConfig();
  }, [toast]);

  // 保存单实例配置
  const handleSingleInstanceToggle = async (checked: boolean) => {
    setSaving(true);
    try {
      await updateSingleInstanceConfig(checked);
      setSingleInstanceEnabled(checked);
      toast({
        title: '设置已保存',
        description: (
          <div className="flex flex-col gap-2">
            <p>请重启应用以使更改生效</p>
            <Button
              size="sm"
              variant="outline"
              onClick={() => window.location.reload()}
              className="w-fit"
            >
              <RefreshCw className="mr-2 h-4 w-4" />
              立即重启
            </Button>
          </div>
        ),
      });
    } catch (error) {
      console.error('保存单实例配置失败:', error);
      toast({
        title: '保存失败',
        description: String(error),
        variant: 'destructive',
      });
    } finally {
      setSaving(false);
    }
  };

  // 保存开机自启动配置
  const handleStartupToggle = async (checked: boolean) => {
    setSaving(true);
    try {
      await updateStartupConfig(checked);
      setStartupEnabled(checked);
      toast({
        title: '设置已保存',
        description: checked ? '已启用开机自启动' : '已禁用开机自启动',
      });
    } catch (error) {
      console.error('保存开机自启动配置失败:', error);
      toast({
        title: '保存失败',
        description: String(error),
        variant: 'destructive',
      });
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="grid gap-6">
      {/* 启动设置 */}
      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <Power className="h-5 w-5 text-primary" />
            <CardTitle>启动设置</CardTitle>
          </div>
          <CardDescription>控制应用启动时的行为</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between p-4 border rounded-lg bg-muted/20">
            <div className="space-y-0.5">
              <Label htmlFor="startup" className="text-base">开机自启动</Label>
              <p className="text-sm text-muted-foreground">
                系统启动时自动运行 DuckCoding，方便快速访问。更改后立即生效。
              </p>
            </div>
            <Switch
              id="startup"
              checked={startupEnabled}
              onCheckedChange={handleStartupToggle}
              disabled={loading || saving}
            />
          </div>
        </CardContent>
      </Card>

      {/* 运行模式 */}
      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <MonitorPlay className="h-5 w-5 text-primary" />
            <CardTitle>运行模式</CardTitle>
          </div>
          <CardDescription>配置应用实例的运行方式（仅供高级用户使用）</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between p-4 border rounded-lg bg-muted/20">
            <div className="space-y-0.5">
              <Label htmlFor="single-instance" className="text-base">单实例模式</Label>
              <p className="text-sm text-muted-foreground">
                启用后，尝试打开第二个实例时会聚焦到现有窗口。禁用可允许同时运行多个实例。
              </p>
            </div>
            <Switch
              id="single-instance"
              checked={singleInstanceEnabled}
              onCheckedChange={handleSingleInstanceToggle}
              disabled={loading || saving}
            />
          </div>
        </CardContent>
      </Card>
    </div>
  );
}