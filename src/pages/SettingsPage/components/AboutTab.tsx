import { useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-shell';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Info, RefreshCw, Github, Globe } from 'lucide-react';
import { getCurrentAppVersion } from '@/services/update';
import duckLogo from '@/assets/duck-logo.png';

interface AboutTabProps {
  onCheckUpdate: () => void;
}

export function AboutTab({ onCheckUpdate }: AboutTabProps) {
  const [version, setVersion] = useState<string>('Loading...');

  useEffect(() => {
    getCurrentAppVersion()
      .then(setVersion)
      .catch((err) => {
        console.error('Failed to get version:', err);
        setVersion('Unknown');
      });
  }, []);

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Info className="h-5 w-5" />
            关于 DuckCoding
          </CardTitle>
          <CardDescription>应用信息与版本管理</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col items-center py-8 space-y-6">
          <div className="relative">
            <div className="w-24 h-24 rounded-2xl bg-gradient-to-br from-blue-500 to-cyan-400 flex items-center justify-center shadow-lg">
              <img src={duckLogo} alt="DuckCoding Logo" className="w-20 h-20 object-contain" />
            </div>
            <Badge className="absolute -bottom-2 -right-2 bg-slate-800 text-white">
              v{version}
            </Badge>
          </div>

          <div className="text-center space-y-2">
            <h2 className="text-2xl font-bold tracking-tight">DuckCoding</h2>
            <p className="text-muted-foreground max-w-md">
              一个专为开发者设计的现代化 AI 辅助编程工具，集成多种大模型能力，提供高效的编码体验。
            </p>
          </div>

          <div className="flex flex-wrap justify-center gap-4 pt-4">
            <Button onClick={onCheckUpdate} className="min-w-[140px]">
              <RefreshCw className="mr-2 h-4 w-4" />
              检查更新
            </Button>

            <Button
              variant="outline"
              onClick={() => open('https://github.com/DuckCoding-dev/DuckCoding')}
            >
              <Github className="mr-2 h-4 w-4" />
              GitHub 仓库
            </Button>

            <Button
              variant="outline"
              onClick={() => open('https://github.com/DuckCoding-dev/DuckCoding/issues')}
            >
              <Globe className="mr-2 h-4 w-4" />
              反馈问题
            </Button>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>开源协议</CardTitle>
        </CardHeader>
        <CardContent className="text-sm text-muted-foreground space-y-2">
          <p>DuckCoding 是一个开源项目，遵循 MIT 许可证。</p>
          <p>Copyright © 2025 DuckCoding Contributors.</p>
        </CardContent>
      </Card>
    </div>
  );
}
