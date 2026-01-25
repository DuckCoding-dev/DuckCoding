import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Info, ShieldCheck, Globe, ListFilter, Plus, X, PlayCircle, Loader2 } from 'lucide-react';

interface ProxySettingsTabProps {
  proxyEnabled: boolean;
  setProxyEnabled: (value: boolean) => void;
  proxyType: 'http' | 'https' | 'socks5';
  setProxyType: (value: 'http' | 'https' | 'socks5') => void;
  proxyHost: string;
  setProxyHost: (value: string) => void;
  proxyPort: string;
  setProxyPort: (value: string) => void;
  proxyUsername: string;
  setProxyUsername: (value: string) => void;
  proxyPassword: string;
  setProxyPassword: (value: string) => void;
  proxyTestUrl: string;
  setProxyTestUrl: (value: string) => void;
  testingProxy: boolean;
  onTestProxy: () => void;
  proxyBypassUrls: string[];
  setProxyBypassUrls: (urls: string[]) => void;
}

export function ProxySettingsTab({
  proxyEnabled,
  setProxyEnabled,
  proxyType,
  setProxyType,
  proxyHost,
  setProxyHost,
  proxyPort,
  setProxyPort,
  proxyUsername,
  setProxyUsername,
  proxyPassword,
  setProxyPassword,
  proxyTestUrl,
  setProxyTestUrl,
  testingProxy,
  onTestProxy,
  proxyBypassUrls,
  setProxyBypassUrls,
}: ProxySettingsTabProps) {
  const addBypassRule = () => {
    const newUrls = [...proxyBypassUrls, ''];
    setProxyBypassUrls(newUrls);
  };

  const removeBypassRule = (index: number) => {
    const newUrls = proxyBypassUrls.filter((_, i) => i !== index);
    setProxyBypassUrls(newUrls);
  };

  const updateBypassRule = (index: number, value: string) => {
    const newUrls = [...proxyBypassUrls];
    newUrls[index] = value;
    setProxyBypassUrls(newUrls);
  };

  return (
    <div className="space-y-6">
      {/* 总开关 */}
      <Card>
        <CardHeader className="pb-4">
          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <CardTitle className="text-base flex items-center gap-2">
                <Globe className="h-5 w-5 text-primary" />
                启用网络代理
              </CardTitle>
              <CardDescription>
                通过代理服务器转发所有网络请求，适用于受限网络环境
              </CardDescription>
            </div>
            <Switch
              checked={proxyEnabled}
              onCheckedChange={setProxyEnabled}
            />
          </div>
        </CardHeader>
      </Card>

      {proxyEnabled && (
        <div className="grid gap-6 md:grid-cols-2">
          {/* 服务器配置 */}
          <Card className="md:col-span-2">
            <CardHeader>
              <CardTitle className="text-base flex items-center gap-2">
                <Info className="h-5 w-5 text-primary" />
                服务器配置
              </CardTitle>
            </CardHeader>
            <CardContent className="grid gap-6 md:grid-cols-2">
              <div className="space-y-2 md:col-span-2">
                <Label>代理协议</Label>
                <Select value={proxyType} onValueChange={(v: any) => setProxyType(v)}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="http">HTTP</SelectItem>
                    <SelectItem value="https">HTTPS</SelectItem>
                    <SelectItem value="socks5">SOCKS5</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-2">
                <Label>主机地址 (Host)</Label>
                <Input
                  placeholder="例如: 127.0.0.1"
                  value={proxyHost}
                  onChange={(e) => setProxyHost(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label>端口 (Port)</Label>
                <Input
                  placeholder="例如: 7890"
                  value={proxyPort}
                  onChange={(e) => setProxyPort(e.target.value)}
                />
              </div>
            </CardContent>
          </Card>

          {/* 认证信息 */}
          <Card>
            <CardHeader>
              <CardTitle className="text-base flex items-center gap-2">
                <ShieldCheck className="h-5 w-5 text-primary" />
                身份认证 (可选)
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label>用户名</Label>
                <Input
                  placeholder="Username"
                  value={proxyUsername}
                  onChange={(e) => setProxyUsername(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label>密码</Label>
                <Input
                  type="password"
                  placeholder="Password"
                  value={proxyPassword}
                  onChange={(e) => setProxyPassword(e.target.value)}
                />
              </div>
            </CardContent>
          </Card>

          {/* 连接测试 */}
          <Card>
            <CardHeader>
              <CardTitle className="text-base flex items-center gap-2">
                <PlayCircle className="h-5 w-5 text-primary" />
                连接测试
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label>测试目标 URL</Label>
                <div className="flex gap-2 mb-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => setProxyTestUrl('https://www.google.com')}
                    className="h-6 text-xs px-2"
                  >
                    Google
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => setProxyTestUrl('https://api.openai.com')}
                    className="h-6 text-xs px-2"
                  >
                    OpenAI
                  </Button>
                </div>
                <Input
                  value={proxyTestUrl}
                  onChange={(e) => setProxyTestUrl(e.target.value)}
                  placeholder="https://..."
                />
              </div>
              <Button
                className="w-full"
                onClick={onTestProxy}
                disabled={testingProxy || !proxyHost || !proxyPort}
              >
                {testingProxy ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    测试中...
                  </>
                ) : (
                  '开始测试'
                )}
              </Button>
            </CardContent>
          </Card>

          {/* 绕过列表 */}
          <Card className="md:col-span-2">
            <CardHeader>
              <div className="flex items-center justify-between">
                <CardTitle className="text-base flex items-center gap-2">
                  <ListFilter className="h-5 w-5 text-primary" />
                  绕过列表 (Bypass)
                </CardTitle>
                <Button variant="outline" size="sm" onClick={addBypassRule}>
                  <Plus className="h-4 w-4 mr-2" />
                  添加规则
                </Button>
              </div>
              <CardDescription>
                以下地址将直接连接，不经过代理服务器。支持 IP、域名和通配符。
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              {proxyBypassUrls.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground bg-muted/20 rounded-lg border border-dashed">
                  <p className="text-sm">暂无规则，所有流量都将经过代理</p>
                </div>
              ) : (
                <div className="grid gap-3 md:grid-cols-2">
                  {proxyBypassUrls.map((url, index) => (
                    <div key={index} className="flex gap-2">
                      <Input
                        value={url}
                        onChange={(e) => updateBypassRule(index, e.target.value)}
                        placeholder="例如: localhost"
                      />
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => removeBypassRule(index)}
                        className="text-muted-foreground hover:text-destructive"
                      >
                        <X className="h-4 w-4" />
                      </Button>
                    </div>
                  ))}
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}