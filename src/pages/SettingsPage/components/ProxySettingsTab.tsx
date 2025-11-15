import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Info, Loader2, AlertCircle, Plus, X } from 'lucide-react';

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

  // 添加新的过滤规则
  const addBypassRule = () => {
    const newUrls = [...proxyBypassUrls, ''];
    setProxyBypassUrls(newUrls);
  };

  // 删除过滤规则
  const removeBypassRule = (index: number) => {
    const newUrls = proxyBypassUrls.filter((_, i) => i !== index);
    setProxyBypassUrls(newUrls);
  };

  // 更新过滤规则
  const updateBypassRule = (index: number, value: string) => {
    const newUrls = [...proxyBypassUrls];
    newUrls[index] = value;
    setProxyBypassUrls(newUrls);
  };

  return (
    <div className="space-y-4 rounded-lg border p-6">
      <div className="flex items-center gap-2">
        <Info className="h-5 w-5" />
        <h3 className="text-lg font-semibold">网络代理配置</h3>
      </div>
      <Separator />

      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <Label>启用代理</Label>
            <p className="text-xs text-muted-foreground">通过代理服务器转发所有网络请求</p>
          </div>
          <input
            type="checkbox"
            checked={proxyEnabled}
            onChange={(e) => setProxyEnabled(e.target.checked)}
            className="h-4 w-4 rounded border-slate-300"
          />
        </div>

        {proxyEnabled && (
          <>
            <div className="space-y-2">
              <Label htmlFor="proxy-type">代理类型</Label>
              <Select value={proxyType} onValueChange={(v: any) => setProxyType(v)}>
                <SelectTrigger id="proxy-type">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="http">HTTP</SelectItem>
                  <SelectItem value="https">HTTPS</SelectItem>
                  <SelectItem value="socks5">SOCKS5</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="proxy-host">代理地址 *</Label>
                <Input
                  id="proxy-host"
                  placeholder="127.0.0.1"
                  value={proxyHost}
                  onChange={(e) => setProxyHost(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="proxy-port">端口 *</Label>
                <Input
                  id="proxy-port"
                  placeholder="7890"
                  value={proxyPort}
                  onChange={(e) => setProxyPort(e.target.value)}
                />
              </div>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="proxy-username">用户名（可选）</Label>
                <Input
                  id="proxy-username"
                  placeholder="username"
                  value={proxyUsername}
                  onChange={(e) => setProxyUsername(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="proxy-password">密码（可选）</Label>
                <Input
                  id="proxy-password"
                  type="password"
                  placeholder="password"
                  value={proxyPassword}
                  onChange={(e) => setProxyPassword(e.target.value)}
                />
              </div>
            </div>

            {/* 代理过滤列表 */}
            <div className="pt-4 border-t space-y-3">
              <div className="space-y-2">
                <Label>代理过滤列表</Label>
                <p className="text-xs text-muted-foreground">
                  这些URL/IP将不使用代理，例如本地地址、内网地址等
                </p>
              </div>

              <div className="space-y-2">
                {proxyBypassUrls.map((url, index) => (
                  <div key={index} className="flex gap-2">
                    <Input
                      value={url}
                      onChange={(e) => updateBypassRule(index, e.target.value)}
                      placeholder="例如: 127.0.0.1, localhost, 192.168.*"
                      className="flex-1"
                    />
                    <Button
                      type="button"
                      variant="outline"
                      size="icon"
                      onClick={() => removeBypassRule(index)}
                      className="h-9 w-9"
                    >
                      <X className="h-4 w-4" />
                    </Button>
                  </div>
                ))}

                <Button
                  type="button"
                  variant="outline"
                  onClick={addBypassRule}
                  className="w-full"
                >
                  <Plus className="h-4 w-4 mr-2" />
                  添加过滤规则
                </Button>
              </div>

              <div className="text-xs text-muted-foreground">
                <p>支持格式示例：</p>
                <ul className="list-disc list-inside mt-1 space-y-1">
                  <li>域名: localhost, 127.0.0.1</li>
                  <li>IP段: 192.168.*, 10.*</li>
                  <li>通配符: *.local, *.lan</li>
                </ul>
              </div>
            </div>

            {/* 测试代理连接 */}
            <div className="pt-4 border-t space-y-3">
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <Label htmlFor="proxy-test-url">测试URL</Label>
                  <div className="flex gap-2">
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onClick={() => setProxyTestUrl('https://duckcoding.com/')}
                      className="h-7 text-xs"
                    >
                      DuckCoding
                    </Button>
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onClick={() => setProxyTestUrl('https://www.google.com/')}
                      className="h-7 text-xs"
                    >
                      Google
                    </Button>
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onClick={() => setProxyTestUrl('https://api.anthropic.com/')}
                      className="h-7 text-xs"
                    >
                      Anthropic
                    </Button>
                  </div>
                </div>
                <Input
                  id="proxy-test-url"
                  placeholder="https://duckcoding.com/"
                  value={proxyTestUrl}
                  onChange={(e) => setProxyTestUrl(e.target.value)}
                />
                <p className="text-xs text-muted-foreground">选择或输入一个URL来测试代理连接</p>
              </div>

              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={onTestProxy}
                disabled={testingProxy || !proxyHost.trim() || !proxyPort || !proxyTestUrl.trim()}
                className="w-full"
              >
                {testingProxy ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    测试中...
                  </>
                ) : (
                  <>
                    <AlertCircle className="mr-2 h-4 w-4" />
                    测试代理连接
                  </>
                )}
              </Button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}